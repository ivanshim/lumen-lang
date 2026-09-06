// Kernel lexer: lossless, maximal-munch segmentation with no semantics.
//
// Guarantees:
//   1. Lossless: every input character becomes part of exactly one token.
//   2. Maximal munch: the longest registered multi-character lexeme wins.
//   3. Fallback: with no multi-character match, one character becomes one token.
//   4. Position: every token carries its byte span and diagnostic line/col.
//
// The lexer has no notion of whitespace, comments, strings, numbers or
// identifiers. Languages strip comments before lexing, register the lexemes
// they care about, and interpret every token afterwards. The only language
// input beyond the lexeme list is the word-character predicate used for
// keyword boundary checks, also supplied through the registry.

use crate::kernel::registry::{KernelResult, TokenRegistry};

/// Byte span in the source: `start` inclusive, `end` exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub lexeme: String,
    pub span: Span,
}

impl Token {
    pub fn new(lexeme: String, span: Span) -> Self {
        Self { lexeme, span }
    }
}

/// A token with diagnostic line/col (derived metadata, never used for parsing).
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

/// Segment `source` into tokens using the registry's lexemes.
pub fn lex(source: &str, token_reg: &TokenRegistry) -> KernelResult<Vec<SpannedToken>> {
    let mut out = Vec::new();
    let bytes = source.as_bytes();
    let mut byte_pos = 0usize;
    let mut line_no = 1usize;
    let mut col_in_line = 1usize;

    while byte_pos < bytes.len() {
        let start_col = col_in_line;
        let remaining = &source[byte_pos..];
        let mut matched = false;

        for multichar in token_reg.multichar_lexemes() {
            let multichar = multichar.as_str();
            if !remaining.starts_with(multichar) {
                continue;
            }

            if token_reg.requires_word_boundary(multichar) {
                let end = byte_pos + multichar.len();
                let left_ok = source[..byte_pos].chars().next_back().map_or(true, |c| !token_reg.is_word_char(c));
                let right_ok = source[end..].chars().next().map_or(true, |c| !token_reg.is_word_char(c));
                if !(left_ok && right_ok) {
                    continue;
                }
            }

            let span = Span::new(byte_pos, byte_pos + multichar.len());
            out.push(SpannedToken::new(Token::new(multichar.to_string(), span), line_no, start_col));
            for byte in multichar.as_bytes() {
                advance_position(*byte, &mut line_no, &mut col_in_line);
            }
            byte_pos += multichar.len();
            matched = true;
            break;
        }

        if matched {
            continue;
        }

        // Fallback: one whole character (which may be several bytes) becomes one token.
        let ch = remaining.chars().next().expect("byte_pos is inside the source");
        let width = ch.len_utf8();
        let span = Span::new(byte_pos, byte_pos + width);
        out.push(SpannedToken::new(Token::new(ch.to_string(), span), line_no, start_col));
        advance_position(if ch == '\n' { b'\n' } else { b' ' }, &mut line_no, &mut col_in_line);
        byte_pos += width;
    }

    Ok(out)
}

fn advance_position(byte: u8, line_no: &mut usize, col_in_line: &mut usize) {
    if byte == b'\n' {
        *line_no += 1;
        *col_in_line = 1;
    } else {
        *col_in_line += 1;
    }
}
