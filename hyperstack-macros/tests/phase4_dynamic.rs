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
    let temp_root = workspace_root.join("target/tests/phase4-dynamic");
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
fn unknown_account_field_is_rejected() {
    let source = format!(
        r#"use hyperstack_macros::hyperstack;

#[hyperstack(idl = "{}")]
mod broken {{
    #[entity(name = "Thing")]
    struct Thing {{
        #[map(pump_sdk::accounts::BondingCurve::bogus, strategy = LastWrite)]
        value: u64,
    }}
}}

fn main() {{}}
"#,
        pump_idl_path()
    );

    let stderr = compile_failure_stderr("unknown_account_field_is_rejected", &source);
    assert!(stderr.contains("Not found: 'bogus' in account fields for 'BondingCurve'"));
}

#[test]
fn account_field_validation_is_case_sensitive() {
    let source = format!(
        r#"use hyperstack_macros::hyperstack;

#[hyperstack(idl = "{}")]
mod broken {{
    #[entity(name = "Thing")]
    struct Thing {{
        #[map(pump_sdk::accounts::BondingCurve::Complete, strategy = LastWrite)]
        value: bool,
    }}
}}

fn main() {{}}
"#,
        pump_idl_path()
    );

    let stderr = compile_failure_stderr("account_field_validation_is_case_sensitive", &source);
    assert!(stderr.contains("Not found: 'Complete' in account fields for 'BondingCurve'"));
}

#[test]
fn missing_computed_section_reference_is_rejected() {
    let source = r#"use hyperstack_macros::hyperstack;

#[hyperstack]
mod broken {
    #[entity(name = "Thing")]
    struct Thing {
        base: u64,
        #[computed(ghost.value + 1)]
        total: u64,
    }
}

fn main() {}
"#;

    let stderr = compile_failure_stderr("missing_computed_section_reference_is_rejected", source);
    assert!(stderr.contains("unknown computed field reference 'ghost.value' on entity 'Thing'"));
}

#[test]
fn invalid_resolver_input_field_is_rejected() {
    let source = r#"use hyperstack_macros::hyperstack;

#[hyperstack]
mod broken {
    #[entity(name = "Thing")]
    struct Thing {
        existing: String,
        #[resolve(from = "ghost.value", resolver = Token)]
        metadata: String,
    }
}

fn main() {}
"#;

    let stderr = compile_failure_stderr("invalid_resolver_input_field_is_rejected", source);
    assert!(stderr.contains("unknown resolver input field 'ghost.value' on entity 'Thing'"));
}

#[test]
fn invalid_resolver_condition_field_is_rejected() {
    let source = r#"use hyperstack_macros::hyperstack;

#[hyperstack]
mod broken {
    #[entity(name = "Thing")]
    struct Thing {
        existing: String,
        #[resolve(from = "existing", resolver = Token, condition = "ghost.value == pending")]
        metadata: String,
    }
}

fn main() {}
"#;

    let stderr = compile_failure_stderr("invalid_resolver_condition_field_is_rejected", source);
    assert!(stderr.contains("unknown resolver condition field 'ghost.value' on entity 'Thing'"));
}

#[test]
fn invalid_view_sort_by_is_rejected() {
    let source = r#"use hyperstack_macros::hyperstack;

#[hyperstack]
mod broken {
    #[entity(name = "Thing")]
    #[view(name = "latest", sort_by = "ghost.value")]
    struct Thing {
        base: u64,
    }
}

fn main() {}
"#;

    let stderr = compile_failure_stderr("invalid_view_sort_by_is_rejected", source);
    assert!(stderr.contains("unknown view field 'ghost.value' on entity 'Thing'"));
}

#[test]
fn computed_cycle_is_rejected() {
    let source = r#"use hyperstack_macros::hyperstack;

#[hyperstack]
mod broken {
    #[entity(name = "Thing")]
    struct Thing {
        #[computed(b)]
        a: u64,
        #[computed(a)]
        b: u64,
    }
}

fn main() {}
"#;

    let stderr = compile_failure_stderr("computed_cycle_is_rejected", source);
    assert!(stderr.contains("computed fields contain a dependency cycle"));
}

#[test]
fn validation_reports_multiple_errors() {
    let source = r#"use hyperstack_macros::hyperstack;

#[hyperstack]
mod broken {
    #[entity(name = "Thing")]
    #[view(name = "latest", sort_by = "ghost.value")]
    struct Thing {
        existing: String,
        #[resolve(from = "missing.field", resolver = Token)]
        metadata: String,
    }
}

fn main() {}
"#;

    let stderr = compile_failure_stderr("validation_reports_multiple_errors", source);
    assert!(stderr.contains("unknown view field 'ghost.value' on entity 'Thing'"));
    assert!(stderr.contains("unknown resolver input field 'missing.field' on entity 'Thing'"));
}
