// Stage 1: Ingest - Lossless tokenization
//
// Convert source → tokens using schema.
// Key principle: tokens are MEANINGFUL units (not characters).
// Strings are atomic, keywords are identified, operators are complete.

use crate::schema::LanguageSchema;

#[derive(Debug, Clone)]
pub struct Token {
    pub lexeme: String,
    pub span: (usize, usize),  // byte range in source
    pub line: usize,
    pub col: usize,
}

/// Strip single-line comments from source.
/// Comments start with # and continue until end of line.
/// Preserves newlines for correct line counting.
/// Respects string boundaries: # inside strings is not a comment.
fn strip_comments(source: &str) -> String {
    let mut result = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut in_string = false;
    let mut string_char = ' ';
    let mut escape_next = false;

    while let Some(ch) = chars.next() {
        // Handle escape sequences in strings
        if escape_next {
            result.push(ch);
            escape_next = false;
            continue;
        }

        if ch == '\\' && in_string {
            result.push(ch);
            escape_next = true;
            continue;
        }

        // Track string state (both single and double quotes)
        if !in_string && (ch == '"' || ch == '\'') {
            in_string = true;
            string_char = ch;
            result.push(ch);
        } else if in_string && ch == string_char {
            in_string = false;
            result.push(ch);
        } else if !in_string && ch == '#' {
            // Skip comment until newline (but preserve the newline)
            while let Some(c) = chars.next() {
                if c == '\n' {
                    result.push('\n');
                    break;
                }
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Tokenize source using schema's multichar sequences
pub fn lex(source: &str, schema: &LanguageSchema) -> Result<Vec<Token>, String> {
    let source = strip_comments(source);
    let mut tokens = Vec::new();
    let bytes = source.as_bytes();
    let mut pos = 0;
    let mut line = 1;
    let mut col = 1;

    while pos < bytes.len() {
        let start_col = col;
        let remaining = &source[pos..];

        // Try multichar sequences first (sorted by length, longest first)
        let mut matched = false;

        for &seq in &schema.multichar_lexemes {
            if remaining.starts_with(seq) {
                // Check word boundary for keywords
                let is_keyword = seq.chars().all(|c| c.is_alphabetic() || c == '_');
                if is_keyword {
                    let after_pos = pos + seq.len();
                    if after_pos < bytes.len() {
                        let next_byte = bytes[after_pos];
                        let next_ch = next_byte as char;
                        if next_ch.is_alphanumeric() || next_ch == '_' {
                            continue;  // Word boundary violated, try next
                        }
                    }
                }

                // Matched! Add token.
                tokens.push(Token {
                    lexeme: seq.to_string(),
                    span: (pos, pos + seq.len()),
                    line,
                    col: start_col,
                });

                // Update position
                for ch in seq.chars() {
                    if ch == '\n' {
                        line += 1;
                        col = 1;
                    } else {
                        col += 1;
                    }
                }

                pos += seq.len();
                matched = true;
                break;
            }
        }

        if matched {
            continue;
        }

        // No multichar match: emit single byte as token
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

    // Add EOF marker
    tokens.push(Token {
        lexeme: "EOF".to_string(),
        span: (pos, pos),
        line,
        col,
    });

    Ok(tokens)
}
