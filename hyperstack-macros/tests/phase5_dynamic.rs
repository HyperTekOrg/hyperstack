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
    let temp_root = workspace_root.join("target/tests/phase5-dynamic");
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
fn invalid_strategy_suggests_valid_value() {
    let source = r#"use hyperstack_macros::hyperstack;

#[hyperstack]
mod broken {
    #[entity(name = "Thing")]
    struct Thing {
        #[map(fake_sdk::accounts::Thing::value, strategy = LastWrit)]
        value: u64,
    }
}

fn main() {}
"#;

    let stderr = compile_failure_stderr("invalid_strategy_suggests_valid_value", source);
    assert!(stderr.contains("invalid strategy 'LastWrit' for #[map]"));
    assert!(stderr.contains("Expected one of: SetOnce, LastWrite"));
    assert!(stderr.contains("Did you mean: LastWrite?"));
}

#[test]
fn unknown_resolver_suggests_valid_name() {
    let source = r#"use hyperstack_macros::hyperstack;

#[hyperstack]
mod broken {
    #[entity(name = "Thing")]
    struct Thing {
        existing: String,
        #[resolve(from = "existing", resolver = Toke)]
        metadata: String,
    }
}

fn main() {}
"#;

    let stderr = compile_failure_stderr("unknown_resolver_suggests_valid_name", source);
    assert!(stderr.contains("unknown resolver 'Toke'"));
    assert!(stderr.contains("Did you mean: Token?"));
}

#[test]
fn unknown_pda_program_suggests_available_name() {
    let source = format!(
        r#"use hyperstack_macros::hyperstack;

#[hyperstack(idl = "{}")]
mod broken {{
    #[entity(name = "Thing")]
    struct Thing {{}}

    pdas! {{
        pum {{
            broken = [literal("broken")];
        }}
    }}
}}

fn main() {{}}
"#,
        pump_idl_path()
    );

    let stderr = compile_failure_stderr("unknown_pda_program_suggests_available_name", &source);
    assert!(stderr.contains("unknown program 'pum' in pdas! block"));
    assert!(stderr.contains("Did you mean: pump?"));
}

#[test]
fn empty_url_template_field_is_rejected() {
    let source = r#"use hyperstack_macros::hyperstack;

#[hyperstack]
mod broken {
    #[entity(name = "Thing")]
    struct Thing {
        #[resolve(url = "https://example.com/{   }/metadata", extract = "name")]
        metadata: String,
    }
}

fn main() {}
"#;

    let stderr = compile_failure_stderr("empty_url_template_field_is_rejected", source);
    assert!(stderr.contains("Empty field reference '{}' in URL template"));
}
