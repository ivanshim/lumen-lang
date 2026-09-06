// Text to tokens. Comments and the prologue are removed first, outside
// strings, keeping line ends; then strings, numbers, quoted names, words,
// symbols, line ends and indentation are cut out in one pass.

use crate::spec::Spec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tk {
    Word,
    Name,
    Number,
    Text,
    Sym,
    Eol,
    Indent,
    BlockOpen,
    BlockClose,
    End,
}

#[derive(Debug, Clone)]
pub struct Tok {
    pub kind: Tk,
    pub text: String,
    pub width: usize,
    pub line: u32,
}

fn strip(source: &str, spec: &Spec) -> String {
    let mut text = source;
    if let Some(p) = spec.one("lexical.prologue") {
        let lead = text.len() - text.trim_start().len();
        if !text[..lead].contains('\n') && text[lead..].starts_with(p) {
            text = &text[lead + p.len()..];
        }
    }
    let lines = spec.list("lexical.comment_line");
    let opens = spec.list("lexical.comment_block.open");
    let closes = spec.list("lexical.comment_block.close");
    let quotes = spec.chars("lexical.string_quotes");
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    let mut quote: Option<char> = None;
    let mut esc = false;
    while let Some(c) = rest.chars().next() {
        let n = c.len_utf8();
        if let Some(q) = quote {
            out.push(c);
            if esc {
                esc = false;
            } else if c == '\\' {
                esc = true;
            } else if c == q {
                quote = None;
            }
            rest = &rest[n..];
        } else if quotes.contains(&c) {
            quote = Some(c);
            out.push(c);
            rest = &rest[n..];
        } else if let Some(k) = opens.iter().position(|o| rest.starts_with(o.as_str())) {
            let body = &rest[opens[k].len()..];
            let end = body.find(closes[k].as_str()).map_or(body.len(), |p| p + closes[k].len());
            out.extend(body[..end].chars().filter(|c| *c == '\n'));
            rest = &body[end..];
        } else if lines.iter().any(|m| rest.starts_with(m.as_str())) {
            rest = rest.find('\n').map_or("", |p| &rest[p..]);
        } else {
            out.push(c);
            rest = &rest[n..];
        }
    }
    out
}

pub fn lex(source: &str, spec: &Spec) -> Result<Vec<Tok>, String> {
    let text = strip(source, spec);
    let cs: Vec<char> = text.chars().collect();
    let quotes = spec.chars("lexical.string_quotes");
    let raw = spec.chars("lexical.raw_quotes");
    let escapes = spec.chars("lexical.string_escapes");
    let point = spec.ch("lexical.number.decimal_point");
    let base = spec.ch("lexical.number.base_marker");
    let expo = spec.ch("lexical.number.exponent_marker");
    let hex: Option<(char, char)> = spec.one("lexical.number.hex_prefix").and_then(|p| {
        let mut it = p.chars();
        Some((it.next()?, it.next()?))
    });
    let prefix = spec.ch("identifier.variable_prefix");
    let quote_name = spec.ch("lexical.name_quote");
    let unit = spec.number("block.indent_size").unwrap_or(4);
    let fold_kw = spec.on("lexical.keywords_case_insensitive");
    let fold_id = spec.on("identifier.case_insensitive");
    let mut toks: Vec<Tok> = Vec::new();
    let tok = |kind: Tk, text: String, line: u32| Tok { kind, text, width: 0, line };
    let (mut i, mut line, mut start_of_line) = (0usize, 1u32, true);
    while i < cs.len() {
        if start_of_line {
            start_of_line = false;
            let mut w = 0;
            let mut j = i;
            while j < cs.len() && (cs[j] == ' ' || cs[j] == '\t') {
                w += if cs[j] == '\t' { unit } else { 1 };
                j += 1;
            }
            let eol = cs[j..].iter().position(|c| *c == '\n').map_or(cs.len(), |p| j + p);
            if cs[j..eol].iter().all(|c| c.is_whitespace()) {
                if eol < cs.len() {
                    line += 1;
                }
                i = (eol + 1).min(cs.len());
                start_of_line = true;
                continue;
            }
            toks.push(Tok { kind: Tk::Indent, text: String::new(), width: w, line });
            i = j;
        }
        let c = cs[i];
        if c == '\n' {
            toks.push(tok(Tk::Eol, "\n".into(), line));
            line += 1;
            i += 1;
            start_of_line = true;
            continue;
        }
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if quotes.contains(&c) {
            let is_raw = raw.contains(&c);
            let (mut s, mut j, mut closed) = (String::new(), i + 1, false);
            while j < cs.len() {
                let d = cs[j];
                if d == '\\' && j + 1 < cs.len() {
                    let e = cs[j + 1];
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
                    j += 2;
                    continue;
                }
                j += 1;
                if d == c {
                    closed = true;
                    break;
                }
                if d == '\n' {
                    line += 1;
                }
                s.push(d);
            }
            if !closed {
                return Err(format!("Unterminated {} string", c));
            }
            toks.push(tok(Tk::Text, s, line));
            i = j;
            continue;
        }
        if c.is_ascii_digit() {
            let mut j = i;
            while j < cs.len() && cs[j].is_ascii_digit() {
                j += 1;
            }
            let at = |k: usize| cs.get(k).copied();
            let hexed = hex.map_or(false, |(d, l)| j - i == 1 && cs[i] == d && at(j) == Some(l) && at(j + 1).map_or(false, |x| x.is_ascii_hexdigit()));
            if hexed {
                j += 1;
                while j < cs.len() && cs[j].is_ascii_hexdigit() {
                    j += 1;
                }
            } else if base.is_some() && at(j) == base {
                j += 1;
                while let Some(d) = at(j) {
                    let more = at(j + 1).map_or(false, |x| x.is_ascii_alphanumeric());
                    if d.is_ascii_alphanumeric() || ((Some(d) == point || Some(d) == expo) && more) {
                        j += 1;
                    } else {
                        break;
                    }
                }
            } else if point.is_some() && at(j) == point && at(j + 1).map_or(false, |x| x.is_ascii_digit()) {
                j += 1;
                while j < cs.len() && cs[j].is_ascii_digit() {
                    j += 1;
                }
            }
            toks.push(tok(Tk::Number, cs[i..j].iter().collect(), line));
            i = j;
            continue;
        }
        if quote_name == Some(c) {
            let mut j = i + 1;
            let mut s = String::new();
            let mut closed = false;
            while j < cs.len() && cs[j] != '\n' {
                let d = cs[j];
                j += 1;
                if d == c {
                    closed = true;
                    break;
                }
                s.push(d);
            }
            if !closed || !spec.looks_like_word(&s) {
                return Err(format!("Expected a name between {} quotes at line {}", c, line));
            }
            toks.push(tok(Tk::Name, s, line));
            i = j;
            continue;
        }
        let prefixed = prefix == Some(c) && cs.get(i + 1).map_or(false, |n| spec.starts_word(*n));
        if spec.starts_word(c) || prefixed {
            let mut j = if prefixed { i + 1 } else { i };
            while j < cs.len() && spec.in_word(cs[j]) {
                j += 1;
            }
            let mut s: String = cs[i..j].iter().collect();
            let mut longest = 0;
            for name in spec.builtins.keys() {
                if name.len() > s.len() && name.starts_with(s.as_str()) {
                    let tail: Vec<char> = name[s.len()..].chars().collect();
                    let same = tail.iter().enumerate().all(|(k, t)| cs.get(j + k) == Some(t));
                    let clean = cs.get(j + tail.len()).map_or(true, |x| !spec.in_word(*x));
                    if same && clean && tail.len() > longest {
                        longest = tail.len();
                    }
                }
            }
            s.extend(&cs[j..j + longest]);
            j += longest;
            let low = s.to_lowercase();
            if fold_id || (fold_kw && spec.reserved.contains(&low)) {
                s = low;
            }
            toks.push(tok(Tk::Word, s, line));
            i = j;
            continue;
        }
        let ahead: String = cs[i..cs.len().min(i + 8)].iter().collect();
        let Some(sym) = spec.symbols.iter().find(|s| ahead.starts_with(s.as_str())).cloned() else {
            return Err(format!("Unexpected character '{}' at line {}", c, line));
        };
        i += sym.chars().count();
        toks.push(tok(Tk::Sym, sym, line));
    }
    toks.push(tok(Tk::End, "EOF".into(), line));
    Ok(toks)
}
