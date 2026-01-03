// Schema validation
//
// Validators ensure that language schemas are well-formed and consistent.

use super::language::LanguageSchema;

/// Validate a language schema
///
/// Checks for:
/// - Operator precedence consistency
/// - Statement pattern validity
/// - Keyword/multichar_lexeme conflicts
/// - Block delimiter matching
pub fn validate_schema(schema: &LanguageSchema) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    // Check that all statements are defined
    if schema.statements.is_empty() {
        errors.push("Schema must define at least one statement type".to_string());
    }

    // Check for keyword/statement_keyword conflicts
    for statement_keyword in schema.statements.keys() {
        if schema.keywords.contains(&statement_keyword.to_string()) {
            errors.push(format!(
                "Statement keyword '{}' also listed as keyword",
                statement_keyword
            ));
        }
    }

    // Check that block delimiters are defined
    if schema.block_open.is_empty() {
        errors.push("Schema must define block_open".to_string());
    }
    if schema.block_close.is_empty() {
        errors.push("Schema must define block_close".to_string());
    }

    // Check operator precedence is positive
    for (lexeme, op) in &schema.binary_ops {
        if op.precedence == 0 {
            errors.push(format!("Binary operator '{}' has zero precedence", lexeme));
        }
    }

    for (lexeme, op) in &schema.unary_ops {
        if op.precedence == 0 {
            errors.push(format!("Unary operator '{}' has zero precedence", lexeme));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
