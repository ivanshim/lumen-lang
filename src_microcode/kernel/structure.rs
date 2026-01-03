// Stage 2: Structural processing
//
// Handle indentation, newlines, EOF per schema rules.
// For Lumen/Mini-Python: process indentation
// For Mini-Rust: just add EOF
// No semantic interpretation.

use super::ingest::Token;
use crate::schema::LanguageSchema;

/// Process tokens for structural significance per schema
pub fn process(tokens: &[Token], _schema: &LanguageSchema) -> Result<Vec<Token>, String> {
    let mut result = tokens.to_vec();

    // TODO: Add indentation processing based on schema
    // For now, just ensure we have EOF
    result = ensure_eof(result);

    Ok(result)
}

fn ensure_eof(mut tokens: Vec<Token>) -> Vec<Token> {
    // If EOF is not already present, add it
    if tokens.last().map(|t| t.lexeme.as_str()) != Some("EOF") {
        let last_line = tokens.last().map(|t| t.line).unwrap_or(1);
        let last_col = tokens.last().map(|t| t.col).unwrap_or(1);
        tokens.push(Token {
            lexeme: "EOF".to_string(),
            span: (0, 0),
            line: last_line,
            col: last_col,
        });
    }
    tokens
}
