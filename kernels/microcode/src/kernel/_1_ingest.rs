// Stage 1: Ingest — source text → tokens.
//
// Tokens are meaningful units: words, numbers, strings (already unescaped),
// operators, line ends and leading indentation. What counts as a comment,
// a string delimiter, an escape or an operator comes from the schema; the
// scanning algorithms are the kernel's.

use crate::schema::LanguageSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Word,
    Number,
    Str,
    Op,
    Newline,
    /// Leading whitespace of a non-blank line; `width` holds its size.
    Indent,
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: Kind,
    pub text: String,
    pub width: usize,
    pub line: usize,
    pub col: usize,
}

impl Token {
    fn new(kind: Kind, text: String, line: usize, col: usize) -> Self {
        Token { kind, text, width: 0, line, col }
    }

    pub fn is(&self, kind: Kind, text: &str) -> bool {
        self.kind == kind && self.text == text
    }
}

/// Remove line comments introduced by the schema's marker, outside strings,
/// keeping newlines so line numbers stay stable.
fn strip_comments(source: &str, schema: &LanguageSchema) -> String {
    let marker = match &schema.lexical.comment {
        Some(m) if !m.is_empty() => m.as_str(),
        _ => return source.to_string(),
    };
    let mut out = String::with_capacity(source.len());
    let mut rest = source;
    let mut in_string: Option<char> = None;
    let mut escaped = false;
    while let Some(ch) = rest.chars().next() {
        if let Some(q) = in_string {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == q {
                in_string = None;
            }
            rest = &rest[ch.len_utf8()..];
            continue;
        }
        if schema.lexical.quotes.contains(&ch) {
            in_string = Some(ch);
            out.push(ch);
            rest = &rest[ch.len_utf8()..];
            continue;
        }
        if rest.starts_with(marker) {
            rest = match rest.find('\n') {
                Some(nl) => &rest[nl..],
                None => "",
            };
            continue;
        }
        out.push(ch);
        rest = &rest[ch.len_utf8()..];
    }
    out
}

/// Identifier characters: underscore always; letters (and, for continuation,
/// digits) from either ASCII or all of Unicode, as the schema chooses.
fn is_word_start(c: char, unicode: bool) -> bool {
    match (c, unicode) {
        ('_', _) => true,
        (c, true) => c.is_alphabetic(),
        (c, false) => c.is_ascii_alphabetic(),
    }
}

fn is_word_char(c: char, unicode: bool) -> bool {
    match (c, unicode) {
        ('_', _) => true,
        (c, true) => c.is_alphanumeric(),
        (c, false) => c.is_ascii_alphanumeric(),
    }
}

pub fn lex(source: &str, schema: &LanguageSchema) -> Result<Vec<Token>, String> {
    let source = strip_comments(source, schema);
    let operators = schema.operators_longest_first();
    let number = &schema.lexical.number;
    let unicode = schema.lexical.identifier_unicode;
    let chars: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    let mut line = 1;
    let mut col = 1;
    let mut at_line_start = true;

    while i < chars.len() {
        if at_line_start {
            // Measure indentation; blank lines produce nothing at all.
            let mut width = 0;
            let mut j = i;
            while j < chars.len() && (chars[j] == ' ' || chars[j] == '\t') {
                width += if chars[j] == '\t' { schema.structure.indent_size } else { 1 };
                j += 1;
            }
            let line_end = chars[j..].iter().position(|&c| c == '\n').map(|p| j + p).unwrap_or(chars.len());
            let blank = chars[j..line_end].iter().all(|c| c.is_whitespace());
            if blank {
                i = if line_end < chars.len() { line_end + 1 } else { line_end };
                line += 1;
                col = 1;
                continue;
            }
            let mut tok = Token::new(Kind::Indent, String::new(), line, 1);
            tok.width = width;
            tokens.push(tok);
            col += j - i;
            i = j;
            at_line_start = false;
        }

        let c = chars[i];

        if c == '\n' {
            tokens.push(Token::new(Kind::Newline, "\n".to_string(), line, col));
            i += 1;
            line += 1;
            col = 1;
            at_line_start = true;
            continue;
        }

        if c == ' ' || c == '\t' || c == '\r' {
            i += 1;
            col += 1;
            continue;
        }

        let start_col = col;

        // Strings: delimiter from the schema, escapes from its table.
        if schema.lexical.quotes.contains(&c) {
            let quote = c;
            let escapes = schema.lexical.escapes.get(&quote);
            let mut text = String::new();
            i += 1;
            col += 1;
            let mut closed = false;
            while i < chars.len() {
                let ch = chars[i];
                if ch == '\\' && i + 1 < chars.len() {
                    let next = chars[i + 1];
                    match escapes.and_then(|table| table.get(&next)) {
                        Some(replacement) => text.push_str(replacement),
                        None => {
                            text.push('\\');
                            text.push(next);
                        }
                    }
                    i += 2;
                    col += 2;
                    continue;
                }
                if ch == quote {
                    closed = true;
                    i += 1;
                    col += 1;
                    break;
                }
                if ch == '\n' {
                    line += 1;
                    col = 0;
                }
                text.push(ch);
                i += 1;
                col += 1;
            }
            if !closed {
                return Err(format!("Unterminated {} string", quote));
            }
            tokens.push(Token::new(Kind::Str, text, line, start_col));
            continue;
        }

        // Numbers: digits, then either a base-N tail or a decimal fraction.
        if c.is_ascii_digit() {
            let mut text = String::new();
            while i < chars.len() && chars[i].is_ascii_digit() {
                text.push(chars[i]);
                i += 1;
            }
            let at_base_marker = number.base_marker.map_or(false, |m| i < chars.len() && chars[i] == m);
            if at_base_marker {
                text.push(chars[i]);
                i += 1;
                while i < chars.len() {
                    let ch = chars[i];
                    let next_is_digit = i + 1 < chars.len() && chars[i + 1].is_ascii_alphanumeric();
                    if ch.is_ascii_alphanumeric()
                        || (number.decimal_point == Some(ch) && next_is_digit)
                        || (number.exponent_marker == Some(ch) && next_is_digit)
                    {
                        text.push(ch);
                        i += 1;
                    } else {
                        break;
                    }
                }
            } else if let Some(point) = number.decimal_point {
                if i + 1 < chars.len() && chars[i] == point && chars[i + 1].is_ascii_digit() {
                    text.push(point);
                    i += 1;
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        text.push(chars[i]);
                        i += 1;
                    }
                }
            }
            col += text.len();
            tokens.push(Token::new(Kind::Number, text, line, start_col));
            continue;
        }

        // Words: identifiers and keywords alike; meaning is decided later.
        if is_word_start(c, unicode) {
            let mut text = String::new();
            while i < chars.len() && is_word_char(chars[i], unicode) {
                text.push(chars[i]);
                i += 1;
            }
            col += text.chars().count();
            tokens.push(Token::new(Kind::Word, text, line, start_col));
            continue;
        }

        // Operators and punctuation: longest match from the schema.
        let rest: String = chars[i..].iter().take(8).collect();
        match operators.iter().find(|op| rest.starts_with(*op)) {
            Some(op) => {
                let n = op.chars().count();
                tokens.push(Token::new(Kind::Op, op.to_string(), line, start_col));
                i += n;
                col += n;
            }
            None => return Err(format!("Unexpected character '{}' at {}:{}", c, line, col)),
        }
    }

    tokens.push(Token::new(Kind::Eof, "EOF".to_string(), line, col));
    Ok(tokens)
}
