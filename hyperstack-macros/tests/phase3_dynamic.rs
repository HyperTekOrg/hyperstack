use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn escape_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "\\\\")
}

fn compile_failure_stderr(name: &str, source: &str) -> String {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .expect("hyperstack-macros should live in workspace root");
    let temp_root = workspace_root.join("target/tests/phase3-dynamic");
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

    String::from_utf8_lossy(&output.stderr).into_owned()
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
fn missing_instruction_error_points_to_from_path() {
    let source = format!(
        r#"use hyperstack_macros::hyperstack;

#[hyperstack(idl = "{}")]
mod broken {{
    #[entity(name = "Thing")]
    struct Thing {{
        #[event(
            from = pump_sdk::instructions::Initialise,
            fields = [user]
        )]
        trades: Vec<pump_sdk::instructions::Buy>,
    }}
}}

fn main() {{}}
"#,
        pump_idl_path()
    );

    let stderr = compile_failure_stderr("missing_instruction_error_points_to_from_path", &source);
    assert!(stderr.contains("Not found: 'Initialise' in instructions"));
    assert!(stderr.contains("src/main.rs:8:"), "stderr was:\n{stderr}");
}

#[test]
fn missing_instruction_field_error_points_to_field_token() {
    let source = format!(
        r#"use hyperstack_macros::hyperstack;

#[hyperstack(idl = "{}")]
mod broken {{
    #[entity(name = "Thing")]
    struct Thing {{
        #[event(
            from = pump_sdk::instructions::Buy,
            fields = [usr]
        )]
        trades: Vec<pump_sdk::instructions::Buy>,
    }}
}}

fn main() {{}}
"#,
        pump_idl_path()
    );

    let stderr = compile_failure_stderr(
        "missing_instruction_field_error_points_to_field_token",
        &source,
    );
    assert!(stderr.contains("Not found: 'usr' in instruction fields for 'buy'"));
    assert!(stderr.contains("src/main.rs:9:"), "stderr was:\n{stderr}");
}

#[test]
fn missing_account_error_points_to_map_path() {
    let source = format!(
        r#"use hyperstack_macros::hyperstack;

#[hyperstack(idl = "{}")]
mod broken {{
    #[entity(name = "Thing")]
    struct Thing {{
        #[map(
            pump_sdk::accounts::BondingCurv::complete,
            strategy = LastWrite
        )]
        complete: bool,
    }}
}}

fn main() {{}}
"#,
        pump_idl_path()
    );

    let stderr = compile_failure_stderr("missing_account_error_points_to_map_path", &source);
    assert!(stderr.contains("Not found: 'BondingCurv' in accounts"));
    assert!(stderr.contains("src/main.rs:8:"), "stderr was:\n{stderr}");
}
