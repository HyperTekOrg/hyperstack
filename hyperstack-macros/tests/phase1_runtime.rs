use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn escape_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "\\\\")
}

fn run_binary_success(name: &str, source: &str) {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .expect("hyperstack-macros should live in workspace root");
    let hyperstack_dir = workspace_root.join("hyperstack");
    let temp_root = workspace_root.join("target/tests/phase1-runtime");
    fs::create_dir_all(&temp_root).expect("create phase1 runtime root");

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
hyperstack = {{ path = "{}" }}
hyperstack-macros = {{ path = "{}" }}
serde = {{ version = "1", features = ["derive"] }}
"#,
        escape_path(&hyperstack_dir),
        escape_path(&manifest_dir)
    );

    fs::write(crate_dir.join("Cargo.toml"), cargo_toml).expect("write temp Cargo.toml");
    fs::write(src_dir.join("main.rs"), source).expect("write temp main.rs");

    let output = Command::new("cargo")
        .arg("run")
        .arg("--quiet")
        .current_dir(&crate_dir)
        .env("CARGO_TARGET_DIR", workspace_root.join("target"))
        .output()
        .expect("run cargo run");

    assert!(
        output.status.success(),
        "expected cargo run to succeed for {name}, stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn generated_loaders_do_not_require_stack_artifact_file() {
    let source = r#"use hyperstack_macros::hyperstack;

#[hyperstack]
mod stream {
    #[entity(name = "Thing")]
    struct Thing {}
}

fn main() {
    let stack_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(".hyperstack/Stream.stack.json");

    if stack_path.exists() {
        std::fs::remove_file(&stack_path).unwrap();
    }

    let _spec = stream::create_thing_spec();
    let _views = stream::get_view_definitions();
    let _bytecode = stream::create_multi_entity_bytecode();
}
"#;

    run_binary_success(
        "generated_loaders_do_not_require_stack_artifact_file",
        source,
    );
}
