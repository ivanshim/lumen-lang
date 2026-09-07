// Text to tokens. Comments and the prologue are removed first, outside
// strings, keeping line ends; then strings, numbers, quoted names, words,
// symbols, line ends and indentation are cut out in one pass.

use crate::table::Table;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    Bare,
    Quoted,
    Numeral,
    Quote,
    Sign,
    LineEnd,
    Lead,
    Open,
    Close,
    Finish,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub shape: Shape,
    pub lexeme: String,
    pub span: usize,
    pub row: u32,
}

fn drop_comments(source: &str, table: &Table) -> String {
    let mut text = source;
    if let Some(p) = table.single("lexical.prologue") {
        let lead = text.len() - text.trim_start().len();
        if !text[..lead].contains('\n') && text[lead..].starts_with(p) {
            text = &text[lead + p.len()..];
        }
    }
    let lines = table.strings("lexical.comment_line");
    let opens = table.strings("lexical.comment_block.open");
    let closes = table.strings("lexical.comment_block.close");
    let quotes = table.letters("lexical.string_quotes");
    let mut kept = String::with_capacity(text.len());
    let mut ahead = text;
    let mut inside: Option<char> = None;
    let mut escaped = false;
    while let Some(c) = ahead.chars().next() {
        let w = c.len_utf8();
        if let Some(opener) = inside {
            kept.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == opener {
                inside = None;
            }
            ahead = &ahead[w..];
        } else if quotes.contains(&c) {
            inside = Some(c);
            kept.push(c);
            ahead = &ahead[w..];
        } else if let Some(which) = opens.iter().position(|o| ahead.starts_with(o.as_str())) {
            let after = &ahead[opens[which].len()..];
            let stop = after.find(closes[which].as_str()).map_or(after.len(), |p| p + closes[which].len());
            kept.extend(after[..stop].chars().filter(|c| *c == '\n'));
            ahead = &after[stop..];
        } else if lines.iter().any(|m| ahead.starts_with(m.as_str())) {
            ahead = ahead.find('\n').map_or("", |p| &ahead[p..]);
        } else {
            kept.push(c);
            ahead = &ahead[w..];
        }
    }
    kept
}

pub fn scan(source: &str, table: &Table) -> Result<Vec<Token>, String> {
    let text = drop_comments(source, table);
    let src: Vec<char> = text.chars().collect();
    let quotes = table.letters("lexical.string_quotes");
    let raw = table.letters("lexical.raw_quotes");
    let escapes = table.letters("lexical.string_escapes");
    let point = table.letter("lexical.number.decimal_point");
    let base = table.letter("lexical.number.base_marker");
    let expo = table.letter("lexical.number.exponent_marker");
    let hex: Option<(char, char)> = table.single("lexical.number.hex_prefix").and_then(|p| {
        let mut it = p.chars();
        Some((it.next()?, it.next()?))
    });
    let prefix = table.letter("identifier.variable_prefix");
    let quote_name = table.letter("lexical.name_quote");
    let unit = table.count("block.indent_size").unwrap_or(4);
    let fold_kw = table.flag("lexical.keywords_case_insensitive");
    let fold_id = table.flag("identifier.case_insensitive");
    let mut tokens: Vec<Token> = Vec::new();
    let tok = |kind: Shape, text: String, row: u32| Token { shape: kind, lexeme: text, span: 0, row: row };
    let (mut pos, mut row, mut at_bol) = (0usize, 1u32, true);
    while pos < src.len() {
        if at_bol {
            at_bol = false;
            let mut width = 0;
            let mut k = pos;
            while k < src.len() && (src[k] == ' ' || src[k] == '\t') {
                width += if src[k] == '\t' { unit } else { 1 };
                k += 1;
            }
            let line_end = src[k..].iter().position(|c| *c == '\n').map_or(src.len(), |p| k + p);
            if src[k..line_end].iter().all(|c| c.is_whitespace()) {
                if line_end < src.len() {
                    row += 1;
                }
                pos = (line_end + 1).min(src.len());
                at_bol = true;
                continue;
            }
            tokens.push(Token { shape: Shape::Lead, lexeme: String::new(), span: width, row: row });
            pos = k;
        }
        let c = src[pos];
        if c == '\n' {
            tokens.push(tok(Shape::LineEnd, "\n".into(), row));
            row += 1;
            pos += 1;
            at_bol = true;
            continue;
        }
        if c.is_whitespace() {
            pos += 1;
            continue;
        }
        if quotes.contains(&c) {
            let is_raw = raw.contains(&c);
            let (mut s, mut k, mut closed) = (String::new(), pos + 1, false);
            while k < src.len() {
                let d = src[k];
                if d == '\\' && k + 1 < src.len() {
                    let e = src[k + 1];
                    if e == '\\' || e == c || (!is_raw && escapes.contains(&e)) {
                        s.push(match e {
                            'n' => '\n',
                            't' => '\t',
                            'r' => '\r',
                            '0' => '\0',
                            o => o,
                        });
                    } else {
                        s.push('\\');
                        s.push(e);
                    }
                    k += 2;
                    continue;
                }
                k += 1;
                if d == c {
                    closed = true;
                    break;
                }
                if d == '\n' {
                    row += 1;
                }
                s.push(d);
            }
            if !closed {
                return Err(format!("Unterminated {} string", c));
            }
            tokens.push(tok(Shape::Quote, s, row));
            pos = k;
            continue;
        }
        if c.is_ascii_digit() {
            let mut k = pos;
            while k < src.len() && src[k].is_ascii_digit() {
                k += 1;
            }
            let at = |k: usize| src.get(k).copied();
            let hexed = hex.map_or(false, |(d, l)| k - pos == 1 && src[pos] == d && at(k) == Some(l) && at(k + 1).map_or(false, |x| x.is_ascii_hexdigit()));
            if hexed {
                k += 1;
                while k < src.len() && src[k].is_ascii_hexdigit() {
                    k += 1;
                }
            } else if base.is_some() && at(k) == base {
                k += 1;
                while let Some(d) = at(k) {
                    let more = at(k + 1).map_or(false, |x| x.is_ascii_alphanumeric());
                    if d.is_ascii_alphanumeric() || ((Some(d) == point || Some(d) == expo) && more) {
                        k += 1;
                    } else {
                        break;
                    }
                }
            } else if point.is_some() && at(k) == point && at(k + 1).map_or(false, |x| x.is_ascii_digit()) {
                k += 1;
                while k < src.len() && src[k].is_ascii_digit() {
                    k += 1;
                }
            }
            tokens.push(tok(Shape::Numeral, src[pos..k].iter().collect(), row));
            pos = k;
            continue;
        }
        if quote_name == Some(c) {
            let mut k = pos + 1;
            let mut s = String::new();
            let mut closed = false;
            while k < src.len() && src[k] != '\n' {
                let d = src[k];
                k += 1;
                if d == c {
                    closed = true;
                    break;
                }
                s.push(d);
            }
            if !closed || !table.name_like(&s) {
                return Err(format!("Expected a name between {} quotes at row {}", c, row));
            }
            tokens.push(tok(Shape::Quoted, s, row));
            pos = k;
            continue;
        }
        let prefixed = prefix == Some(c) && src.get(pos + 1).map_or(false, |n| table.begins_name(*n));
        if table.begins_name(c) || prefixed {
            let mut k = if prefixed { pos + 1 } else { pos };
            while k < src.len() && table.extends_name(src[k]) {
                k += 1;
            }
            let mut s: String = src[pos..k].iter().collect();
            let mut longest = 0;
            for name in table.prims.keys() {
                if name.len() > s.len() && name.starts_with(s.as_str()) {
                    let tail: Vec<char> = name[s.len()..].chars().collect();
                    let same = tail.iter().enumerate().all(|(n, t)| src.get(k + n) == Some(t));
                    let clean = src.get(k + tail.len()).map_or(true, |x| !table.extends_name(*x));
                    if same && clean && tail.len() > longest {
                        longest = tail.len();
                    }
                }
            }
            s.extend(&src[k..k + longest]);
            k += longest;
            let low = s.to_lowercase();
            if fold_id || (fold_kw && table.keywords.contains(&low)) {
                s = low;
            }
            tokens.push(tok(Shape::Bare, s, row));
            pos = k;
            continue;
        }
        let ahead: String = src[pos..src.len().min(pos + 8)].iter().collect();
        let Some(sym) = table.signs.iter().find(|s| ahead.starts_with(s.as_str())).cloned() else {
            return Err(format!("Unexpected character '{}' at row {}", c, row));
        };
        pos += sym.chars().count();
        tokens.push(tok(Shape::Sign, sym, row));
    }
    tokens.push(tok(Shape::Finish, "EOF".into(), row));
    Ok(tokens)
}
