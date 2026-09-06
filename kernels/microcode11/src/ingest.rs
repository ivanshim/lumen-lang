// Stage 1, ingest: text to tokens.
//
// The prologue and the comments go first, outside strings and with line
// ends kept. Then the text is cut into strings (unescaped here), numbers,
// quoted names, words, symbols, line ends and the indentation that opens
// each line.

use crate::spec::Spec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Word,
    Name,
    Number,
    Str,
    Symbol,
    Newline,
    Indent,
    Open,
    Close,
    End,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: Kind,
    pub text: String,
    pub width: usize,
    pub line: u32,
}

/// Text with the prologue and every comment removed.
fn cleaned(source: &str, spec: &Spec) -> String {
    let mut text = source;
    if let Some(prologue) = spec.first("lexical.prologue") {
        let lead = text.len() - text.trim_start().len();
        if !text[..lead].contains('\n') && text[lead..].starts_with(prologue) {
            text = &text[lead + prologue.len()..];
        }
    }
    let line_marks = spec.words("lexical.comment_line");
    let block_opens = spec.words("lexical.comment_block.open");
    let block_closes = spec.words("lexical.comment_block.close");
    let quotes = spec.glyphs("lexical.string_quotes");
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    let mut open_quote: Option<char> = None;
    let mut backslash = false;
    while i < text.len() {
        let c = text[i..].chars().next().unwrap();
        if let Some(q) = open_quote {
            out.push(c);
            if backslash {
                backslash = false;
            } else if c == '\\' {
                backslash = true;
            } else if c == q {
                open_quote = None;
            }
            i += c.len_utf8();
        } else if quotes.contains(&c) {
            open_quote = Some(c);
            out.push(c);
            i += c.len_utf8();
        } else if let Some(k) = block_opens.iter().position(|o| text[i..].starts_with(o.as_str())) {
            let after = i + block_opens[k].len();
            let end = text[after..].find(block_closes[k].as_str()).map_or(text.len(), |p| after + p + block_closes[k].len());
            out.extend(text[after..end].chars().filter(|c| *c == '\n'));
            i = end;
        } else if line_marks.iter().any(|m| text[i..].starts_with(m.as_str())) {
            i = text[i..].find('\n').map_or(text.len(), |p| i + p);
        } else {
            out.push(c);
            i += c.len_utf8();
        }
    }
    out
}

pub fn tokens(source: &str, spec: &Spec) -> Result<Vec<Token>, String> {
    let text = cleaned(source, spec);
    let chars: Vec<char> = text.chars().collect();
    let quotes = spec.glyphs("lexical.string_quotes");
    let raw_quotes = spec.glyphs("lexical.raw_quotes");
    let escapes = spec.glyphs("lexical.string_escapes");
    let point = spec.glyph("lexical.number.decimal_point");
    let base_mark = spec.glyph("lexical.number.base_marker");
    let exp_mark = spec.glyph("lexical.number.exponent_marker");
    let hex: Option<(char, char)> = spec.first("lexical.number.hex_prefix").and_then(|p| {
        let mut it = p.chars();
        Some((it.next()?, it.next()?))
    });
    let prefix = spec.glyph("identifier.variable_prefix");
    let name_quote = spec.glyph("lexical.name_quote");
    let indent_unit = spec.count("block.indent_size").unwrap_or(4);
    let fold_keywords = spec.flag("lexical.keywords_case_insensitive");
    let fold_names = spec.flag("identifier.case_insensitive");

    let mut out: Vec<Token> = Vec::new();
    let mut i = 0;
    let mut line: u32 = 1;
    let mut fresh = true;
    let push = |out: &mut Vec<Token>, kind: Kind, text: String, line: u32| out.push(Token { kind, text, width: 0, line });

    while i < chars.len() {
        if fresh {
            fresh = false;
            // Measure the indentation; a blank line yields nothing.
            let mut width = 0;
            let mut j = i;
            while j < chars.len() && (chars[j] == ' ' || chars[j] == '\t') {
                width += if chars[j] == '\t' { indent_unit } else { 1 };
                j += 1;
            }
            let eol = chars[j..].iter().position(|c| *c == '\n').map_or(chars.len(), |p| j + p);
            if chars[j..eol].iter().all(|c| c.is_whitespace()) {
                i = (eol + 1).min(chars.len());
                if eol < chars.len() {
                    line += 1;
                }
                fresh = true;
                continue;
            }
            out.push(Token { kind: Kind::Indent, text: String::new(), width, line });
            i = j;
        }
        let c = chars[i];
        if c == '\n' {
            push(&mut out, Kind::Newline, "\n".to_string(), line);
            line += 1;
            i += 1;
            fresh = true;
            continue;
        }
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        // Strings.
        if quotes.contains(&c) {
            let raw = raw_quotes.contains(&c);
            let mut s = String::new();
            let mut j = i + 1;
            let mut closed = false;
            while j < chars.len() {
                let d = chars[j];
                if d == '\\' && j + 1 < chars.len() {
                    let e = chars[j + 1];
                    let known = e == '\\' || e == c || (!raw && escapes.contains(&e));
                    if known {
                        s.push(match e {
                            'n' => '\n',
                            't' => '\t',
                            'r' => '\r',
                            '0' => '\0',
                            other => other,
                        });
                    } else {
                        s.push('\\');
                        s.push(e);
                    }
                    j += 2;
                    continue;
                }
                if d == c {
                    closed = true;
                    j += 1;
                    break;
                }
                if d == '\n' {
                    line += 1;
                }
                s.push(d);
                j += 1;
            }
            if !closed {
                return Err(format!("Unterminated {} string", c));
            }
            push(&mut out, Kind::Str, s, line);
            i = j;
            continue;
        }
        // Numbers.
        if c.is_ascii_digit() {
            let mut s = String::new();
            let mut j = i;
            while j < chars.len() && chars[j].is_ascii_digit() {
                s.push(chars[j]);
                j += 1;
            }
            let at = |k: usize| chars.get(k).copied();
            let is_hex = hex.map_or(false, |(d, l)| {
                s.len() == 1 && s.starts_with(d) && at(j) == Some(l) && at(j + 1).map_or(false, |c| c.is_ascii_hexdigit())
            });
            if is_hex {
                s.push(chars[j]);
                j += 1;
                while j < chars.len() && chars[j].is_ascii_hexdigit() {
                    s.push(chars[j]);
                    j += 1;
                }
            } else if base_mark.is_some() && at(j) == base_mark {
                s.push(chars[j]);
                j += 1;
                while let Some(d) = at(j) {
                    let more = at(j + 1).map_or(false, |n| n.is_ascii_alphanumeric());
                    if d.is_ascii_alphanumeric() || ((Some(d) == point || Some(d) == exp_mark) && more) {
                        s.push(d);
                        j += 1;
                    } else {
                        break;
                    }
                }
            } else if point.is_some() && at(j) == point && at(j + 1).map_or(false, |c| c.is_ascii_digit()) {
                s.push(chars[j]);
                j += 1;
                while j < chars.len() && chars[j].is_ascii_digit() {
                    s.push(chars[j]);
                    j += 1;
                }
            }
            push(&mut out, Kind::Number, s, line);
            i = j;
            continue;
        }
        // Quoted names.
        if name_quote == Some(c) {
            let mut s = String::new();
            let mut j = i + 1;
            let mut closed = false;
            while j < chars.len() && chars[j] != '\n' {
                let d = chars[j];
                j += 1;
                if d == c {
                    closed = true;
                    break;
                }
                s.push(d);
            }
            if !closed || !s.starts_with(|d| spec.word_start(d)) || !s.chars().all(|d| spec.word_char(d)) {
                return Err(format!("Expected a name between {} quotes at line {}", c, line));
            }
            push(&mut out, Kind::Name, s, line);
            i = j;
            continue;
        }
        // Words, with an optional variable prefix; a builtin name may run on
        // past the word (println!, console.log).
        let prefixed = prefix == Some(c) && chars.get(i + 1).map_or(false, |n| spec.word_start(*n));
        if spec.word_start(c) || prefixed {
            let mut s = String::new();
            let mut j = i;
            if prefixed {
                s.push(c);
                j += 1;
            }
            while j < chars.len() && spec.word_char(chars[j]) {
                s.push(chars[j]);
                j += 1;
            }
            let mut extra = 0;
            for name in spec.natives.keys() {
                if name.len() > s.len() && name.starts_with(s.as_str()) {
                    let tail: Vec<char> = name[s.len()..].chars().collect();
                    let fits = tail.iter().enumerate().all(|(k, t)| chars.get(j + k) == Some(t));
                    let ends = chars.get(j + tail.len()).map_or(true, |c| !spec.word_char(*c));
                    if fits && ends && tail.len() > extra {
                        extra = tail.len();
                    }
                }
            }
            s.extend(chars[j..j + extra].iter());
            j += extra;
            let lower = s.to_lowercase();
            if fold_names || (fold_keywords && spec.reserved.contains(&lower)) {
                s = lower;
            }
            push(&mut out, Kind::Word, s, line);
            i = j;
            continue;
        }
        // Symbols, longest first.
        let ahead: String = chars[i..chars.len().min(i + 8)].iter().collect();
        match spec.symbols.iter().find(|s| ahead.starts_with(s.as_str())) {
            Some(sym) => {
                let sym = sym.clone();
                i += sym.chars().count();
                push(&mut out, Kind::Symbol, sym, line);
            }
            None => return Err(format!("Unexpected character '{}' at line {}", c, line)),
        }
    }
    push(&mut out, Kind::End, "EOF".to_string(), line);
    Ok(out)
}
