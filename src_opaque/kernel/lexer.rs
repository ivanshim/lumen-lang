// Lexer for opaque kernel - generic, language-agnostic tokenization
// Reuses the tokenization pattern from stream kernel

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub token_type: String,
    pub lexeme: String,
    pub line: usize,
    pub col: usize,
}

pub fn lex(source: &str, keywords: &[&str]) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = source.chars().peekable();
    let mut line = 1;
    let mut col = 1;

    while let Some(&ch) = chars.peek() {
        match ch {
            ' ' | '\t' => {
                chars.next();
                col += 1;
            }
            '\n' => {
                tokens.push(Token {
                    token_type: "newline".to_string(),
                    lexeme: "\n".to_string(),
                    line,
                    col,
                });
                chars.next();
                line += 1;
                col = 1;
            }
            '"' => {
                chars.next();
                let mut string = String::new();
                let start_col = col;
                col += 1;

                while let Some(&ch) = chars.peek() {
                    if ch == '"' {
                        chars.next();
                        col += 1;
                        break;
                    } else if ch == '\\' {
                        chars.next();
                        col += 1;
                        if let Some(&next_ch) = chars.peek() {
                            match next_ch {
                                'n' => string.push('\n'),
                                't' => string.push('\t'),
                                '"' => string.push('"'),
                                _ => string.push(next_ch),
                            }
                            chars.next();
                            col += 1;
                        }
                    } else {
                        string.push(ch);
                        chars.next();
                        col += 1;
                    }
                }

                tokens.push(Token {
                    token_type: "string".to_string(),
                    lexeme: string,
                    line,
                    col: start_col,
                });
            }
            c if c.is_ascii_digit() => {
                let start_col = col;
                let mut number = String::new();
                let mut has_dot = false;

                while let Some(&ch) = chars.peek() {
                    if ch.is_ascii_digit() {
                        number.push(ch);
                        chars.next();
                        col += 1;
                    } else if ch == '.' && !has_dot && chars.clone().nth(1) != Some('.') {
                        // Only include one dot, and not if followed by another dot (range operator)
                        has_dot = true;
                        number.push(ch);
                        chars.next();
                        col += 1;
                    } else {
                        break;
                    }
                }

                tokens.push(Token {
                    token_type: "number".to_string(),
                    lexeme: number,
                    line,
                    col: start_col,
                });
            }
            c if c.is_alphabetic() || c == '_' => {
                let start_col = col;
                let mut ident = String::new();

                while let Some(&ch) = chars.peek() {
                    if ch.is_alphanumeric() || ch == '_' {
                        ident.push(ch);
                        chars.next();
                        col += 1;
                    } else {
                        break;
                    }
                }

                let token_type = if keywords.contains(&ident.as_str()) {
                    format!("keyword_{}", ident)
                } else {
                    "identifier".to_string()
                };

                tokens.push(Token {
                    token_type,
                    lexeme: ident,
                    line,
                    col: start_col,
                });
            }
            '(' => {
                tokens.push(Token {
                    token_type: "lparen".to_string(),
                    lexeme: "(".to_string(),
                    line,
                    col,
                });
                chars.next();
                col += 1;
            }
            ')' => {
                tokens.push(Token {
                    token_type: "rparen".to_string(),
                    lexeme: ")".to_string(),
                    line,
                    col,
                });
                chars.next();
                col += 1;
            }
            '{' => {
                tokens.push(Token {
                    token_type: "lbrace".to_string(),
                    lexeme: "{".to_string(),
                    line,
                    col,
                });
                chars.next();
                col += 1;
            }
            '}' => {
                tokens.push(Token {
                    token_type: "rbrace".to_string(),
                    lexeme: "}".to_string(),
                    line,
                    col,
                });
                chars.next();
                col += 1;
            }
            '=' if chars.clone().nth(1) == Some('=') => {
                tokens.push(Token {
                    token_type: "operator".to_string(),
                    lexeme: "==".to_string(),
                    line,
                    col,
                });
                chars.next();
                chars.next();
                col += 2;
            }
            '!' if chars.clone().nth(1) == Some('=') => {
                tokens.push(Token {
                    token_type: "operator".to_string(),
                    lexeme: "!=".to_string(),
                    line,
                    col,
                });
                chars.next();
                chars.next();
                col += 2;
            }
            '=' => {
                tokens.push(Token {
                    token_type: "assign".to_string(),
                    lexeme: "=".to_string(),
                    line,
                    col,
                });
                chars.next();
                col += 1;
            }
            '+' | '-' | '*' | '/' | '%' | '>' | '<' => {
                let op = ch.to_string();
                tokens.push(Token {
                    token_type: "operator".to_string(),
                    lexeme: op,
                    line,
                    col,
                });
                chars.next();
                col += 1;
            }
            ';' => {
                tokens.push(Token {
                    token_type: "semicolon".to_string(),
                    lexeme: ";".to_string(),
                    line,
                    col,
                });
                chars.next();
                col += 1;
            }
            ',' => {
                tokens.push(Token {
                    token_type: "comma".to_string(),
                    lexeme: ",".to_string(),
                    line,
                    col,
                });
                chars.next();
                col += 1;
            }
            '.' if chars.clone().nth(1) == Some('.') => {
                // Range operator: .. or ..=
                let start_col = col;
                chars.next(); // consume first dot
                col += 1;
                chars.next(); // consume second dot
                col += 1;

                let lexeme = if chars.peek() == Some(&'=') {
                    chars.next();
                    col += 1;
                    "..=".to_string()
                } else {
                    "..".to_string()
                };

                tokens.push(Token {
                    token_type: "range_op".to_string(),
                    lexeme,
                    line,
                    col: start_col,
                });
            }
            _ => {
                chars.next();
                col += 1;
            }
        }
    }

    tokens
}
