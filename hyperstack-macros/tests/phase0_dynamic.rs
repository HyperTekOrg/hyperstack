use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn run_compile_failure(name: &str, source: &str, extra_files: &[(&str, &str)], expected: &[&str]) {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .expect("hyperstack-macros should live in workspace root");
    let temp_root = workspace_root.join("target/tests/phase0-dynamic");
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
    for (relative_path, contents) in extra_files {
        let file_path = crate_dir.join(relative_path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).expect("create extra file parent dir");
        }
        fs::write(file_path, contents).expect("write extra test file");
    }

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

fn run_idl_compile_failure(name: &str, module_body: &str, expected: &[&str]) {
    let source = r#"use hyperstack_macros::hyperstack;

#[hyperstack(idl = "fixture/minimal.json")]
mod broken {
__MODULE_BODY__
}

fn main() {}
"#
    .replace("__MODULE_BODY__", module_body);

    let minimal_idl = r#"{
  "name": "ore",
  "instructions": [],
  "accounts": [],
  "types": [],
  "events": [],
  "errors": [],
  "constants": []
}
"#;

    run_compile_failure(
        name,
        &source,
        &[("fixture/minimal.json", minimal_idl)],
        expected,
    );
}

fn escape_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "\\\\")
}

#[test]
fn after_instruction_attribute_is_hard_error() {
    run_idl_compile_failure(
        "after_instruction_attribute_is_hard_error",
        "    #[after_instruction(ore_sdk::instructions::Initialize)]\n    fn bad() {}",
        &[
            "Direct use of #[after_instruction] is not allowed.",
            "Use declarative macros instead:",
        ],
    );
}

#[test]
fn resolve_key_attribute_is_hard_error() {
    run_idl_compile_failure(
        "resolve_key_attribute_is_hard_error",
        "    #[resolve_key(strategy = \"direct_field\")]\n    struct Marker;",
        &["#[resolve_key] requires 'account' parameter"],
    );
}

#[test]
fn register_pda_attribute_is_hard_error() {
    run_idl_compile_failure(
        "register_pda_attribute_is_hard_error",
        "    #[register_pda(instruction = ore_sdk::instructions::Initialize)]\n    struct Marker;",
        &["#[register_pda] requires 'pda_field' parameter"],
    );
}

#[test]
fn manual_pdas_block_is_hard_error() {
    run_idl_compile_failure(
        "manual_pdas_block_is_hard_error",
        "    #[entity(name = \"Thing\")]\n    struct Marker {}\n\n    pdas! {\n        missing_program {\n            broken = [literal(\"broken\")];\n        }\n    }",
        &["unknown program 'missing_program' in pdas! block"],
    );
}
