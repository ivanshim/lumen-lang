// Stage 2: Structure - Indentation and block processing
//
// For indentation-based languages (Lumen, Python):
//   Convert indentation levels → { } block markers
// For brace-based languages (Rust):
//   Just pass through (braces already in token stream)
//
// Algorithm:
// 1. Track indentation level on each line
// 2. When indentation increases after ":", insert {
// 3. When indentation decreases, insert }
// 4. Skip blank lines and normalize indentation handling

use super::ingest::Token;
use crate::schema::LanguageSchema;

/// Process indentation and insert block markers
pub fn process_structure(
    tokens: Vec<Token>,
    schema: &LanguageSchema,
) -> Result<Vec<Token>, String> {
    // For brace-based languages, skip processing
    if schema.block_open_marker == "{" {
        return Ok(tokens);
    }

    // For indentation-based languages
    let mut result = Vec::new();
    let mut indent_stack = vec![0]; // Track indentation levels
    let mut current_indent = 0;
    let mut i = 0;

    while i < tokens.len() {
        let token = &tokens[i];

        // Track line starts to detect indentation
        if i == 0 || (i > 0 && tokens[i - 1].lexeme == "\n") {
            // Count spaces/tabs for indentation
            let mut spaces = 0;
            let mut j = i;

            while j < tokens.len() && (tokens[j].lexeme == " " || tokens[j].lexeme == "\t") {
                if tokens[j].lexeme == " " {
                    spaces += 1;
                } else if tokens[j].lexeme == "\t" {
                    spaces += schema.indentation_size;
                }
                j += 1;
            }

            // Skip blank lines
            if j < tokens.len() && tokens[j].lexeme == "\n" {
                // Just output the whitespace and newline
                while i < j {
                    result.push(tokens[i].clone());
                    i += 1;
                }
                result.push(tokens[j].clone());
                i = j + 1;
                continue;
            }

            current_indent = spaces / schema.indentation_size;

            // Skip whitespace tokens
            while i < j {
                i += 1;
            }

            // Process indent changes
            let last_indent = *indent_stack.last().unwrap();

            if current_indent > last_indent {
                // Increased indentation: insert {
                indent_stack.push(current_indent);
                result.push(Token {
                    lexeme: "{".to_string(),
                    span: (token.span.0, token.span.0),
                    line: token.line,
                    col: token.col,
                });
            } else if current_indent < last_indent {
                // Decreased indentation: insert } for each level
                while indent_stack.len() > 1 && indent_stack[indent_stack.len() - 1] > current_indent {
                    indent_stack.pop();
                    result.push(Token {
                        lexeme: "}".to_string(),
                        span: (token.span.0, token.span.0),
                        line: token.line,
                        col: token.col,
                    });
                }
            }

            if i >= tokens.len() {
                break;
            }
        }

        // Check for colon (block opener)
        if token.lexeme == ":" && schema.block_open_marker == ":" {
            result.push(token.clone());
            i += 1;

            // Skip newline after colon
            if i < tokens.len() && tokens[i].lexeme == "\n" {
                result.push(tokens[i].clone());
                i += 1;
            }

            // The { will be inserted on next line when indentation increases
            continue;
        }

        // Regular token
        result.push(token.clone());
        i += 1;
    }

    // Close all remaining open blocks
    while indent_stack.len() > 1 {
        indent_stack.pop();
        result.push(Token {
            lexeme: "}".to_string(),
            span: (0, 0),
            line: 0,
            col: 0,
        });
    }

    Ok(result)
}
