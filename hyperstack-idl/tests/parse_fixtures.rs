use hyperstack_idl::parse::parse_idl_file;
use std::path::PathBuf;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn test_parse_ore_legacy() {
    let idl = parse_idl_file(&fixture_path("ore.json")).expect("should parse ore.json");
    assert_eq!(
        idl.instructions.len(),
        19,
        "ore should have 19 instructions"
    );
    assert!(idl.name.is_some(), "ore should have a name");
}

#[test]
fn test_parse_entropy_legacy() {
    let idl = parse_idl_file(&fixture_path("entropy.json")).expect("should parse entropy.json");
    assert_eq!(
        idl.instructions.len(),
        5,
        "entropy should have 5 instructions"
    );
}

#[test]
fn test_parse_pump_modern() {
    let idl = parse_idl_file(&fixture_path("pump.json")).expect("should parse pump.json");
    assert_eq!(idl.instructions.len(), 6, "pump should have 6 instructions");
}

#[test]
fn test_parse_meteora_dlmm_modern() {
    let idl =
        parse_idl_file(&fixture_path("meteora_dlmm.json")).expect("should parse meteora_dlmm.json");
    assert_eq!(
        idl.instructions.len(),
        74,
        "meteora_dlmm should have 74 instructions"
    );
    assert_eq!(
        idl.constants.len(),
        30,
        "meteora_dlmm should have 30 constants"
    );
}
