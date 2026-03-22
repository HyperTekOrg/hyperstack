use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn escape_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "\\\\")
}

fn run_compile_failure(name: &str, source: &str, expected: &[&str]) {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .expect("hyperstack-macros should live in workspace root");
    let temp_root = workspace_root.join("target/tests/phase2-dynamic");
    fs::create_dir_all(&temp_root).expect("create dynamic test root");

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    let crate_dir = temp_root.join(format!("{name}-{unique}"));
    let src_dir = crate_dir.join("src");
    fs::create_dir_all(&src_dir).expect("create temp crate src dir");

    let cargo_toml = format!(
        r#"[package]
name = "{name}"
version = "0.0.0"
edition = "2021"

[workspace]

[dependencies]
hyperstack-macros = {{ path = "{}" }}
"#,
        escape_path(&manifest_dir)
    );

    fs::write(crate_dir.join("Cargo.toml"), cargo_toml).expect("write temp Cargo.toml");
    fs::write(src_dir.join("main.rs"), source).expect("write temp main.rs");

    let output = Command::new("cargo")
        .arg("check")
        .arg("--quiet")
        .current_dir(&crate_dir)
        .env("CARGO_TARGET_DIR", workspace_root.join("target"))
        .output()
        .expect("run cargo check");

    assert!(
        !output.status.success(),
        "expected cargo check to fail for {name}"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    for needle in expected {
        assert!(
            stderr.contains(needle),
            "expected stderr for {name} to contain {needle:?}, got:\n{stderr}"
        );
    }
}

fn pump_idl_path() -> String {
    escape_path(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root")
            .join("hyperstack-idl/tests/fixtures/pump.json"),
    )
}

#[test]
fn invalid_map_strategy_is_rejected_early() {
    let source = r#"use hyperstack_macros::hyperstack;

#[hyperstack]
mod broken {
    #[entity(name = "Thing")]
    struct Thing {
        #[map(fake_sdk::accounts::Thing::value, strategy = Invalid)]
        value: u64,
    }
}

fn main() {}
"#;

    run_compile_failure(
        "invalid_map_strategy_is_rejected_early",
        source,
        &["invalid strategy 'Invalid' for #[map]"],
    );
}

#[test]
fn invalid_condition_is_rejected_early() {
    let source = r#"use hyperstack_macros::hyperstack;

#[hyperstack]
mod broken {
    #[entity(name = "Thing")]
    struct Thing {
        #[aggregate(from = fake_sdk::instructions::Trade, field = amount, condition = "amount >> 1")]
        total: u64,
    }
}

fn main() {}
"#;

    run_compile_failure(
        "invalid_condition_is_rejected_early",
        source,
        &["Invalid condition expression 'amount >> 1'"],
    );
}

#[test]
fn missing_instruction_gets_suggestion() {
    let source = format!(
        r#"use hyperstack_macros::hyperstack;

#[hyperstack(idl = "{}")]
mod broken {{
    #[entity(name = "Thing")]
    struct Thing {{
        #[event(from = pump_sdk::instructions::Initialise, fields = [user])]
        trades: Vec<pump_sdk::instructions::Buy>,
    }}
}}

fn main() {{}}
"#,
        pump_idl_path()
    );

    run_compile_failure(
        "missing_instruction_gets_suggestion",
        &source,
        &[
            "Not found: 'Initialise' in instructions",
            "Did you mean: initialize?",
        ],
    );
}

#[test]
fn missing_instruction_field_gets_suggestion() {
    let source = format!(
        r#"use hyperstack_macros::hyperstack;

#[hyperstack(idl = "{}")]
mod broken {{
    #[entity(name = "Thing")]
    struct Thing {{
        #[event(from = pump_sdk::instructions::Buy, fields = [usr])]
        trades: Vec<pump_sdk::instructions::Buy>,
    }}
}}

fn main() {{}}
"#,
        pump_idl_path()
    );

    run_compile_failure(
        "missing_instruction_field_gets_suggestion",
        &source,
        &[
            "Not found: 'usr' in instruction fields for 'buy'",
            "Did you mean: user?",
        ],
    );
}

#[test]
fn missing_account_type_gets_suggestion() {
    let source = format!(
        r#"use hyperstack_macros::hyperstack;

#[hyperstack(idl = "{}")]
mod broken {{
    #[entity(name = "Thing")]
    struct Thing {{
        #[map(pump_sdk::accounts::BondingCurv::complete, strategy = LastWrite)]
        complete: bool,
    }}
}}

fn main() {{}}
"#,
        pump_idl_path()
    );

    run_compile_failure(
        "missing_account_type_gets_suggestion",
        &source,
        &[
            "Not found: 'BondingCurv' in accounts",
            "Did you mean: BondingCurve?",
        ],
    );
}
