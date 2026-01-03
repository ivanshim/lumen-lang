// Stage 1: Ingestion
//
// Lossless segmentation of source into lexemes.
// Uses schema tables for multi-char sequences.
// No semantic interpretation.

use crate::schema::LanguageSchema;

#[derive(Debug, Clone)]
pub struct Token {
    pub lexeme: String,
    pub span: (usize, usize), // byte range
    pub line: usize,
    pub col: usize,
}

/// Lex source using schema tables
/// No assumptions about what lexemes mean.
pub fn lex(source: &str, schema: &LanguageSchema) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let bytes = source.as_bytes();
    let mut pos = 0;
    let mut line = 1;
    let mut col = 1;

    while pos < bytes.len() {
        let start_col = col;
        let remaining = &source[pos..];

        // Try maximal-munch with schema's multi-char sequences
        let mut matched = false;
        for &sequence in &schema.multichar_lexemes {
            if remaining.starts_with(sequence) {
                // Check if this is an identifier-like keyword that needs word boundary checking
                let is_identifier_keyword = sequence.chars().all(|c| c.is_alphabetic() || c == '_');
                let mut can_match = true;

                if is_identifier_keyword {
                    // Check the character after the match
                    let after_pos = pos + sequence.len();
                    if after_pos < bytes.len() {
                        let next_byte = bytes[after_pos];
                        let next_ch = next_byte as char;
                        // If the next character is alphanumeric or underscore, don't match
                        if next_ch.is_alphanumeric() || next_ch == '_' {
                            can_match = false;
                        }
                    }
                }

                if can_match {
                    tokens.push(Token {
                        lexeme: sequence.to_string(),
                        span: (pos, pos + sequence.len()),
                        line,
                        col: start_col,
                    });

                    // Update position tracking
                    for byte in sequence.as_bytes() {
                        if *byte == b'\n' {
                            line += 1;
                            col = 1;
                        } else {
                            col += 1;
                        }
                    }

                    pos += sequence.len();
                    matched = true;
                    break;
                }
            }
        }

        if matched {
            continue;
        }

        // Single character token
        let byte = bytes[pos];
        let ch = byte as char;
        tokens.push(Token {
            lexeme: ch.to_string(),
            span: (pos, pos + 1),
            line,
            col: start_col,
        });

        if byte == b'\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }

        pos += 1;
    }

    Ok(tokens)
}
