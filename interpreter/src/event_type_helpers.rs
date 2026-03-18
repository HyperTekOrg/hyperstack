#![allow(dead_code)]

/// Extract the base name from a potentially scoped event type.
/// "ore::RoundState" -> "RoundState"
/// "RoundState" -> "RoundState"  (backwards compat)
pub fn event_type_base_name(event_type: &str) -> &str {
    event_type.rsplit("::").next().unwrap_or(event_type)
}

/// Extract the program name from a scoped event type.
/// "ore::RoundState" -> Some("ore")
/// "RoundState" -> None
pub fn event_type_program(event_type: &str) -> Option<&str> {
    event_type.rsplit_once("::").map(|(prefix, _)| prefix)
}

/// Strip the "State" or "IxState" suffix from a potentially scoped event type,
/// returning just the base account/instruction name.
/// "ore::RoundState" -> "Round"
/// "ore::DeployIxState" -> "Deploy"
/// "RoundState" -> "Round"
pub fn strip_event_type_suffix(event_type: &str) -> &str {
    let base = event_type_base_name(event_type);
    base.strip_suffix("IxState")
        .or_else(|| base.strip_suffix("State"))
        .unwrap_or(base)
}

/// Create a scoped event type name.
/// ("ore", "Round", false) -> "ore::RoundState"
/// ("ore", "Deploy", true) -> "ore::DeployIxState"
pub fn scoped_event_type(program_name: &str, type_name: &str, is_instruction: bool) -> String {
    let suffix = if is_instruction { "IxState" } else { "State" };
    format!("{}::{}{}", program_name, type_name, suffix)
}
