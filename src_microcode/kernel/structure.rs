// Stage 2: Structure - Indentation and block processing
//
// For indentation-based languages (Lumen, Python):
//   Convert indentation levels → { } block markers
// For brace-based languages (Rust):
//   Just pass through (braces already in token stream)
//
// Algorithm:
// 1. Process line-by-line to track indentation
// 2. When indentation increases, insert {
// 3. When indentation decreases, insert }
// 4. Handle colons as block openers (for languages that use them, like PythonCore)

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

    let mut result = Vec::new();
    let mut indent_stack = vec![0];
    let mut i = 0;

    while i < tokens.len() {
        // Skip to next line start if not at beginning
        if i > 0 && tokens[i - 1].lexeme != "\n" && result.last().map(|t: &Token| t.lexeme.as_str()) != Some("\n") {
            // Not at line start, just add token
            result.push(tokens[i].clone());
            i += 1;
            continue;
        }

        // We're at line start. Measure indentation
        let mut indent_level = 0;

        // Count indentation (spaces or tabs)
        while i < tokens.len() && (tokens[i].lexeme == " " || tokens[i].lexeme == "\t") {
            if tokens[i].lexeme == " " {
                indent_level += 1;
            } else {
                indent_level += schema.indentation_size;
            }
            result.push(tokens[i].clone());
            i += 1;
        }

        // Skip empty/blank lines
        if i < tokens.len() && tokens[i].lexeme == "\n" {
            result.push(tokens[i].clone());
            i += 1;
            continue;
        }

        // Convert indent level to indentation units
        let indent_units = indent_level / schema.indentation_size;
        let current_indent = *indent_stack.last().unwrap();

        // Handle indentation changes
        if indent_units > current_indent {
            // Indentation increased: insert {
            indent_stack.push(indent_units);
            result.push(Token {
                lexeme: "{".to_string(),
                span: (tokens[i].span.0, tokens[i].span.0),
                line: tokens[i].line,
                col: 0,
            });
        } else if indent_units < current_indent {
            // Indentation decreased: insert } for each level
            while indent_stack.len() > 1 && *indent_stack.last().unwrap() > indent_units {
                indent_stack.pop();
                result.push(Token {
                    lexeme: "}".to_string(),
                    span: (tokens[i].span.0, tokens[i].span.0),
                    line: tokens[i].line,
                    col: 0,
                });
            }
        }

        // Process tokens on this line until newline
        while i < tokens.len() && tokens[i].lexeme != "\n" {
            // If we see a colon, mark end of line for block (for languages like PythonCore)
            if tokens[i].lexeme == ":" && schema.block_open_marker == ":" {
                result.push(tokens[i].clone());
                i += 1;
                // Skip whitespace after colon
                while i < tokens.len() && tokens[i].lexeme == " " {
                    result.push(tokens[i].clone());
                    i += 1;
                }
                // Next line will handle indentation increase
                break;
            }

            result.push(tokens[i].clone());
            i += 1;
        }

        // Add newline if present
        if i < tokens.len() && tokens[i].lexeme == "\n" {
            result.push(tokens[i].clone());
            i += 1;
        }
    }

    // Close all remaining open indentation blocks
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
