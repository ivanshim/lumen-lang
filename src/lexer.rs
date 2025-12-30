// src/lexer.rs
//
// Pure lexical analysis + indentation handling.
// NO language semantics.
// NO operators.
// NO keywords.

use crate::registry::LumenResult;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Word(String),
    Number(f64),
    String(String),
    Punct(char),

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
    let mut line_no = 1;

    for raw in source.lines() {
        let mut col = 1;
        let mut chars = raw.chars().peekable();

        // Count leading spaces
        let mut spaces = 0;
        while let Some(' ') = chars.peek() {
            chars.next();
            spaces += 1;
            col += 1;
        }

        let rest: String = chars.collect();
        if rest.trim().is_empty() {
            line_no += 1;
            continue;
        }

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
        }

        lex_line(&rest.trim(), line_no, col, &mut out)?;
        out.push(SpannedToken::new(Token::Newline, line_no, col));
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
    let mut i = 0;
    let bytes = s.as_bytes();

    while i < bytes.len() {
        let col = base_col + i;

        if bytes[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }

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

        if is_word_start(bytes[i]) {
            let start = i;
            i += 1;
            while i < bytes.len() && is_word_continue(bytes[i]) {
                i += 1;
            }
            out.push(SpannedToken::new(
                Token::Word(s[start..i].to_string()),
                line,
                col,
            ));
            continue;
        }

        // Single-character punctuation ONLY
        let ch = bytes[i] as char;
        i += 1;
        out.push(SpannedToken::new(Token::Punct(ch), line, col));
    }

    Ok(())
}

fn is_word_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

fn is_word_continue(b: u8) -> bool {
    is_word_start(b) || b.is_ascii_digit()
}
