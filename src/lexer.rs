// src/lexer.rs
//
// Pure lexical analysis + indentation handling.
// No AST. No evaluation.
// Converts source text -> tokens (including INDENT/DEDENT like Python).

use crate::registry::LumenResult;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // atoms
    Ident(String),
    Number(f64),
    String(String),

    // punctuation / operators
    LParen,
    RParen,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Equals,
    EqEq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,

    // keywords (still "lexical", not semantic)
    If,
    Else,
    While,
    And,
    Or,
    Not,
    True,
    False,

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

pub fn lex(source: &str) -> LumenResult<Vec<SpannedToken>> {
    let mut out = Vec::new();
    let mut indents = vec![0usize];

    let mut line_no = 1usize;

    for raw in source.lines() {
        // count leading spaces
        let mut spaces = 0usize;
        for ch in raw.chars() {
            if ch == ' ' {
                spaces += 1;
            } else {
                break;
            }
        }

        let rest = &raw[spaces..];

        // skip blank / whitespace-only lines (do not emit NEWLINE)
        if rest.trim().is_empty() {
            line_no += 1;
            continue;
        }

        // indentation handling (4-space indents)
        let current = *indents.last().unwrap();
        if spaces > current {
            if (spaces - current) % 4 != 0 {
                return Err(format!("Invalid indentation at line {line_no}"));
            }
            indents.push(spaces);
            out.push(SpannedToken::new(Token::Indent, line_no, 1));
        } else if spaces < current {
            while *indents.last().unwrap() > spaces {
                indents.pop();
                out.push(SpannedToken::new(Token::Dedent, line_no, 1));
            }
            if *indents.last().unwrap() != spaces {
                return Err(format!("Indentation mismatch at line {line_no}"));
            }
        }

        lex_line(rest, line_no, spaces + 1, &mut out)?;
        out.push(SpannedToken::new(Token::Newline, line_no, spaces + rest.len() + 1));
        line_no += 1;
    }

    while indents.len() > 1 {
        indents.pop();
        out.push(SpannedToken::new(Token::Dedent, line_no, 1));
    }

    out.push(SpannedToken::new(Token::Eof, line_no, 1));
    Ok(out)
}

fn lex_line(s: &str, line: usize, base_col: usize, out: &mut Vec<SpannedToken>) -> LumenResult<()> {
    let bytes = s.as_bytes();
    let mut i = 0usize;

    while i < bytes.len() {
        let col = base_col + i;

        // whitespace
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

        // numbers: 123 or 123.45
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
            let word = &s[start..i];
            let tok = match word {
                "if" => Token::If,
                "else" => Token::Else,
                "while" => Token::While,
                "and" => Token::And,
                "or" => Token::Or,
                "not" => Token::Not,
                "true" => Token::True,
                "false" => Token::False,
                _ => Token::Ident(word.to_string()),
            };
            out.push(SpannedToken::new(tok, line, col));
            continue;
        }

        // two-char operators
        if i + 1 < bytes.len() {
            let two = &s[i..i + 2];
            let tok2 = match two {
                "==" => Some(Token::EqEq),
                "!=" => Some(Token::NotEq),
                "<=" => Some(Token::LtEq),
                ">=" => Some(Token::GtEq),
                _ => None,
            };
            if let Some(t) = tok2 {
                out.push(SpannedToken::new(t, line, col));
                i += 2;
                continue;
            }
        }

        // single-char operators / punctuation
        let ch = bytes[i] as char;
        let tok = match ch {
            '(' => Token::LParen,
            ')' => Token::RParen,
            '+' => Token::Plus,
            '-' => Token::Minus,
            '*' => Token::Star,
            '/' => Token::Slash,
            '%' => Token::Percent,
            '=' => Token::Equals,
            '<' => Token::Lt,
            '>' => Token::Gt,
            _ => return Err(format!("Unexpected character '{ch}' at line {line}, col {col}")),
        };
        out.push(SpannedToken::new(tok, line, col));
        i += 1;
    }

    Ok(())
}

fn is_word_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}
fn is_word_continue(b: u8) -> bool {
    is_word_start(b) || b.is_ascii_digit()
}
