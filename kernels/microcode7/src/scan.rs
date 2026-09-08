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
    // A closing marker at the very end (ext.lexical.epilogue).
    if let Some(cut) = table.strings("ext.lexical.epilogue").iter().find_map(|e| text.trim_end().strip_suffix(e.as_str())) {
        text = cut;
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

/// A source that is text with code in it (ext.lexical.template): what
/// lies between the prologue and the epilogue is code, and everything
/// else is written out as it stands, as though the program said so.
pub fn scan(source: &str, table: &Table) -> Result<Vec<Token>, String> {
    if !table.flag("ext.lexical.template") {
        return scan_code(source, table);
    }
    let opening = table.single("lexical.prologue").ok_or_else(|| "A template needs lexical.prologue".to_string())?;
    let closing = table.single("ext.lexical.epilogue");
    let telling = table
        .prims
        .iter()
        .find(|(_, op)| **op == crate::form::Prim::Tell)
        .map(|(word, _)| word.clone())
        .ok_or_else(|| "A template needs a builtin that writes what it is given".to_string())?;
    let ending = table.single("stmt.terminator").unwrap_or(";").to_string();
    let mut out: Vec<Token> = Vec::new();
    let says = |text: &str, out: &mut Vec<Token>| {
        if text.is_empty() {
            return;
        }
        out.push(Token { shape: Shape::Bare, lexeme: telling.clone(), span: 0, row: 1 });
        out.push(Token { shape: Shape::Quote, lexeme: text.to_string(), span: 0, row: 1 });
        out.push(Token { shape: Shape::Sign, lexeme: ending.clone(), span: 0, row: 1 });
    };
    let mut rest = source;
    while let Some(at) = rest.find(opening) {
        says(&rest[..at], &mut out);
        let after = &rest[at + opening.len()..];
        let (code, tail) = match closing.and_then(|e| after.find(e)) {
            Some(end) => (&after[..end], &after[end + closing.map_or(0, str::len)..]),
            None => (after, ""),
        };
        let mut inside = scan_code(code, table)?;
        inside.pop();
        out.append(&mut inside);
        // Each run of code stands as a statement, however it ended.
        out.push(Token { shape: Shape::Sign, lexeme: ending.clone(), span: 0, row: 1 });
        // One line end straight after the closing marker belongs to it.
        rest = tail.strip_prefix('\n').unwrap_or_else(|| tail.strip_prefix("\r\n").unwrap_or(tail));
    }
    says(rest, &mut out);
    out.push(Token { shape: Shape::Finish, lexeme: "EOF".into(), span: 0, row: 1 });
    Ok(out)
}

fn scan_code(source: &str, table: &Table) -> Result<Vec<Token>, String> {
    let text = drop_comments(source, table);
    let src: Vec<char> = text.chars().collect();
    let quotes = table.letters("lexical.string_quotes");
    let raw = table.letters("lexical.raw_quotes");
    let weaving = table.letters("ext.lexical.interpolating_quotes");
    let escapes = table.letters("lexical.string_escapes");
    let point = table.letter("lexical.number.decimal_point");
    let base = table.letter("lexical.number.base_marker");
    let expo = table.letter("lexical.number.exponent_marker");
    let powers = table.letters("ext.lexical.number.exponent");
    // Each way of writing a number in a base of its own, as the digit
    // and letter that open it and the base its digits are read in.
    let mut in_base: Vec<(char, char, u32)> = Vec::new();
    for (key, radix) in [
        ("lexical.number.hex_prefix", 16u32),
        ("ext.lexical.number.binary_prefix", 2),
        ("ext.lexical.number.octal_prefix", 8),
    ] {
        for p in table.strings(key) {
            let mut it = p.chars();
            if let (Some(d), Some(l)) = (it.next(), it.next()) {
                in_base.push((d, l, radix));
            }
        }
    }
    // Marks written between digits to break them up count for nothing.
    let apart = table.letters("ext.lexical.number.separator");
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
            let woven = weaving.contains(&c);
            // Positions in s that were escaped, so never open a variable.
            let mut plain: Vec<usize> = Vec::new();
            let (mut s, mut k, mut closed) = (String::new(), pos + 1, false);
            while k < src.len() {
                let d = src[k];
                if d == '\\' && k + 1 < src.len() {
                    let e = src[k + 1];
                    if woven && Some(e) == prefix {
                        plain.push(s.chars().count());
                        s.push(e);
                        k += 2;
                        continue;
                    }
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
            if woven {
                weave(&s, &plain, table, row, &mut tokens)?;
            } else {
                tokens.push(tok(Shape::Quote, s, row));
            }
            pos = k;
            continue;
        }
        if c.is_ascii_digit() {
            let mut k = pos;
            while k < src.len() && (src[k].is_ascii_digit() || apart.contains(&src[k])) {
                k += 1;
            }
            let at = |k: usize| src.get(k).copied();
            let opened = in_base
                .iter()
                .find(|(d, l, radix)| k - pos == 1 && src[pos] == *d && at(k) == Some(*l) && at(k + 1).map_or(false, |x| x.is_digit(*radix)))
                .copied();
            if let Some((_, _, radix)) = opened {
                k += 1;
                while k < src.len() && (src[k].is_digit(radix) || apart.contains(&src[k])) {
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
            } else {
                if point.is_some() && at(k) == point && at(k + 1).map_or(false, |x| x.is_ascii_digit()) {
                    k += 1;
                    while k < src.len() && (src[k].is_ascii_digit() || apart.contains(&src[k])) {
                        k += 1;
                    }
                }
                // 1e9, 2.5E-3: a power of ten after the letter.
                let sign_len = usize::from(matches!(at(k + 1), Some('+') | Some('-')));
                if at(k).map_or(false, |c| powers.contains(&c)) && at(k + 1 + sign_len).map_or(false, |x| x.is_ascii_digit()) {
                    k += 1 + sign_len;
                    while k < src.len() && src[k].is_ascii_digit() {
                        k += 1;
                    }
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
        // A sign written before a name and saying nothing: PHP's `\Error`.
        let led = table.letters("ext.lexical.name_lead").contains(&c) && src.get(pos + 1).map_or(false, |n| table.begins_name(*n));
        let prefixed = prefix == Some(c) && src.get(pos + 1).map_or(false, |n| table.begins_name(*n));
        if table.begins_name(c) || prefixed || led {
            let mut k = if prefixed || led { pos + 1 } else { pos };
            while k < src.len() && table.extends_name(src[k]) {
                k += 1;
            }
            let mut s: String = src[if led { pos + 1 } else { pos }..k].iter().collect();
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

/// One piece of an interpolating string: literal text, or code to scan.
enum Piece {
    Text(String),
    Code(String),
}

/// Cut a string with values woven in (ext.lexical.interpolating_quotes)
/// into pieces: `$name`, `$name[i]` with a plain index, and `{$expr}`
/// are code, the rest text. Char positions listed in `plain` are text.
fn pieces(s: &str, plain: &[usize], table: &Table) -> Vec<Piece> {
    let sigil = table.letter("identifier.variable_prefix");
    let cs: Vec<char> = s.chars().collect();
    let variable_at = |at: usize| {
        cs.get(at).copied() == sigil && !plain.contains(&at) && cs.get(at + 1).map_or(false, |n| table.begins_name(*n))
    };
    let mut out = Vec::new();
    let mut text = String::new();
    let mut at = 0;
    while at < cs.len() {
        // {$expr}: up to the brace that balances the opener.
        if cs[at] == '{' && variable_at(at + 1) {
            let mut depth = 0i32;
            let close = cs[at..].iter().position(|c| {
                depth += match c { '{' => 1, '}' => -1, _ => 0 };
                depth == 0
            });
            if let Some(len) = close {
                out.push(Piece::Text(std::mem::take(&mut text)));
                out.push(Piece::Code(cs[at + 1..at + len].iter().collect()));
                at += len + 1;
                continue;
            }
        }
        if variable_at(at) {
            let mut end = at + 1;
            while end < cs.len() && table.extends_name(cs[end]) {
                end += 1;
            }
            if cs.get(end) == Some(&'[') {
                let inside: String = cs[end + 1..].iter().take_while(|c| **c != ']').collect();
                let closed = cs.get(end + 1 + inside.chars().count()) == Some(&']');
                let plain_index = !inside.is_empty() && (inside.chars().all(|c| c.is_ascii_digit()) || inside.starts_with(|c| Some(c) == sigil));
                if closed && plain_index {
                    end += inside.chars().count() + 2;
                }
            }
            out.push(Piece::Text(std::mem::take(&mut text)));
            out.push(Piece::Code(cs[at..end].iter().collect()));
            at = end;
            continue;
        }
        text.push(cs[at]);
        at += 1;
    }
    out.push(Piece::Text(text));
    out
}

/// The tokens of a woven string: a bracketed concatenation starting
/// from the empty string, so the result is text whatever is woven in.
fn weave(s: &str, plain: &[usize], table: &Table, row: u32, tokens: &mut Vec<Token>) -> Result<(), String> {
    let cut = pieces(s, plain, table);
    let sign = |text: &str| Token { shape: Shape::Sign, lexeme: text.to_string(), span: 0, row };
    let quote = |text: String| Token { shape: Shape::Quote, lexeme: text, span: 0, row };
    if !cut.iter().any(|p| matches!(p, Piece::Code(_))) {
        tokens.push(quote(s.to_string()));
        return Ok(());
    }
    let (Some(open), Some(close), Some(join)) = (table.single("syntax.group.open"), table.single("syntax.group.close"), table.single("op.concat")) else {
        return Err("String interpolation needs syntax.group and op.concat".to_string());
    };
    tokens.push(sign(open));
    tokens.push(quote(String::new()));
    for piece in cut {
        match piece {
            Piece::Text(t) if t.is_empty() => {}
            Piece::Text(t) => {
                tokens.push(sign(join));
                tokens.push(quote(t));
            }
            Piece::Code(code) => {
                tokens.push(sign(join));
                // The code woven into a string is code already, so it is
                // read as code and not as text with code in it.
                let inner = scan_code(&code, table)?;
                tokens.extend(inner.into_iter().filter(|t| !matches!(t.shape, Shape::Lead | Shape::Finish)).map(|mut t| {
                    t.row = row;
                    t
                }));
            }
        }
    }
    tokens.push(sign(close));
    Ok(())
}
