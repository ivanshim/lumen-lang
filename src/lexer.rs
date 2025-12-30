// src/lexer.rs

use crate::registry::LumenResult;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // atoms
    Ident(String),
    Number(f64),
    String(String),

    // keywords
    If,
    Else,
    While,
    Print,
    And,
    Or,
    Not,

    // operators / syntax
    Plus,
    Minus,
    Star,
    Slash,
    Eq,
    EqEq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    LParen,
    RParen,
    Colon,

    // layout
    Newline,
    Indent,
    Dedent,
    Eof,
}

#[derive(Debug, Clone)]
pub struct SpannedToken {
    pub tok: Token,
    pub line: usize,
    pub col: usize,
}

impl SpannedToken {
    fn new(tok: Token, line: usize, col: usize) -> Self {
        Self { tok, line, col }
    }
}

// ... keep lex() as you have it ...

fn lex_line(
    s: &str,
    line: usize,
    base_col: usize,
    out: &mut Vec<SpannedToken>,
) -> LumenResult<()> {
    let mut i = 0;
    let bytes = s.as_bytes();

    while i < bytes.len() {
        let col = base_col + i;

        if bytes[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }

        // strings: "..."
        if bytes[i] == b'"' {
            i += 1;
            let start = i;
            while i < bytes.len() && bytes[i] != b'"' {
                i += 1;
            }
            if i >= bytes.len() {
                return Err(format!("Unterminated string at line {line}"));
            }
            let val = &s[start..i];
            i += 1;
            out.push(SpannedToken::new(Token::String(val.to_string()), line, col));
            continue;
        }

        // numbers: 123 or 12.34
        if bytes[i].is_ascii_digit() {
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i < bytes.len() && bytes[i] == b'.' {
                i += 1;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let n: f64 = s[start..i].parse().unwrap();
            out.push(SpannedToken::new(Token::Number(n), line, col));
            continue;
        }

        // identifiers / keywords
        if is_word_start(bytes[i]) {
            let start = i;
            i += 1;
            while i < bytes.len() && is_word_continue(bytes[i]) {
                i += 1;
            }
            let w = &s[start..i];
            let tok = match w {
                "if" => Token::If,
                "else" => Token::Else,
                "while" => Token::While,
                "print" => Token::Print,
                "and" => Token::And,
                "or" => Token::Or,
                "not" => Token::Not,
                _ => Token::Ident(w.to_string()),
            };
            out.push(SpannedToken::new(tok, line, col));
            continue;
        }

        // operators / punctuation (multi-char first)
        match bytes[i] {
            b'=' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'=' {
                    i += 2;
                    out.push(SpannedToken::new(Token::EqEq, line, col));
                } else {
                    i += 1;
                    out.push(SpannedToken::new(Token::Eq, line, col));
                }
            }
            b'!' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'=' {
                    i += 2;
                    out.push(SpannedToken::new(Token::NotEq, line, col));
                } else {
                    return Err(format!("Unexpected '!' at line {line}, col {col}"));
                }
            }
            b'<' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'=' {
                    i += 2;
                    out.push(SpannedToken::new(Token::LtEq, line, col));
                } else {
                    i += 1;
                    out.push(SpannedToken::new(Token::Lt, line, col));
                }
            }
            b'>' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'=' {
                    i += 2;
                    out.push(SpannedToken::new(Token::GtEq, line, col));
                } else {
                    i += 1;
                    out.push(SpannedToken::new(Token::Gt, line, col));
                }
            }
            b'+' => {
                i += 1;
                out.push(SpannedToken::new(Token::Plus, line, col));
            }
            b'-' => {
                i += 1;
                out.push(SpannedToken::new(Token::Minus, line, col));
            }
            b'*' => {
                i += 1;
                out.push(SpannedToken::new(Token::Star, line, col));
            }
            b'/' => {
                i += 1;
                out.push(SpannedToken::new(Token::Slash, line, col));
            }
            b'(' => {
                i += 1;
                out.push(SpannedToken::new(Token::LParen, line, col));
            }
            b')' => {
                i += 1;
                out.push(SpannedToken::new(Token::RParen, line, col));
            }
            b':' => {
                i += 1;
                out.push(SpannedToken::new(Token::Colon, line, col));
            }
            _ => {
                let ch = bytes[i] as char;
                return Err(format!("Unexpected character '{ch}' at line {line}, col {col}"));
            }
        }
    }

    Ok(())
}

fn is_word_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

fn is_word_continue(b: u8) -> bool {
    is_word_start(b) || b.is_ascii_digit()
}
