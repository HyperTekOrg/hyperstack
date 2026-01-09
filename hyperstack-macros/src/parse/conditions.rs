use crate::ast::{ComparisonOp, FieldPath, LogicalOp, ParsedCondition};

/// Parse a condition expression string into a ParsedCondition AST
///
/// Supported syntax:
/// - Comparisons: "field > 100", "amount >= 1000000"
/// - Logical ops: "amount > 100 && user != \"excluded\"", "a < 10 || a > 1000"
/// - Field refs: "amount", "data.field", "accounts.user"
///
/// Returns None if parsing fails (will emit compile error)
pub fn parse_condition_expression(expr: &str) -> Option<ParsedCondition> {
    let expr = expr.trim();

    // Try to parse as logical expression first (contains && or ||)
    if let Some(parsed) = try_parse_logical(expr) {
        return Some(parsed);
    }

    // Parse as comparison
    try_parse_comparison(expr)
}

fn try_parse_logical(expr: &str) -> Option<ParsedCondition> {
    // Split on && or || (respecting precedence: && before ||)
    // For simplicity, we can use a basic tokenizer

    // Find top-level || first (lowest precedence)
    if let Some(pos) = find_top_level_operator(expr, "||") {
        let left = expr[..pos].trim();
        let right = expr[pos + 2..].trim();

        return Some(ParsedCondition::Logical {
            op: LogicalOp::Or,
            conditions: vec![
                parse_condition_expression(left)?,
                parse_condition_expression(right)?,
            ],
        });
    }

    // Find top-level && (higher precedence)
    if let Some(pos) = find_top_level_operator(expr, "&&") {
        let left = expr[..pos].trim();
        let right = expr[pos + 2..].trim();

        return Some(ParsedCondition::Logical {
            op: LogicalOp::And,
            conditions: vec![
                parse_condition_expression(left)?,
                parse_condition_expression(right)?,
            ],
        });
    }

    None
}

fn try_parse_comparison(expr: &str) -> Option<ParsedCondition> {
    // Try each comparison operator in order (longer ones first to avoid conflicts)
    let operators = [
        (">=", ComparisonOp::GreaterThanOrEqual),
        ("<=", ComparisonOp::LessThanOrEqual),
        ("==", ComparisonOp::Equal),
        ("!=", ComparisonOp::NotEqual),
        (">", ComparisonOp::GreaterThan),
        ("<", ComparisonOp::LessThan),
    ];

    for (op_str, op) in &operators {
        if let Some(pos) = find_top_level_operator(expr, op_str) {
            let field = expr[..pos].trim();
            let value = expr[pos + op_str.len()..].trim();

            // Parse field path
            let field_segments: Vec<&str> = field.split('.').collect();
            let field_path = FieldPath::new(&field_segments);

            // Parse value (number, string, or bool)
            let value_json = parse_value(value)?;

            return Some(ParsedCondition::Comparison {
                field: field_path,
                op: op.clone(),
                value: value_json,
            });
        }
    }

    None
}

fn find_top_level_operator(expr: &str, op: &str) -> Option<usize> {
    // Find operator position, ignoring those inside quotes or parentheses
    let mut depth = 0;
    let mut in_quotes = false;
    let mut quote_char = '\0';

    let chars: Vec<char> = expr.chars().collect();
    for i in 0..chars.len() {
        let c = chars[i];

        if !in_quotes {
            if c == '"' || c == '\'' {
                in_quotes = true;
                quote_char = c;
            } else if c == '(' {
                depth += 1;
            } else if c == ')' {
                depth -= 1;
            } else if depth == 0 && expr[i..].starts_with(op) {
                return Some(i);
            }
        } else if c == quote_char && (i == 0 || chars[i - 1] != '\\') {
            in_quotes = false;
        }
    }

    None
}

fn parse_value(value: &str) -> Option<serde_json::Value> {
    use serde_json::Value;

    // Remove underscores from numeric literals (Rust style: 1_000_000)
    let value_clean = value.replace('_', "");

    // Try parsing as different types
    if value_clean == "true" {
        Some(Value::Bool(true))
    } else if value_clean == "false" {
        Some(Value::Bool(false))
    } else if let Ok(num) = value_clean.parse::<i64>() {
        Some(Value::Number(num.into()))
    } else if let Ok(num) = value_clean.parse::<f64>() {
        serde_json::Number::from_f64(num).map(Value::Number)
    } else if (value.starts_with('"') && value.ends_with('"'))
        || (value.starts_with('\'') && value.ends_with('\''))
    {
        Some(Value::String(value[1..value.len() - 1].to_string()))
    } else {
        // Could be a field reference - for now, treat as string
        Some(Value::String(value.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_comparison() {
        let parsed = parse_condition_expression("amount > 1000").unwrap();
        match parsed {
            ParsedCondition::Comparison { field, op, value } => {
                assert_eq!(field.segments, vec!["amount"]);
                assert!(matches!(op, ComparisonOp::GreaterThan));
                assert_eq!(value, serde_json::json!(1000));
            }
            _ => panic!("Expected comparison"),
        }
    }

    #[test]
    fn test_numeric_with_underscores() {
        let parsed = parse_condition_expression("amount > 1_000_000_000_000").unwrap();
        match parsed {
            ParsedCondition::Comparison { field, op, value } => {
                assert_eq!(field.segments, vec!["amount"]);
                assert!(matches!(op, ComparisonOp::GreaterThan));
                assert_eq!(value, serde_json::json!(1000000000000i64));
            }
            _ => panic!("Expected comparison"),
        }
    }

    #[test]
    fn test_logical_and() {
        let parsed = parse_condition_expression("amount > 100 && user != \"excluded\"").unwrap();
        match parsed {
            ParsedCondition::Logical { op, conditions } => {
                assert!(matches!(op, LogicalOp::And));
                assert_eq!(conditions.len(), 2);
            }
            _ => panic!("Expected logical"),
        }
    }

    #[test]
    fn test_nested_field_path() {
        let parsed = parse_condition_expression("data.amount >= 500").unwrap();
        match parsed {
            ParsedCondition::Comparison { field, op, value } => {
                assert_eq!(field.segments, vec!["data", "amount"]);
                assert!(matches!(op, ComparisonOp::GreaterThanOrEqual));
                assert_eq!(value, serde_json::json!(500));
            }
            _ => panic!("Expected comparison"),
        }
    }
}
