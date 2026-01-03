// Stage 2: Structural processing
//
// Handle indentation, newlines, EOF per schema rules.
// For indentation-based languages (Lumen/Mini-Python):
//   - Convert indentation levels to synthetic { } block markers
// For brace-based languages (Mini-Rust):
//   - Just ensure EOF is present
// No semantic interpretation.

use super::ingest::Token;
use crate::schema::LanguageSchema;

/// Check if a keyword expects a block
fn keyword_expects_block(keyword: &str) -> bool {
    keyword == "if" || keyword == "while" || keyword == "for" || keyword == "else"
}

/// Find the most recent block-expecting keyword in result
/// Returns the keyword if found, or empty string
fn find_last_keyword(result: &[Token]) -> String {
    // Scan backwards to find a block-expecting keyword that starts a statement
    // A keyword is at a statement boundary if:
    // - It's at the beginning, OR
    // - The token immediately before it (skipping internal expressions) is a newline or }

    // First, find the most recent block-expecting keyword anywhere
    for i in (0..result.len()).rev() {
        let token = &result[i];

        if keyword_expects_block(&token.lexeme) {
            // Check if this keyword starts a statement
            // It starts a statement if the previous non-whitespace token is newline, }, or beginning
            if i == 0 {
                return token.lexeme.clone();
            }

            // Look immediately before this keyword (skipping whitespace)
            // to see if there's a newline or }
            if i > 0 {
                let mut found_boundary = false;
                for j in (0..i).rev() {
                    let prev = &result[j];
                    if prev.lexeme == " " || prev.lexeme == "\t" {
                        continue; // Skip whitespace
                    }
                    if prev.lexeme == "\n" || prev.lexeme == "}" {
                        found_boundary = true;
                    }
                    break; // Stop at first non-whitespace token
                }
                if found_boundary {
                    return token.lexeme.clone();
                }
            }
        }
    }

    String::new()
}

/// Process tokens for structural significance per schema
pub fn process(tokens: &[Token], schema: &LanguageSchema) -> Result<Vec<Token>, String> {
    let mut result = Vec::new();

    // Only process indentation for indentation-based languages
    if !schema.is_indentation_based {
        // For brace-based languages, just remove indentation whitespace
        for token in tokens {
            if token.lexeme == " " || token.lexeme == "\t" {
                continue; // Skip individual space/tab tokens
            }
            result.push(token.clone());
        }
        result = ensure_eof(result);
        return Ok(result);
    }

    // Indentation processing for indentation-based languages
    let mut indent_stack: Vec<usize> = vec![0]; // Track indentation levels
    let mut i = 0;

    while i < tokens.len() {
        let token = &tokens[i];

        // Handle newlines - check indentation of next line
        if token.lexeme == "\n" {
            result.push(token.clone());
            i += 1;

            // Count indentation on next line
            let mut indent_level = 0;
            let mut j = i;
            while j < tokens.len() {
                let t = &tokens[j];
                if t.lexeme == " " {
                    indent_level += 1;
                    j += 1;
                } else if t.lexeme == "\t" {
                    indent_level += 4;
                    j += 1;
                } else {
                    break;
                }
            }

            // Skip empty lines
            if j >= tokens.len() || tokens[j].lexeme == "\n" {
                i = j;
                continue;
            }

            // Check what keyword this indent follows
            let last_keyword = find_last_keyword(&result);
            let should_have_block = !last_keyword.is_empty();

            // Process indentation changes
            let current_indent = *indent_stack.last().unwrap_or(&0);

            if indent_level > current_indent && should_have_block {
                indent_stack.push(indent_level);
                result.push(Token {
                    lexeme: "{".to_string(),
                    span: (token.span.0, token.span.0),
                    line: token.line,
                    col: 0,
                });
            } else if indent_level < current_indent {
                while indent_stack.len() > 1 && indent_stack[indent_stack.len() - 1] > indent_level {
                    indent_stack.pop();
                    result.push(Token {
                        lexeme: "}".to_string(),
                        span: (token.span.0, token.span.0),
                        line: token.line,
                        col: 0,
                    });
                }
            }

            // Skip the whitespace tokens we counted
            i = j;
            continue;
        }

        // Skip standalone whitespace tokens (not at line start)
        if token.lexeme == " " || token.lexeme == "\t" {
            i += 1;
            continue;
        }

        // Regular token
        result.push(token.clone());
        i += 1;
    }

    // Close any remaining indentation levels
    while indent_stack.len() > 1 {
        indent_stack.pop();
        result.push(Token {
            lexeme: "}".to_string(),
            span: (0, 0),
            line: 0,
            col: 0,
        });
    }

    // Ensure EOF is present
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
