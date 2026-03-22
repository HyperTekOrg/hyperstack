use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn escape_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "\\\\")
}

fn compile_failure_stderr_with_files(
    name: &str,
    source: &str,
    extra_files: &[(&str, &str)],
) -> String {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .expect("hyperstack-macros should live in workspace root");
    let temp_root = workspace_root.join("target/tests/key-resolution-dynamic");
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

    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn compile_success_with_files(name: &str, source: &str, extra_files: &[(&str, &str)]) {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .expect("hyperstack-macros should live in workspace root");
    let hyperstack_dir = workspace_root.join("hyperstack");
    let temp_root = workspace_root.join("target/tests/key-resolution-dynamic");
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
hyperstack = {{ path = "{}" }}
hyperstack-macros = {{ path = "{}" }}
borsh = {{ version = "1.5", features = ["derive"] }}
serde = {{ version = "1", features = ["derive"] }}
"#,
        escape_path(&hyperstack_dir),
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
        output.status.success(),
        "expected cargo check to succeed for {name}, stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn minimal_idl() -> &'static str {
    r#"{
  "name": "fake",
  "instructions": [
    {
      "name": "Trade",
      "accounts": [{ "name": "thing" }],
      "args": [
        { "name": "user", "type": "string" },
        { "name": "id", "type": "string" }
      ]
    }
  ],
  "accounts": [
    {
      "name": "Thing",
      "type": {
        "kind": "struct",
        "fields": [{ "name": "id", "type": "string" }]
      }
    },
    {
      "name": "Position",
      "type": {
        "kind": "struct",
        "fields": [{ "name": "amount", "type": "u64" }]
      }
    }
  ],
  "types": [],
  "events": [],
  "errors": [],
  "constants": []
}"#
}

#[test]
fn instruction_source_without_lookup_path_is_rejected() {
    let source = r#"use hyperstack_macros::hyperstack;

#[hyperstack(idl = "fixture/minimal.json")]
mod broken {
    #[entity(name = "Thing")]
    struct Thing {
        #[map(fake_sdk::accounts::Thing::id, primary_key, strategy = SetOnce)]
        id: String,

        #[aggregate(from = fake_sdk::instructions::Trade, strategy = Count)]
        trades: u64,
    }
}

fn main() {}
"#;

    let stderr = compile_failure_stderr_with_files(
        "instruction_source_without_lookup_path_is_rejected",
        source,
        &[("fixture/minimal.json", minimal_idl())],
    );
    assert!(stderr.contains(
        "instruction source 'fake_sdk::instructions::Trade' cannot resolve the primary key"
    ));
    assert!(stderr.contains("Add a `primary_key` mapping or `lookup_by = ...`"));
}

#[test]
fn account_source_without_pk_lookup_or_resolver_is_rejected() {
    let source = r#"use hyperstack_macros::hyperstack;

#[hyperstack(idl = "fixture/minimal.json")]
mod broken {
    #[entity(name = "Thing")]
    struct Thing {
        #[map(fake_sdk::accounts::Thing::id, primary_key, strategy = SetOnce)]
        id: String,

        #[map(fake_sdk::accounts::Position::amount, strategy = LastWrite)]
        amount: u64,
    }
}

fn main() {}
"#;

    let stderr = compile_failure_stderr_with_files(
        "account_source_without_pk_lookup_or_resolver_is_rejected",
        source,
        &[("fixture/minimal.json", minimal_idl())],
    );
    assert!(stderr
        .contains("account source 'fake_sdk::accounts::Position' cannot resolve the primary key"));
    assert!(stderr.contains("lookup-index-backed field"));
}

#[test]
fn event_source_without_lookup_or_join_is_rejected() {
    let source = r#"use hyperstack_macros::hyperstack;

#[hyperstack(idl = "fixture/minimal.json")]
mod broken {
    #[entity(name = "Thing")]
    struct Thing {
        #[map(fake_sdk::accounts::Thing::id, primary_key, strategy = SetOnce)]
        id: String,

        #[event(from = fake_sdk::instructions::Trade, fields = [user])]
        trades: Vec<fake_sdk::instructions::Trade>,
    }
}

fn main() {}
"#;

    let stderr = compile_failure_stderr_with_files(
        "event_source_without_lookup_or_join_is_rejected",
        source,
        &[("fixture/minimal.json", minimal_idl())],
    );
    assert!(stderr.contains("event source '"));
    assert!(stderr.contains("cannot resolve the primary key"));
    assert!(stderr.contains("Add `lookup_by = ...` or `join_on = ...`"));
}

#[test]
fn derive_from_primary_key_field_does_not_require_lookup_by() {
    let source = r#"use hyperstack_macros::hyperstack;

#[hyperstack(idl = "fixture/minimal.json")]
mod valid {
    #[entity(name = "Thing")]
    struct Thing {
        #[map(fake_sdk::accounts::Thing::id, primary_key, strategy = SetOnce)]
        id: String,

        #[derive_from(from = fake_sdk::instructions::Trade, field = id, strategy = LastWrite)]
        latest_id: String,
    }
}

fn main() {}
"#;

    compile_success_with_files(
        "derive_from_primary_key_field_does_not_require_lookup_by",
        source,
        &[("fixture/minimal.json", minimal_idl())],
    );
}
