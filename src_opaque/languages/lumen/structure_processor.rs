// Lumen structure processor for opaque kernel
// Handles indentation without semantic knowledge

use crate::kernel::lexer::Token;

/// Process Lumen source to add indentation markers
/// Converts indentation into explicit block markers (marker_indent_start, marker_indent_end)
pub fn process_indentation(_source: &str, tokens: Vec<Token>) -> Result<Vec<Token>, String> {
    let mut result = Vec::new();
    let mut current_depth = 0i32;
    let mut depth_stack = vec![0i32];

    for token in tokens {
        // Check if this is an indent token
        if token.token_type == "indent" {
            // Calculate depth from indent length (4 spaces = 1 depth)
            let spaces = token.lexeme.len();
            let depth = (spaces / 4) as i32;

            // Don't emit indent tokens, use them to track depth
            if depth > current_depth {
                // Going deeper - emit start markers
                for _ in current_depth..depth {
                    result.push(Token {
                        token_type: "marker_indent_start".to_string(),
                        lexeme: "".to_string(),
                        line: token.line,
                        col: token.col,
                    });
                }
                current_depth = depth;
                depth_stack.push(depth);
            } else if depth < current_depth {
                // Going shallower - emit end markers
                while depth_stack.len() > 1 && *depth_stack.last().unwrap() > depth {
                    result.push(Token {
                        token_type: "marker_indent_end".to_string(),
                        lexeme: "".to_string(),
                        line: token.line,
                        col: token.col,
                    });
                    depth_stack.pop();
                }
                current_depth = depth;
            }
            // Skip the indent token itself
        } else if token.token_type == "newline" {
            // Keep newlines
            result.push(token);
        } else {
            // Keep all other tokens
            result.push(token);
        }
    }

    // At end of file, emit remaining end markers
    while depth_stack.len() > 1 {
        result.push(Token {
            token_type: "marker_indent_end".to_string(),
            lexeme: "".to_string(),
            line: 1,
            col: 1,
        });
        depth_stack.pop();
    }

    Ok(result)
}
