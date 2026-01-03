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

/// Process tokens for structural significance per schema
pub fn process(tokens: &[Token], _schema: &LanguageSchema) -> Result<Vec<Token>, String> {
    let mut result = Vec::new();
    let mut indent_stack: Vec<usize> = vec![0]; // Track indentation levels
    let mut i = 0;
    let mut last_statement_keyword = String::new(); // Track last control keyword

    while i < tokens.len() {
        let token = &tokens[i];

        // Remember control flow keywords that expect blocks
        if token.lexeme == "if" || token.lexeme == "while" || token.lexeme == "for" {
            last_statement_keyword = token.lexeme.clone();
        } else if token.lexeme == "else" {
            last_statement_keyword = "else".to_string();
        } else if token.lexeme == "\n" {
            // Reset keyword tracking on newline (unless followed by else)
            // Actually, we'll handle this below
        } else if token.lexeme != " " && token.lexeme != "\t" && !token.lexeme.is_empty() {
            // Only reset if it's a non-whitespace, non-newline token
            // But keep the keyword if we're about to see a newline+indent
        }

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
                last_statement_keyword.clear();
                continue;
            }

            // Process indentation changes only if the last keyword expects a block
            let current_indent = *indent_stack.last().unwrap_or(&0);
            let should_have_block =
                last_statement_keyword == "if" ||
                last_statement_keyword == "while" ||
                last_statement_keyword == "for" ||
                last_statement_keyword == "else";

            if indent_level > current_indent && should_have_block {
                indent_stack.push(indent_level);
                result.push(Token {
                    lexeme: "{".to_string(),
                    span: (token.span.0, token.span.0),
                    line: token.line,
                    col: 0,
                });
                last_statement_keyword.clear();
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
                last_statement_keyword.clear();
            } else {
                last_statement_keyword.clear();
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
