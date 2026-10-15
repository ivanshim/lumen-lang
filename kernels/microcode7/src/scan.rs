// Text to tokens. Comments and the prologue are removed first, outside
// strings, keeping line ends; then strings, numbers, quoted names, words,
// symbols, line ends and indentation are cut out in one pass.

use crate::table::Table;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Shape {
    Bare,
    Quoted,
    Numeral,
    Quote,
    ByteQuote,
    Woven,
    WovenEnd,
    Field,
    FieldText,
    Unheld,
    CharacterRow,
    EscapeNotice,
    Sign,
    LineEnd,
    Lead,
    Open,
    Close,
    Finish,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Token {
    pub shape: Shape,
    pub lexeme: String,
    /// The spelling as written, kept only when NFKC folding changed it.
    /// Keyword and reserved-name classification reads this; the name a
    /// Python identifier binds under is `lexeme`.
    pub raw: Option<String>,
    pub span: usize,
    pub row: u32,
    pub utf8_column: usize,
    pub utf8_end: usize,
    pub column: usize,
    pub end_column: usize,
    pub end_row: u32,
}

impl Token {
    /// The spelling the language's own words are matched against: the
    /// text as written, which NFKC folding may have changed.
    pub fn spelling(&self) -> &str {
        match &self.raw { Some(written) => written, None => &self.lexeme }
    }
}

/// Where a marker stands in a piece of text, found however it is
/// written where the table says such markers are known that way. A
/// marker is written in letters that have a case, so lowering them
/// leaves every place in the text where it already was.
fn marker_at(text: &str, marker: &str, folded: bool) -> Option<usize> {
    match folded {
        true => text.to_ascii_lowercase().find(&marker.to_ascii_lowercase()),
        false => text.find(marker),
    }
}

fn drop_comments(source: &str, table: &Table) -> String {
    let mut text = source;
    let any_case = table.scanner.flag_ext_lexical_prologue_folded;
    if let Some(p) = table.scanner.single_lexical_prologue.as_deref() {
        let lead = text.len() - text.trim_start().len();
        let opens = match any_case {
            true => text[lead..].to_ascii_lowercase().starts_with(&p.to_ascii_lowercase()),
            false => text[lead..].starts_with(p),
        };
        // Once imports can be read, their bindings belong to the run.
        let read_import = p.split_whitespace().next().map_or(false, |head| table.spells("ext.stmt.import", head));
        if !read_import && !text[..lead].contains('\n') && opens {
            text = &text[lead + p.len()..];
        }
    }
    // A closing marker at the very end (ext.lexical.epilogue).
    if let Some(cut) = table.scanner.strings_ext_lexical_epilogue.as_slice().iter().find_map(|e| text.trim_end().strip_suffix(e.as_str())) {
        text = cut;
    }
    if !table.scanner.strings_ext_lexical_string_long.as_slice().is_empty() { return text.to_string(); }
    let lines = table.scanner.strings_lexical_comment_line.as_slice();
    let opens = table.scanner.strings_lexical_comment_block_open.as_slice();
    let closes = table.scanner.strings_lexical_comment_block_close.as_slice();
    let quotes = table.letters("lexical.string_quotes");
    let folding = table.scanner.single_ext_lexical_heredoc.as_deref();
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
        } else if folding.map_or(false, |mark| ahead.starts_with(mark)) {
            // A folded string spells what it spells, comment marks and
            // quote marks alike, so the whole of it is carried over
            // untouched and nothing in it is taken for a comment. One
            // never closed goes over whole as well, and the scanner is
            // left to say so.
            let reach = folded(ahead, table).map_or(ahead.len(), |(_, _, far)| far);
            kept.push_str(&ahead[..reach]);
            ahead = &ahead[reach..];
        } else if let Some(delimiter) = table.scanner.strings_ext_lexical_string_long.as_slice().iter().find(|d| ahead.starts_with(d.as_str())) {
            let mut end = delimiter.len();
            let mut shield = false;
            for (offset, ch) in ahead[end..].char_indices() {
                let at = delimiter.len() + offset;
                if !shield && ahead[at..].starts_with(delimiter.as_str()) {
                    end = at + delimiter.len();
                    break;
                }
                end = at + ch.len_utf8();
                shield = !shield && ch == '\\';
            }
            kept.push_str(&ahead[..end]);
            ahead = &ahead[end..];
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
            for ending in table.scanner.strings_ext_lexical_line_continuation.as_slice() {
                if kept.ends_with(ending.as_str()) {
                    kept.push(' ');
                    break;
                }
            }
            ahead = ahead.find('\n').map_or("", |p| &ahead[p..]);
        } else {
            kept.push(c);
            ahead = &ahead[w..];
        }
    }
    kept
}

/// What a backslash stands for in a run of text: the letters that name
/// characters of their own, the mark that ends the run, which may always
/// be written escaped, the letter and brackets that name a character by
/// its number with what the language says of a number amiss, and the
/// sigil, where one written escaped is a sigil and opens no name.
struct Backslash<'a> {
    letters: &'a [char],
    ends: Option<char>,
    numbered: Option<char>,
    unbracketed: Option<char>,
    eights: bool,
    in_bytes: bool,
    open: Option<char>,
    shut: Option<char>,
    amiss: &'a str,
    beyond: &'a str,
    sigil: Option<char>,
}

impl Backslash<'_> {
    /// What the escape written at `at` stands for, put into `out`, and
    /// where the reading goes on from. A letter the language never names
    /// keeps its backslash, since text it has no word for is text as it
    /// was written. Places whose character came escaped are noted in
    /// `plain`: such a one is text and opens nothing.
    fn reads(&self, src: &[char], at: usize, out: &mut String, plain: &mut Vec<usize>) -> Result<usize, String> {
        let e = src[at + 1];
        if self.sigil == Some(e) {
            plain.push(out.chars().count());
            out.push(e);
            return Ok(at + 2);
        }
        // The letter followed by its opening bracket names a character
        // by its number: the digits between the brackets are read in
        // sixteens and the character of that number stands in their
        // place. A number naming no character of its own — half of a
        // pair standing for one character between them — is left as it
        // was written, text made of characters having no room for it.
        if self.numbered == Some(e) && src.get(at + 2).copied() == self.open {
            let (mut j, mut digits, mut closed) = (at + 3, String::new(), false);
            while j < src.len() {
                let d = src[j];
                j += 1;
                if Some(d) == self.shut {
                    closed = true;
                    break;
                }
                if !d.is_ascii_hexdigit() {
                    return Err(self.amiss.to_string());
                }
                digits.push(d);
            }
            if !closed || digits.is_empty() {
                return Err(self.amiss.to_string());
            }
            let bare = digits.trim_start_matches('0');
            let number = match bare.is_empty() {
                true => 0,
                false => u32::from_str_radix(bare, 16).map_err(|_| self.beyond.to_string())?,
            };
            if number > 0x10FFFF {
                return Err(self.beyond.to_string());
            }
            // Text kept as bytes takes the number spelled out in the
            // bytes that spell it, whatever it names — half of a pair
            // standing for one character between them included, which
            // no letter of its own answers to.
            if self.in_bytes {
                for byte in numbered_bytes(number) {
                    plain.push(out.chars().count());
                    out.push(char::from(byte));
                }
                return Ok(j);
            }
            match char::from_u32(number) {
                Some(made) => {
                    plain.push(out.chars().count());
                    out.push(made);
                }
                None => {
                    out.push('\\');
                    out.push(e);
                    out.extend(src[at + 2..j].iter());
                }
            }
            return Ok(j);
        }
        // A character named by figures in eights, three of them at
        // most, counting from the figure the backslash is followed by.
        // What is asked for beyond the widest character of one byte is
        // taken by its low eight bits, the way the reference takes it.
        if self.eights && e.is_digit(8) {
            let (mut number, mut j) = (e.to_digit(8).expect("a figure in eights"), at + 2);
            while j < src.len() && j < at + 4 {
                let Some(d) = src[j].to_digit(8) else { break };
                number = number * 8 + d;
                j += 1;
            }
            plain.push(out.chars().count());
            out.push(char::from_u32(number & 0xFF).expect("a character of one byte"));
            return Ok(j);
        }
        // The same by number written bare, no brackets about it: one
        // figure in sixteens or two. Where no figure follows, the
        // letter names nothing and is kept the way it was written.
        if self.unbracketed == Some(e) {
            let (mut number, mut j) = (0u32, at + 2);
            while j < src.len() && j < at + 4 && src[j].is_ascii_hexdigit() {
                number = number * 16 + src[j].to_digit(16).expect("a figure in sixteens");
                j += 1;
            }
            match char::from_u32(number).filter(|_| j > at + 2) {
                Some(made) => {
                    plain.push(out.chars().count());
                    out.push(made);
                }
                None => {
                    out.push('\\');
                    out.push(e);
                }
            }
            return Ok(j);
        }
        if e == '\\' || Some(e) == self.ends || self.letters.contains(&e) {
            out.push(match e {
                'n' => '\n',
                't' => '\t',
                'r' => '\r',
                '0' => '\0',
                'a' => char::from(7),
                'b' => char::from(8),
                'f' => char::from(12),
                'v' => char::from(11),
                o => o,
            });
        } else {
            out.push('\\');
            out.push(e);
        }
        Ok(at + 2)
    }
}

/// A line of a folded string with the closing label's indentation off
/// its front. A line written in less far than the label gives up the
/// indentation it has and no more.
fn shorn(line: &str, indent: usize) -> &str {
    let wide = line.len() - line.trim_start_matches([' ', '\t']).len();
    &line[wide.min(indent)..]
}

/// A string folded over lines, opened where this text begins
/// (ext.lexical.heredoc): the mark, a label bare or in quotes, a line
/// end, then lines to the one the label stands at the head of again.
/// Answers what the body says, with the closing label's indentation
/// taken off every line of it, whether the label was written in raw
/// quotes, which asks for a body that spells only itself, and how many
/// bytes of the text the whole of it takes. Nothing where no such
/// string is opened here, or where no line closes the one that is.
fn folded(text: &str, table: &Table) -> Option<(String, bool, usize)> {
    let opened = text.strip_prefix(table.scanner.single_ext_lexical_heredoc.as_deref()?)?.trim_start_matches([' ', '\t']);
    let quoted = table.letters("lexical.string_quotes").into_iter().find(|q| opened.starts_with(*q));
    let named = &opened[quoted.map_or(0, char::len_utf8)..];
    let mut wide = named.len();
    for (n, c) in named.char_indices() {
        if !(if n == 0 { table.begins_name(c) } else { table.extends_name(c) }) {
            wide = n;
            break;
        }
    }
    let (label, mut after) = named.split_at(wide);
    if label.is_empty() {
        return None;
    }
    if let Some(q) = quoted {
        after = after.strip_prefix(q)?;
    }
    after = after.trim_start_matches([' ', '\t']);
    let raw = quoted.map_or(false, |q| table.letters("lexical.raw_quotes").contains(&q));
    let mut rest = after.strip_prefix("\r\n").or_else(|| after.strip_prefix('\n'))?;
    let mut lines: Vec<&str> = Vec::new();
    loop {
        let line = rest.split('\n').next().unwrap_or("");
        let bare = line.trim_start_matches([' ', '\t']);
        // The label closes the body where nothing goes on from it: a
        // longer word merely begun with it is a word of the body.
        let closes = bare.strip_prefix(label).map_or(false, |on| on.chars().next().map_or(true, |c| !table.extends_name(c)));
        if closes {
            let indent = line.len() - bare.len();
            let mut said: Vec<&str> = lines.iter().map(|l| shorn(l, indent)).collect();
            // The last line of the body is the one the closing label's
            // line end belongs to, a carriage return leading it and all.
            if let Some(last) = said.last_mut() {
                *last = last.strip_suffix('\r').unwrap_or(last);
            }
            return Some((said.join("\n"), raw, text.len() - rest.len() + indent + label.len()));
        }
        // The line end before the closing label is the label's, not the
        // body's; a body whose last line has none closes nowhere.
        if line.len() == rest.len() {
            return None;
        }
        lines.push(line);
        rest = &rest[line.len() + 1..];
    }
}

/// A source that is text with code in it (ext.lexical.template): what
/// lies between the prologue and the epilogue is code, and everything
/// else is written out as it stands, as though the program said so.
pub fn scan(source: &str, table: &Table) -> Result<Vec<Token>, String> {
    scan_at(source, table).map_err(|(said, _)| said)
}

/// A character's number laid out in the bytes that spell it. One below
/// a hundred and twenty-eight is a byte on its own; above that the
/// number is cut into six-bit pieces, the first byte saying by its
/// leading ones how many pieces there are and each of the rest
/// carrying one. Nothing here refuses a number for what it names, so
/// half of a pair is spelled as readily as anything else.

fn numbered_bytes(number: u32) -> Vec<u8> {
    let pieces = match number {
        n if n < 0x80 => return vec![n as u8],
        n if n < 0x800 => 2,
        n if n < 0x10000 => 3,
        _ => 4,
    };
    let lead = [0u8, 0, 0xC0, 0xE0, 0xF0][pieces];
    let mut out = vec![lead | (number >> (6 * (pieces - 1))) as u8];
    for left in (0..pieces - 1).rev() {
        out.push(0x80 | ((number >> (6 * left)) & 0x3F) as u8);
    }
    out
}

/// The place the closing marker holds when nothing is spelling it
/// out. Quoted text, a string laid over lines under a label, and a
/// comment written between an opening and a closing word all spell out
/// whatever stands in them, marker and all, so the search steps over
/// each of those whole rather than reading within it.
///
/// A comment running to the end of its line is the one that does not
/// shield: the reference lets a marker written in such a comment close
/// the run, and only the end of the line rescues what comes after. So
/// such a comment is searched as far as its line reaches and no
/// further.
fn code_gives_out(after: &str, closing: &str, table: &Table) -> Option<usize> {
    let lines = table.scanner.strings_lexical_comment_line.as_slice();
    let opens = table.scanner.strings_lexical_comment_block_open.as_slice();
    let shuts = table.scanner.strings_lexical_comment_block_close.as_slice();
    let quotes = table.letters("lexical.string_quotes");
    let mut here = 0;
    while here < after.len() {
        let ahead = &after[here..];
        if ahead.starts_with(closing) {
            return Some(here);
        }
        if let Some((_, _, wide)) = folded(ahead, table) {
            here += wide;
            continue;
        }
        if let Some(n) = opens.iter().position(|o| ahead.starts_with(o.as_str())) {
            let body = &ahead[opens[n].len()..];
            let shut = shuts.get(n).map_or("", String::as_str);
            here += opens[n].len() + body.find(shut).map_or(body.len(), |p| p + shut.len());
            continue;
        }
        if lines.iter().any(|m| ahead.starts_with(m.as_str())) {
            let reach = ahead.find('\n').map_or(ahead.len(), |p| p + 1);
            if let Some(p) = ahead[..reach].find(closing) {
                return Some(here + p);
            }
            here += reach;
            continue;
        }
        let c = ahead.chars().next().expect("a character");
        here += c.len_utf8();
        if !quotes.contains(&c) {
            continue;
        }
        // Within quotes, on to the mark that shuts them, counting a
        // backslash as taking the character behind it out of the
        // reckoning. Quotes never shut reach to the end of the text.
        let mut shielded = false;
        for (n, d) in after[here..].char_indices() {
            if shielded {
                shielded = false;
                continue;
            }
            if d == '\\' {
                shielded = true;
                continue;
            }
            if d == c {
                here += n + d.len_utf8();
                break;
            }
        }
    }
    None
}

/// The same, saying besides which row the reading stopped on.
pub fn scan_at(source: &str, table: &Table) -> Result<Vec<Token>, (String, u32)> {
    let mut read = scan_notices(source, table)?;
    read.retain(|item| item.shape != Shape::EscapeNotice);
    Ok(read)
}

/// The base a number was read in when a keyword stands written straight
/// against it, as the reference warns, else None.
fn invalid_literal_base(run: &str, table: &Table) -> Option<&'static str> {
    if check_numeral(run, table).is_ok() { return None; }
    for word in ["else", "not", "and", "or", "in", "if", "is", "for"] {
        if let Some(prefix) = run.strip_suffix(word) {
            if !prefix.is_empty() && check_numeral(prefix, table).is_ok()
                && !(prefix == "0" && word == "or")
                && !((prefix.starts_with("0x") || prefix.starts_with("0X")) && matches!(word, "and" | "else")) {
                return Some(if prefix.starts_with("0b") || prefix.starts_with("0B") { "binary" }
                    else if prefix.starts_with("0o") || prefix.starts_with("0O") { "octal" }
                    else if prefix.starts_with("0x") || prefix.starts_with("0X") { "hexadecimal" }
                    else if prefix.ends_with(table.letters("ext.lexical.number.imaginary").as_slice()) { "imaginary" }
                    else { "decimal" });
            }
        }
    }
    None
}

/// Read the numeric candidate once for tokenization and warning collection.
fn numeral_end(src: &[char], pos: usize, table: &Table) -> usize {
    let point = table.letter("lexical.number.decimal_point");
    let strict = table.flag("ext.lexical.number.separator.strict");
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
    let mut k = pos;
    while k < src.len() && (src[k].is_ascii_digit() || apart.contains(&src[k])) {
        k += 1;
    }
    let at = |k: usize| src.get(k).copied();
    let opened = in_base
        .iter()
        .find(|(d, l, radix)| {
            let first = k + 1 + usize::from(table.scanner.flag_ext_lexical_number_separator_after_prefix
                && at(k + 1).map_or(false, |x| apart.contains(&x)));
            k - pos == 1 && src[pos] == *d && at(k) == Some(*l) && (strict || at(first).map_or(false, |x| x.is_digit(*radix)))
        })
        .copied();
    if let Some((_, _, radix)) = opened {
        k += 1;
        while k < src.len() && (src[k].is_digit(radix) || apart.contains(&src[k]) || (strict && table.extends_name(src[k]))) {
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
        if point.is_some() && at(k) == point && (table.flag("ext.lexical.number.point.open") || table.flag("ext.lexical.number.point.bare") || table.flag("ext.lexical.number.point_edge") || table.flag("ext.lexical.number.point_open") || at(k + 1).map_or(false, |x| x.is_ascii_digit())) {
            k += 1;
            while k < src.len() && (src[k].is_ascii_digit() || apart.contains(&src[k])) {
                k += 1;
            }
        }
        // 1e9, 2.5E-3: a power of ten after the letter.
        let sign_len = usize::from(matches!(at(k + 1), Some('+') | Some('-')));
        if at(k).map_or(false, |c| powers.contains(&c)) && (strict || at(k + 1 + sign_len).map_or(false, |x| x.is_ascii_digit())) {
            k += 1 + sign_len;
            while k < src.len() && (src[k].is_ascii_digit() || apart.contains(&src[k])) {
                k += 1;
            }
        }
    }
    if at(k).map_or(false, |x| table.letters("ext.lexical.number.imaginary").contains(&x)) {
        k += 1;
    }
    if strict {
        while at(k).map_or(false, |x| table.extends_name(x)) { k += 1; }
    }
    k
}

pub fn escape_notices(source: &str, table: &Table) -> Vec<(String, u32)> {
    if !table.scanner.has_any_ext_lexical_escape_warning { return vec![]; }
    // A reading that stops short on a fault later in the text still
    // says what it noticed about a string already read, so the notices
    // are gathered by walking the strings in turn, keeping only those
    // whose string finished rather than faulted short.
    let input = if table.syntax_exceptions && source.contains('\r') {
        std::borrow::Cow::Owned(source.replace("\r\n", "\n").replace('\r', "\n"))
    } else { std::borrow::Cow::Borrowed(source) };
    let src: Vec<char> = input.chars().collect();
    let mut notices: Vec<(String, u32)> = Vec::new();
    let (mut pos, mut row) = (0usize, 1u32);
    while pos < src.len() {
        if table.scanner.strings_lexical_comment_line.as_slice().iter().any(|m| src[pos..].starts_with(&m.chars().collect::<Vec<_>>())) {
            while pos < src.len() && src[pos] != '\n' { pos += 1; }
            continue;
        }
        if src[pos] == '\n' { row += 1; pos += 1; continue; }
        if let Some((body, end, raw, fields)) = quoted_start(&src, pos, table) {
            let mut quote = Quotation { gate: None, source: &src, next: pos, row, table, made: Vec::new(), substitutes: std::collections::BTreeMap::new(), pending_literal: false };
            if quote.literal(body, &end, raw, fields, 0).is_ok() {
                for token in &quote.made {
                    if token.shape == Shape::EscapeNotice { notices.push((token.lexeme.clone(), token.row)); }
                }
            }
            pos = quote.next;
            row = quote.row;
            continue;
        }
        // Names and symbols must be consumed whole before the next number.
        let opens_a_number = src[pos].is_ascii_digit()
            || ((table.flag("ext.lexical.number.point.open") || table.flag("ext.lexical.number.point.bare") || table.flag("ext.lexical.number.point_edge") || table.flag("ext.lexical.number.point_open"))
                && Some(src[pos]) == table.letter("lexical.number.decimal_point")
                && src.get(pos + 1).map_or(false, char::is_ascii_digit));
        if opens_a_number {
            let end = numeral_end(&src, pos, table);
            let run: String = src[pos..end].iter().collect();
            if let Some(base) = invalid_literal_base(&run, table) {
                notices.push((format!("invalid {base} literal"), row));
            }
            pos = end;
            continue;
        }
        if table.begins_name(src[pos]) {
            pos += 1;
            while pos < src.len() && table.extends_name(src[pos]) { pos += 1; }
            continue;
        }
        let ahead: String = src[pos..src.len().min(pos + 8)].iter().collect();
        pos += table.signs.iter().find(|sign| ahead.starts_with(sign.as_str()))
            .map_or(1, |sign| sign.chars().count());
    }
    notices
}

fn scan_notices(source: &str, table: &Table) -> Result<Vec<Token>, (String, u32)> {
    if !table.scanner.flag_ext_lexical_template {
        return scan_code_marking(source, table, 1);
    }
    let opening = table.scanner.single_lexical_prologue.as_deref().ok_or_else(|| ("A template needs lexical.prologue".to_string(), 0))?;
    let closing = table.scanner.single_ext_lexical_epilogue.as_deref();
    let telling = table
        .prims
        .iter()
        .find(|(_, op)| **op == crate::form::Prim::Tell)
        .map(|(word, _)| word.clone())
        .ok_or_else(|| ("A template needs a builtin that writes what it is given".to_string(), 0))?;
    let ending = table.scanner.single_stmt_terminator.as_deref().unwrap_or(";").to_string();
    let mut out: Vec<Token> = Vec::new();
    let says = |text: &str, row: u32, out: &mut Vec<Token>| {
        if text.is_empty() {
            return;
        }
        out.push(Token { utf8_column: 0, utf8_end: 0, end_row: 0, end_column: 0, column: 1, shape: Shape::Bare, lexeme: telling.clone(), raw: None, span: 0, row });
        out.push(Token { utf8_column: 0, utf8_end: 0, end_row: 0, end_column: 0, column: 1, shape: Shape::Quote, lexeme: text.to_string(), raw: None, span: 0, row });
        out.push(Token { utf8_column: 0, utf8_end: 0, end_row: 0, end_column: 0, column: 1, shape: Shape::Sign, lexeme: ending.clone(), raw: None, span: 0, row });
    };
    let mut rest = source;
    // The rows of the page are counted through the weave, so that what
    // a run of code says of itself names the page's own lines.
    let mut row: u32 = 1;
    let briefly = table.scanner.single_ext_lexical_prologue_echo.as_deref();
    let folded = table.scanner.flag_ext_lexical_prologue_folded;
    // A shorter marker still, which the run may or may not be given.
    let shortly = table.scanner.single_ext_lexical_prologue_brief.as_deref();
    loop {
        // A run of code may be opened by either marker, whichever comes
        // first. The brief one asks for what the run comes to be
        // written out, so the word that writes stands before it.
        // The run of code is opened by whichever marker comes soonest.
        // Two of them may stand in one place, one being the opening of
        // the other; there the longer is meant, since the shorter says
        // nothing the longer does not say more exactly.
        let mut soonest: Option<(usize, &str, bool)> = None;
        for (mark, writes) in [(Some(opening), false), (briefly, true), (shortly, false)] {
            let Some(mark) = mark else { continue };
            let Some(at) = marker_at(rest, mark, folded) else { continue };
            let takes = soonest.map_or(true, |(was, seen, _): (usize, &str, bool)| {
                at < was || (at == was && mark.len() > seen.len())
            });
            if takes {
                soonest = Some((at, mark, writes));
            }
        }
        let Some((at, mark, writes)) = soonest else { break };
        says(&rest[..at], row, &mut out);
        row += rest[..at].matches('\n').count() as u32;
        let after = &rest[at + mark.len()..];
        let (code, tail) = match closing.and_then(|e| code_gives_out(after, e, table)) {
            Some(end) => (&after[..end], &after[end + closing.map_or(0, str::len)..]),
            None => (after, ""),
        };
        let mut inside = scan_code_marking(code, table, row)?;
        inside.pop();
        let ended = inside.last().map_or(row, |t| t.row);
        if writes {
            out.push(Token { utf8_column: 0, utf8_end: 0, end_row: 0, end_column: 0, column: 1, shape: Shape::Bare, lexeme: telling.clone(), raw: None, span: 0, row });
        }
        out.append(&mut inside);
        // Each run of code stands as a statement, however it ended.
        out.push(Token { utf8_column: 0, utf8_end: 0, end_row: 0, end_column: 0, column: 1, shape: Shape::Sign, lexeme: ending.clone(), raw: None, span: 0, row: ended });
        row += code.matches('\n').count() as u32;
        // One line end straight after the closing marker belongs to it.
        let shorter = tail.strip_prefix('\n').unwrap_or_else(|| tail.strip_prefix("\r\n").unwrap_or(tail));
        if shorter.len() != tail.len() {
            row += 1;
        }
        rest = shorter;
    }
    says(rest, row, &mut out);
    out.push(Token { utf8_column: 0, utf8_end: 0, end_row: 0, end_column: 0, column: 1, shape: Shape::Finish, lexeme: "EOF".into(), raw: None, span: 0, row });
    Ok(out)
}

fn scan_code(source: &str, table: &Table) -> Result<Vec<Token>, String> {
    let mut ended = (1, 1);
    scan_code_from(source, table, 1, &mut ended, None, &mut false)
}

/// The same, begun at a given row, so that a run of code woven into a
/// page names the rows of the page and not its own.
fn scan_code_marking(source: &str, table: &Table, first: u32) -> Result<Vec<Token>, (String, u32)> {
    let mut ended = (first, 1);
    scan_code_from(source, table, first, &mut ended, None, &mut false).map_err(|said| (said, ended.0))
}

/// The marks and letters opening a string, found before names are cut.
fn quoted_start(src: &[char], offset: usize, table: &Table) -> Option<(usize, Vec<char>, bool, bool)> {
    if table.ascii_quotation.get(*src.get(offset)? as usize) == Some(&0) { return None; }
    let quotes = &table.quoted_letters;
    let mut flags = 0u8;
    let mut begin = offset;
    while !quotes.contains(src.get(begin)?) {
        if begin - offset >= 2 { return None; }
        let kind = table.quoted_prefixes.iter().find(|(letter, _)| *letter == src[begin])?.1.min(3);
        let bit = 1 << kind;
        if flags & bit != 0 { return None; }
        flags |= bit;
        begin += 1;
    }
    if begin - offset == 2 && flags != 3 && flags != 9 { return None; }
    let ending = table.quoted_terminators.iter().filter(|s| src[begin..].starts_with(s))
        .max_by_key(|s| s.len()).cloned().unwrap_or_else(|| vec![src[begin]]);
    Some((begin + ending.len(), ending, flags & 1 != 0, flags & 8 != 0))
}

/// What the language says of a run of letters before a quote that
/// spells no valid string prefix (`quoted_start` has already said
/// no): nothing where the run is not made only of prefix letters, or
/// no quote follows it, since that is an ordinary word and not an
/// attempted prefix at all. A repeated letter of the same kind
/// (`bb''`) names no two kinds to blame, so it reads as the
/// language's plain word for text it cannot read; two differing
/// kinds that do not belong together (raw and plain, byte and plain,
/// byte and format, or format and template) name the conflicting
/// pair in Python's prefix order.
fn prefix_conflict(src: &[char], offset: usize, table: &Table) -> Option<String> {
    if table.ascii_quotation.get(*src.get(offset)? as usize) == Some(&0) { return None; }
    let mut seen = 0u8;
    let mut duplicate = false;
    let mut at = offset;
    let quotes = &table.quoted_letters;
    while !quotes.contains(src.get(at)?) {
        let family = table.quoted_prefixes.iter().find(|(letter, _)| *letter == src[at])?.1;
        let bit = 1u8 << family;
        duplicate |= seen & bit != 0;
        seen |= bit;
        at += 1;
        if at - offset > 5 { return None; }
    }
    let width = at - offset;
    if width == 0 { return None; }
    let has = |bit| seen & (1u8 << bit) != 0u8;
    let permitted = width == 1 || width == 2 && has(0) && (has(1) || has(3) || has(4));
    if permitted && !duplicate { return None; }
    let generic = || table.scanner.single_ext_builtin_source_syntax.as_deref().unwrap_or_default().to_owned();
    if duplicate { return Some(generic()); }
    let first = if has(2) { 'u' } else if has(1) { 'b' } else { 'f' };
    let second = if has(2) {
        if has(1) { 'b' } else if has(0) { 'r' } else if has(3) { 'f' } else { 't' }
    } else if has(1) && has(3) { 'f' } else { 't' };
    Some(match table.scanner.strings_ext_lexical_string_prefix_incompatible.as_slice() {
        [before, between, after] => format!("{before}{first}{between}{second}{after}"),
        _ => generic(),
    })
}

type NameGate<'a> = std::rc::Rc<std::cell::RefCell<dyn FnMut() -> Result<(), String> + 'a>>;

struct Quotation<'s, 'a> {
    gate: Option<NameGate<'a>>,
    source: &'s [char],
    next: usize,
    table: &'a Table,
    row: u32,
    made: Vec<Token>,
    substitutes: std::collections::BTreeMap<usize, u32>,
    pending_literal: bool,
}

/// How many fields deep a string may nest and still be named
/// unterminated by its own word rather than the language's plain one:
/// a string nested a level or two inside a field, such as a plain
/// string quoted inside an f-string's expression, is still named this
/// way by the reference. Past shallow nesting, the reference
/// tokenizes and parses together and can meet a syntax fault inside
/// the nesting before ever meeting the end of the text (see
/// test_syntax_error_in_nested_fstring, which nests an already-invalid
/// expression, `1 1`, some 199 fields deep and is told only `invalid
/// syntax`), a pass this reader does not make, so past a depth no
/// ordinary program reaches it keeps its more generic word instead.
const NESTED_UNTERMINATED_DEPTH: u32 = 8;

impl Quotation<'_, '_> {
    fn here(&self) -> Option<char> { self.source.get(self.next).copied() }
    fn forward(&mut self, count: usize) {
        let stop = (self.next + count).min(self.source.len());
        self.row += self.source[self.next..stop].iter().filter(|&&c| c == '\n').count() as u32;
        self.next = stop;
    }
    fn bad(&self) -> String {
        self.table.scanner.single_ext_lexical_string_amiss.as_deref().unwrap_or("Invalid string literal").to_owned()
    }
    fn token(&mut self, kind: Shape, text: String) {
        self.made.push(Token { utf8_column: 0, utf8_end: 0, end_row: 0, end_column: 0, column: 1, row: self.row, shape: kind, lexeme: text, raw: None, span: 0 });
    }
    fn flush(&mut self, text: &mut String, missing: &mut bool) {
        let kind = if *missing { Shape::Unheld } else if self.substitutes.is_empty() { Shape::Quote } else { Shape::CharacterRow };
        let value = if *missing {
            self.table.scanner.single_ext_lexical_escape_unavailable.as_deref().unwrap_or("Unicode escape cannot be represented").to_owned()
        } else if kind == Shape::CharacterRow {
            text.char_indices().map(|(position, ch)| self.substitutes.get(&position).copied().unwrap_or(ch as u32).to_string()).collect::<Vec<_>>().join(" ")
        } else { std::mem::take(text) };
        self.substitutes.clear();
        self.token(kind, value);
        text.clear();
        *missing = false;
    }
    /// Named unterminated only at `depth` zero: a string nested inside
    /// a field is one more piece of an outer string already read this
    /// way, and the reference, tokenizing and parsing together, can
    /// meet a syntax fault inside that nesting before ever meeting the
    /// end of the text, so it does not keep this word for text so
    /// deeply nested that it is likelier to hold some other fault.
    fn literal(&mut self, body: usize, end: &[char], raw: bool, fields: bool, depth: u32) -> Result<(), String> {
        if fields && depth >= 150 { return Err(self.table.scanner.single_ext_builtin_source_syntax.as_deref().unwrap_or("Invalid string literal").to_owned()); }
        let bytes = self.source[self.next..body - end.len()].iter().any(|c|
            self.table.spells("ext.lexical.string.prefix.bytes", &c.to_string()));
        let template = if fields {
            self.source[self.next..body].iter().take_while(|c| c.is_ascii_alphabetic())
                .find(|c| self.table.spells("ext.lexical.string.prefix.template", &c.to_string())).copied()
        } else { None };
        self.forward(body - self.next);
        if fields { self.token(Shape::Woven, template.map(|c| c.to_string()).unwrap_or_default()); }
        let mut noticed = false;
        let mut saved = String::new();
        let mut missing = false;
        let mut part_start = body;
        while !self.source[self.next..].starts_with(end) {
            let ch = self.here().ok_or_else(|| {
                self.pending_literal = fields || depth > 0;
                let key = match (end.len() > 1, fields, template.is_some()) {
                    (false, false, _) => "ext.lexical.string.unterminated",
                    (true, false, _) => "ext.lexical.string.unterminated.triple",
                    (false, true, false) => "ext.lexical.string.prefix.format.unterminated",
                    (true, true, false) => "ext.lexical.string.prefix.format.unterminated.triple",
                    (false, true, true) => "ext.lexical.string.prefix.template.unterminated",
                    (true, true, true) => "ext.lexical.string.prefix.template.unterminated.triple",
                };
                match self.table.single(key) {
                    Some(said) if depth < NESTED_UNTERMINATED_DEPTH => said.to_owned(),
                    None if end.len() > 1 && !fields => format!("Unterminated {} string", end[0]),
                    _ => self.bad(),
                }
            })?;
            if bytes && (!ch.is_ascii() || ch == '\\' && self.source.get(self.next + 1).map_or(false, |c| !c.is_ascii())) {
                return Err(self.table.scanner.single_ext_lexical_string_bytes_ascii.as_deref().unwrap_or("").to_owned());
            }
            match ch {
                '\n' if end.len() == 1 => {
                    let complaint = if !fields && depth == 0 && self.table.syntax_exceptions {
                        self.table.scanner.single_ext_lexical_string_unterminated.as_deref().map(str::to_owned)
                    } else { None };
                    return Err(complaint.unwrap_or_else(|| self.bad()));
                },
                '\\' => self.slash(part_start, raw, fields, bytes, &mut saved, &mut missing, &mut noticed)?,
                '{' | '}' if fields => {
                    if self.source.get(self.next + 1) == Some(&ch) {
                        saved.push(ch);
                        self.forward(2);
                        noticed = false;
                    } else {
                        if ch == '}' { return Err(self.field_fault(1, &[], template)); }
                        self.flush(&mut saved, &mut missing);
                        self.field(part_start, raw, depth, end, 0, template)?;
                        part_start = self.next;
                        noticed = false;
                    }
                }
                _ if ch == '\n' => { saved.push(ch); self.forward(1); },
                _ => {
                    let left = self.next;
                    let right = self.source[left + 1..].iter().position(|letter| {
                        *letter == end[0] || matches!(*letter, '\\' | '\n')
                            || fields && matches!(*letter, '{' | '}') || bytes && !letter.is_ascii()
                    }).map_or(self.source.len(), |width| left + width + 1);
                    saved.extend(self.source[left..right].iter().copied());
                    self.next = right;
                },
            }
        }
        self.forward(end.len());
        if bytes { self.token(Shape::ByteQuote, saved); } else { self.flush(&mut saved, &mut missing); }
        if fields { self.token(Shape::WovenEnd, String::new()); }
        Ok(())
    }
    /// The escape a backslash begins that the reference warns about: what
    /// follows the backslash, and whether it is written in figures of
    /// eights. None where the reference takes the escape without a word.
    fn noticed_escape(&self, ch: char, bytes: bool) -> Option<(String, bool)> {
        if ['\\', '\r', '\n'].contains(&ch) { return None; }
        let table = self.table;
        let mut known = table.letters("lexical.string_escapes");
        known.extend(table.letters("lexical.string_quotes"));
        known.extend(table.letters("ext.lexical.escape.controls"));
        if known.contains(&ch) { return None; }
        if table.spells("ext.lexical.escape.byte", &ch.to_string()) { return None; }
        let lettered = table.spells("ext.lexical.escape.named", &ch.to_string())
            || table.spells("ext.lexical.escape.codepoint", &ch.to_string())
            || table.spells("ext.lexical.escape.codepoint.wide", &ch.to_string());
        if lettered { return if bytes { Some((ch.to_string(), false)) } else { None }; }
        if table.scanner.flag_ext_lexical_escape_octal && ch.is_digit(8) {
            let mut worth = 0u32;
            let mut tail = String::new();
            for ahead in 1..=3 {
                let Some(d) = self.source.get(self.next + ahead).and_then(|c| c.to_digit(8)) else { break };
                worth = worth * 8 + d;
                tail.push(self.source[self.next + ahead]);
            }
            return if worth > 0o377 { Some((tail, true)) } else { None };
        }
        Some((ch.to_string(), false))
    }

    fn slash(&mut self, body: usize, raw: bool, fields: bool, bytes: bool, text: &mut String, missing: &mut bool, noticed: &mut bool) -> Result<(), String> {
        let begin = self.next;
        let ch = *self.source.get(begin + 1).ok_or_else(|| self.bad())?;
        if !raw && !*noticed && self.table.scanner.strings_ext_lexical_escape_warning.as_slice().len() == 4 {
            if let Some((tail, octal)) = self.noticed_escape(ch, bytes) {
                let mut text = self.table.scanner.strings_ext_lexical_escape_warning.as_slice()[3].replace("{}", &tail);
                if octal { text = text.replacen("escape sequence", "octal escape sequence", 1); }
                self.token(Shape::EscapeNotice, text);
                *noticed = true;
            }
        }
        if raw || fields && matches!(ch, '{' | '}') {
            text.push('\\');
            self.forward(1);
            if raw && !(fields && matches!(ch, '{' | '}')) { text.push(ch); self.forward(1); }
            return Ok(());
        }
        let table = self.table;
        let letter = ch.to_string();
        if bytes && ["ext.lexical.escape.named", "ext.lexical.escape.codepoint", "ext.lexical.escape.codepoint.wide"].iter().any(|key| table.spells(key, &letter)) {
            text.extend(['\\', ch]);
            self.forward(2);
            return Ok(());
        }
        if table.spells("ext.lexical.escape.named", &letter) {
            self.forward(2);
            if self.here() == Some('{') {
                self.forward(1);
                while let Some(c) = self.here() {
                    if c == '}' || c == '\n' || table.letters("lexical.string_quotes").contains(&c) { break; }
                    self.forward(1);
                }
                if self.here() == Some('}') && self.next != begin + 3 {
                    let name: String = self.source[begin + 3..self.next].iter().collect();
                    self.forward(1);
                    if let Some(gate) = self.gate.as_ref() { gate.borrow_mut()()?; }
                    match crate::unicode::name_value(&name) {
                        Some(found) => text.push_str(&found),
                        None if table.scanner.single_ext_lexical_escape_named_unknown.as_deref().is_some() => {
                            let complaint = table.scanner.single_ext_lexical_escape_named_unknown.as_deref().expect("checked above");
                            let span = self.source[begin..self.next].iter().map(|c| c.len_utf8()).sum::<usize>();
                            let offset: usize = self.source[body..begin].iter().map(|c| c.len_utf8()).sum();
                            let words = complaint.replacen("{}", &offset.to_string(), 1)
                                .replacen("{}", &(offset + span - 1).to_string(), 1);
                            return Err(words);
                        }
                        None => *missing = true,
                    }
                    return Ok(());
                }
            }
            let consumed: String = self.source[begin..self.next].iter().collect();
            let mut message = table.scanner.single_ext_lexical_escape_named_amiss.as_deref().unwrap_or("Invalid string literal").to_owned();
            let offset: usize = self.source[body..begin].iter().map(|c| c.len_utf8()).sum();
            for place in [offset, offset + consumed.len() - 1] {
                message = message.replacen("{}", &place.to_string(), 1);
            }
            return Err(message);
        }
        let count = ["ext.lexical.escape.codepoint", "ext.lexical.escape.codepoint.wide"].iter()
            .find(|key| table.spells(key, &letter)).and_then(|key| table.count(&format!("{key}.digits")));
        if let Some(count) = count {
            let amiss = table.scanner.single_ext_lexical_escape_codepoint_amiss.as_deref().unwrap_or("Invalid Unicode escape");
            let beyond = table.scanner.single_ext_lexical_escape_codepoint_beyond.as_deref().unwrap_or("Unicode code point out of range");
            let mut value = 0u32;
            for place in begin + 2..begin + 2 + count {
                let digit = self.source.get(place).and_then(|c| c.to_digit(16)).ok_or_else(|| amiss.to_owned())?;
                value = value.checked_mul(16).and_then(|n| n.checked_add(digit)).ok_or_else(|| beyond.to_owned())?;
            }
            if value > 0x10ffff { return Err(beyond.to_owned()); }
            if let Some(ch) = char::from_u32(value) { text.push(ch); }
            else { self.substitutes.insert(text.len(), value); text.push('?'); }
            self.forward(count + 2);
            return Ok(());
        }
        if table.spells("ext.lexical.escape.byte", &letter) {
            if let Some(width) = table.scanner.count_ext_lexical_escape_byte_digits {
                if (begin + 2..begin + 2 + width).any(|at| !self.source.get(at).map_or(false, char::is_ascii_hexdigit)) {
                    return Err(table.scanner.single_ext_lexical_escape_codepoint_amiss.as_deref().unwrap_or("Invalid Unicode escape").to_owned());
                }
            }
        }
        let mut letters = table.letters("lexical.string_escapes");
        letters.extend(table.letters("ext.lexical.escape.controls"));
        if table.scanner.flag_ext_lexical_escape_continued && (ch == '\r' || ch == '\n') {
            self.forward(2);
            if ch == '\r' && self.here() == Some('\n') { self.forward(1); }
            return Ok(());
        }
        // A wide numbered alphabet keeps all three octal figures.
        if !bytes && table.scanner.flag_ext_lexical_escape_octal && table.scanner.count_ext_lexical_escape_codepoint_digits.is_some() && ch.is_digit(8) {
            self.forward(1);
            let mut worth = 0;
            while self.next < begin + 4 {
                let Some(d) = self.here().and_then(|c| c.to_digit(8)) else { break; };
                worth = 8 * worth + d;
                self.forward(1);
            }
            text.push(char::from_u32(worth).expect("three figures in eights"));
            return Ok(());
        }
        let slash = Backslash {
            letters: &letters, ends: None, numbered: table.letter("ext.lexical.escape.codepoint"),
            unbracketed: table.letter("ext.lexical.escape.byte"), eights: table.scanner.flag_ext_lexical_escape_octal,
            in_bytes: table.scanner.flag_ext_system_text_bytes, open: table.letter("ext.lexical.escape.codepoint.open"),
            shut: table.letter("ext.lexical.escape.codepoint.close"),
            amiss: table.scanner.single_ext_lexical_escape_codepoint_amiss.as_deref().unwrap_or("Invalid Unicode escape"),
            beyond: table.scanner.single_ext_lexical_escape_codepoint_beyond.as_deref().unwrap_or("Unicode code point out of range"), sigil: None,
        };
        let after = slash.reads(self.source, begin, text, &mut Vec::new())?;
        self.forward(after - begin);
        Ok(())
    }
    fn between_field_marks(&mut self) -> String {
        let mut whitespace = String::new();
        while let Some(ch) = self.here() {
            if ch.is_whitespace() {
                whitespace.push(ch);
                self.forward(1);
                continue;
            }
            if ch == '\\' && matches!(self.source.get(self.next + 1), Some('\n') | Some('\r')) {
                self.forward(1);
                let joined = self.here();
                self.forward(1);
                if joined == Some('\r') && self.here() == Some('\n') { self.forward(1); }
                whitespace.push('\\');
                whitespace.push('\n');
                continue;
            }
            let comment = self.table.scanner.strings_lexical_comment_line.as_slice().iter().any(|word|
                self.source[self.next..].starts_with(&word.chars().collect::<Vec<_>>()));
            if !comment { break; }
            while self.here().map_or(false, |c| c != '\n') { self.forward(1); }
        }
        whitespace
    }
    fn field_fault(&self, index: usize, inserts: &[String], template: Option<char>) -> String {
        let errors = if template.is_some() { self.table.strings("ext.lexical.string.template.errors") } else { self.table.scanner.strings_ext_lexical_string_format_errors.as_slice() };
        let mut words = errors.get(index)
            .cloned().unwrap_or_else(|| self.bad());
        for insert in inserts { words = words.replacen("{}", insert, 1); }
        words
    }
    fn field(&mut self, body: usize, raw: bool, depth: u32, delimiter: &[char], levels: usize, template: Option<char>) -> Result<(), String> {
        if levels >= 3 { return Err(self.field_fault(18, &[], template)); }
        self.forward(1);
        let origin = self.next;
        let field_row = self.row;
        let mut nesting = Vec::new();
        let mut comments = Vec::new();
        loop {
            let ch = self.here().ok_or_else(|| { self.pending_literal = true; self.field_fault(14, &[], template) })?;
            // A field is code, so a backslash in it stands exactly as one
            // written outside of any string does: it joins a line ending
            // right after it, and names nothing else. Caught here, before
            // `quoted_start`, so a quote following a bad backslash is
            // never mistaken for a nested string's opening mark.
            if ch == '\\' && !matches!(self.source.get(self.next + 1), Some('\n') | Some('\r')) {
                let said = self.table.scanner.single_ext_lexical_line_continuation_amiss.as_deref().unwrap_or_default().to_owned();
                return Err(said);
            }
            if let Some((body, end, bare, woven)) = quoted_start(self.source, self.next, self.table) {
                let previous = self.made.len();
                let possible_end = body == self.next + end.len() && end == delimiter && nesting.is_empty();
                match self.literal(body, &end, bare, woven, depth + 1) {
                    Ok(()) => {}
                    Err(said) => {
                        let unclosed = ["ext.lexical.string.unterminated", "ext.lexical.string.unterminated.triple"]
                            .iter().any(|label| self.table.single(label) == Some(said.as_str()));
                        return Err(if possible_end && unclosed { self.field_fault(0, &[], template) } else { said });
                    }
                }
                self.made.truncate(previous);
                continue;
            }
            if let Some(said) = prefix_conflict(self.source, self.next, self.table) { return Err(said); }
            if self.table.scanner.strings_lexical_comment_line.as_slice().iter().any(|s| self.source[self.next..].starts_with(&s.chars().collect::<Vec<_>>())) {
                let begins = self.next;
                while self.here().map_or(false, |c| c != '\n') { self.forward(1); }
                comments.push(begins..self.next);
                if self.here().is_none() { self.pending_literal = true; return Err(self.field_fault(14, &[], template)); }
                continue;
            }
            if ch == '\u{00a0}' { return Err(self.field_fault(15, &[], template)); }
            if !nesting.is_empty() && [';', '/', '>'].contains(&ch) {
                let prior = self.source[origin..self.next].iter().rev().find(|c| !c.is_whitespace());
                if prior.map_or(false, |c| ['{', '[', '('].contains(c)) { return Err(self.field_fault(3, &[], template)); }
            }
            if nesting.is_empty() {
                let leading: String = (origin..self.next).filter(|i| !comments.iter().any(|r| r.contains(i)))
                    .map(|i| self.source[i]).collect();
                let needs_value = leading.chars().all(|c| c.is_whitespace() || ['+', '-', '~'].contains(&c));
                let following = self.source.get(self.next + 1).copied();
                let forbidden = [';', '$'].contains(&ch) || needs_value && (
                    [',', '/', '>', '<', '%'].contains(&ch)
                    || ch == '*' && following == Some('*')
                    || ch == '.' && !following.map_or(false, |c| c == '.' || c.is_ascii_digit()));
                if forbidden { return Err(self.field_fault(if needs_value { 3 } else { 8 }, &[], template)); }
                let word: String = self.source[self.next..].iter().take_while(|c| c.is_alphanumeric() || **c == '_').collect();
                if !word.is_empty() && !self.source.get(self.next.wrapping_sub(1)).map_or(false, |c| c.is_alphanumeric() || *c == '_')
                    && self.table.spells("ext.op.lambda", &word) {
                    let after_comma = leading.rsplit(',').next().unwrap_or("").trim();
                    let issue = if after_comma.is_empty() { 4 } else { 3 };
                    return Err(self.field_fault(issue, &[], template));
                }
            }
            let after = self.source.get(self.next + 1).copied();
            let before = self.next.checked_sub(1).and_then(|i| self.source.get(i)).copied();
            if nesting.is_empty() {
                let equals = ch == '=' && after != Some('=') && !matches!(before, Some('=' | '!' | '<' | '>'));
                if equals || ch == ':' || ch == '}' || ch == '!' && after != Some('=') { break; }
            }
            if let Some(index) = ['(', '[', '{'].iter().position(|&c| c == ch) {
                nesting.push([')', ']', '}'][index]);
            } else if [')', ']', '}'].contains(&ch) {
                let Some(wanted) = nesting.pop() else { return Err(self.field_fault(12, &[ch.to_string()], template)); };
                if wanted != ch {
                    let opener = ['(', '[', '{'][[')', ']', '}'].iter().position(|c| *c == wanted).unwrap()];
                    return Err(self.field_fault(13, &[ch.to_string(), opener.to_string()], template));
                }
            }
            self.forward(1);
        }
        let code: String = (origin..self.next).filter(|i| !comments.iter().any(|r| r.contains(i))).map(|i| self.source[i]).collect();
        if code.trim().is_empty() { return Err(self.field_fault(2, &[self.here().unwrap().to_string()], template)); }
        let stripped = code.trim_start();
        if stripped.starts_with('*') && !stripped.contains(',') { return Err(self.field_fault(16, &[], template)); }
        let words: Vec<&str> = code.split_whitespace().collect();
        if words.len() == 2 && words.iter().all(|word| word.chars().all(|ch| ch.is_ascii_digit())) {
            let where_digit = (origin..self.next).find(|&at| self.source[at].is_ascii_digit()).unwrap_or(origin);
            self.next = where_digit;
            self.row = self.source[..where_digit].iter().filter(|&&ch| ch == '\n').count() as u32 + 1;
            return Err(String::from("SyntaxError: invalid syntax. Perhaps you forgot a comma?"));
        }
        // Notices in a field belong to its source row, just like its tokens.
        for (notice, relative_row) in escape_notices(&code, self.table) {
            self.token(Shape::EscapeNotice, notice);
            self.made.last_mut().unwrap().row = field_row + relative_row.saturating_sub(1);
        }
        let debugging = self.here() == Some('=');
        if debugging {
            self.forward(1);
            let mut label = code.clone();
            label.push('=');
            label.push_str(&self.between_field_marks());
            self.token(Shape::Quote, label);
        }
        let convert = if self.here() == Some('!') {
            self.forward(1);
            if self.here().is_none() || self.source[self.next..].starts_with(delimiter) { self.pending_literal = self.here().is_none(); return Err(self.field_fault(0, &[], template)); }
            let first = self.here().unwrap();
            let problem = match first {
                '}' | ':' => Some(5),
                c if c.is_whitespace() => Some(7),
                c if !c.is_alphabetic() && c != '_' => Some(6),
                _ => None,
            };
            if let Some(problem) = problem { return Err(self.field_fault(problem, &[], template)); }
            let mut name = String::new();
            while let Some(c) = self.here().filter(|c| c.is_alphanumeric() || *c == '_') {
                name.push(c);
                self.forward(1);
            }
            if !["a", "r", "s"].contains(&name.as_str()) { return Err(self.field_fault(11, &[name], template)); }
            self.between_field_marks();
            name
        } else if debugging && self.here() != Some(':') { "r".to_owned() } else { String::new() };
        if self.source[self.next..].starts_with(delimiter) || self.here().is_none() { self.pending_literal = self.here().is_none(); return Err(self.field_fault(0, &[], template)); }
        if !matches!(self.here(), Some('}' | ':')) {
            let issue = if debugging || convert.is_empty() { 9 } else { 10 };
            return Err(self.field_fault(issue, &[], template));
        }
        self.token(Shape::Field, convert);
        self.token(Shape::FieldText, code.clone());
        let left = self.table.scanner.single_syntax_group_open.as_deref().ok_or_else(|| self.bad())?.to_owned();
        let right = self.table.scanner.single_syntax_group_close.as_deref().ok_or_else(|| self.bad())?.to_owned();
        self.token(Shape::Sign, left);
        let tokens = match scan_with_gate(&code, self.table, self.gate.clone()) {
            Ok(tokens) => tokens,
            Err((message, relative_row, relative_column)) => {
                let mut line = 1;
                let mut start = origin;
                while line < relative_row && start < self.source.len() {
                    if self.source[start] == '\n' { line += 1; }
                    start += 1;
                }
                self.next = (start + relative_column.saturating_sub(1)).min(self.source.len());
                self.row = field_row + relative_row.saturating_sub(1);
                return Err(message);
            }
        };
        // The tokens were read out of the field's own text, whose rows
        // count from one; they stand where the field stands, so each is
        // moved down to the row the field opens on.
        self.made.extend(tokens.into_iter().filter(|t| !matches!(t.shape, Shape::Lead | Shape::LineEnd | Shape::Finish)).map(|mut t| {
            t.row = t.row + field_row.saturating_sub(1);
            if t.end_row != 0 { t.end_row = t.end_row + field_row.saturating_sub(1); }
            t
        }));
        self.token(Shape::Sign, right);
        self.token(Shape::Woven, String::new());
        let mut noticed = false;
        let mut specification = String::new();
        let mut missing = false;
        if self.here() == Some(':') {
            self.forward(1);
            while self.here() != Some('}') {
                if self.source[self.next..].starts_with(delimiter) { return Err(self.field_fault(19, &[], template)); }
                match self.here().ok_or_else(|| { self.pending_literal = true; self.field_fault(0, &[], template) })? {
                    '\n' | '\r' if delimiter.len() == 1 => return Err(self.field_fault(17, &[], template)),
                    '{' => { self.flush(&mut specification, &mut missing); self.field(body, raw, depth, delimiter, levels + 1, template)?; noticed = false; }
                    '\\' => self.slash(body, raw, true, false, &mut specification, &mut missing, &mut noticed)?,
                    c => { specification.push(c); self.forward(1); }
                }
            }
        }
        if self.here() != Some('}') { self.pending_literal = self.here().is_none(); return Err(self.field_fault(0, &[], template)); }
        self.forward(1);
        self.flush(&mut specification, &mut missing);
        self.token(Shape::WovenEnd, String::new());
        Ok(())
    }
}

fn continued_past_end(tokens: &[Token], position: &mut (u32, usize)) -> String {
    let mut nesting: Vec<&Token> = Vec::new();
    for item in tokens.iter().filter(|item| item.shape == Shape::Sign) {
        if ["(", "[", "{"].contains(&item.lexeme.as_str()) { nesting.push(item); }
        else if [")", "]", "}"].contains(&item.lexeme.as_str()) { nesting.pop(); }
    }
    match nesting.last() {
        Some(begin) => {
            *position = (begin.row, begin.column);
            format!("SyntaxError: '{}' was never closed", begin.lexeme)
        }
        None => "SyntaxError: unexpected EOF while parsing".to_owned(),
    }
}

/// Ask the quotation reader whether EOF interrupted a formatted literal,
/// without changing the SyntaxError used by ordinary compilation.
pub fn pending_quotation(source: &str, table: &Table) -> bool {
    let mut pending = false;
    let _ = scan_code_from(source, table, 1, &mut (1, 1), None, &mut pending);
    pending
}

fn scan_code_from<'a>(source: &str, table: &'a Table, first: u32, ended: &mut (u32, usize), gate: Option<NameGate<'a>>, pending: &mut bool) -> Result<Vec<Token>, String> {
    let paired_ending = source.as_bytes().ends_with(b"\r\n");
    let input = if table.syntax_exceptions && source.contains('\r') {
        std::borrow::Cow::Owned(source.replace("\r\n", "\n").replace('\r', "\n"))
    } else { std::borrow::Cow::Borrowed(source) };
    let source = input.as_ref();
    let text = drop_comments(source, table);
    let src: Vec<char> = text.chars().collect();
    let mut punctuation: std::collections::BTreeMap<char, Vec<(&String, Vec<char>)>> = std::collections::BTreeMap::new();
    for sign in &table.signs {
        if let Some(lead) = sign.chars().next() {
            punctuation.entry(lead).or_default().push((sign, sign.chars().collect()));
        }
    }
    let quotes = table.letters("lexical.string_quotes");
    let raw = table.letters("lexical.raw_quotes");
    let weaving = table.letters("ext.lexical.interpolating_quotes");
    let escapes = table.letters("lexical.string_escapes");
    // A character an escape names by its number: the letter that begins
    // one, the brackets the number stands in, and what the language
    // says of a number badly written or of one beyond the last there is.
    let numbered = table.letter("ext.lexical.escape.codepoint");
    let number_open = table.letter("ext.lexical.escape.codepoint.open");
    let number_close = table.letter("ext.lexical.escape.codepoint.close");
    let badly = table.scanner.single_ext_lexical_escape_codepoint_amiss.as_deref().unwrap_or("Bad character number");
    let too_far = table.scanner.single_ext_lexical_escape_codepoint_beyond.as_deref().unwrap_or("Character number too large");
    // The letter beginning a character named by a bare number instead.
    let unbracketed = table.letter("ext.lexical.escape.byte");
    // Whether figures in eights after a backslash name a character too.
    let eights = table.scanner.flag_ext_lexical_escape_octal;
    // Whether what is kept is bytes rather than the letters they spell.
    let in_bytes = table.scanner.flag_ext_system_text_bytes;
    let point = table.letter("lexical.number.decimal_point");
    let prefix = table.letter("identifier.variable_prefix");
    let folding = table.scanner.single_ext_lexical_heredoc.as_deref();
    // The escapes a folded string reads: those of a quoted one, less the
    // quote marks, which have nothing to be shielded from there.
    let unquoted: Vec<char> = escapes.iter().copied().filter(|e| !quotes.contains(e)).collect();
    let quote_name = table.letter("lexical.name_quote");
    let unit = table.scanner.count_block_indent_size.unwrap_or(4);
    let fold_kw = table.scanner.flag_lexical_keywords_case_insensitive;
    let fold_id = table.scanner.flag_identifier_case_insensitive;
    let mut tokens: Vec<Token> = Vec::new();
    let line_beginnings: Vec<usize> = std::iter::once(0).chain(src.iter().enumerate().filter_map(|(i, c)| (*c == '\n').then_some(i + 1))).collect();
    // The reading moves forward, so the line last found is tried first.
    let last_line = std::cell::Cell::new(0usize);
    let at_column = |index: usize| {
        let mut line = last_line.get();
        if line_beginnings[line] > index || line_beginnings.get(line + 1).is_some_and(|next| *next <= index) {
            line = line_beginnings.partition_point(|start| *start <= index) - 1;
            last_line.set(line);
        }
        index - line_beginnings[line] + 1
    };
    let column = std::cell::Cell::new(1usize);
    let tok = |kind: Shape, text: String, row: u32| Token { utf8_column: 0, utf8_end: 0, end_row: 0, end_column: 0, column: column.get(), shape: kind, lexeme: text, raw: None, span: 0, row: row };
    let tok_spelled = |kind: Shape, text: String, raw: Option<String>, row: u32| Token { utf8_column: 0, utf8_end: 0, end_row: 0, end_column: 0, column: column.get(), shape: kind, lexeme: text, raw, span: 0, row: row };
    let (mut pos, mut row, mut at_bol) = (0usize, first, true);
    while pos < src.len() {
        // The row the reading has reached is kept where the caller can
        // see it, so that a reading that stops names the right line.
        *ended = (row, at_column(pos));
        if at_bol {
            at_bol = false;
            let mut width = 0;
            let mut k = pos;
            while k < src.len() && (src[k] == ' ' || src[k] == '\t') {
                width += match src[k] {
                    '\t' if table.syntax_exceptions => 8 - width % 8,
                    '\t' => unit, _ => 1,
                };
                k += 1;
            }
            let line_end = src[k..].iter().position(|c| *c == '\n').map_or(src.len(), |p| k + p);
            if src[k..line_end].iter().all(|c| c.is_ascii_whitespace() || !table.syntax_exceptions && c.is_whitespace())
                || table.scanner.strings_lexical_comment_line.as_slice().iter().any(|mark| src[k..].starts_with(&mark.chars().collect::<Vec<_>>())) {
                if line_end < src.len() {
                    row += 1;
                }
                pos = (line_end + 1).min(src.len());
                at_bol = true;
                continue;
            }
            tokens.push(Token { utf8_column: 0, utf8_end: 0, end_row: 0, end_column: 0, column: 1, shape: Shape::Lead, lexeme: src[pos..k].iter().collect(), raw: None, span: width, row: row });
            pos = k;
        }
        column.set(at_column(pos));
        ended.1 = column.get();
        if !table.scanner.strings_ext_lexical_string_long.as_slice().is_empty() {
            if table.scanner.strings_lexical_comment_line.as_slice().iter().any(|m| src[pos..].starts_with(&m.chars().collect::<Vec<_>>())) {
                while pos < src.len() && src[pos] != '\n' { pos += 1; }
                continue;
            }
            if let Some((body, end, raw, fields)) = quoted_start(&src, pos, table) {
                let mut quote = Quotation { gate: gate.clone(), source: &src, next: pos, row, table, made: Vec::new(), substitutes: std::collections::BTreeMap::new(), pending_literal: false };
                if let Err(mut words) = quote.literal(body, &end, raw, fields, 0) {
                    *pending = quote.pending_literal;
                    if !fields && table.syntax_exceptions && words.starts_with("SyntaxError: unterminated ") {
                        let mut detected = quote.row;
                        if !paired_ending && quote.next >= src.len() && src.last() == Some(&'\n') { detected = detected.saturating_sub(1); }
                        words.push_str(&format!(" (detected at line {detected})"));
                        let mut stop = quote.next.min(src.len());
                        if stop != 0 && src[stop - 1] == '\n' { stop -= 1; }
                        if end.len() == 1 && stop > 1 && src[stop - 1] == end[0] {
                            let mut slash_at = stop - 1;
                            while slash_at > 0 && src[slash_at - 1] == '\\' { slash_at -= 1; }
                            if (stop - 1 - slash_at) % 2 != 0 { words.push_str("; perhaps you escaped the end quote?"); }
                        }
                    }
                    if fields && (words.contains("invalid decimal literal") || words.contains("Perhaps you forgot a comma?")) {
                        ended.0 = quote.row;
                        ended.1 = at_column(quote.next);
                    }
                    return Err(words);
                }
                if !fields {
                    for token in &mut quote.made {
                        if token.shape == Shape::EscapeNotice { continue; }
                        token.column = at_column(pos); token.row = row;
                        token.end_column = at_column(quote.next); token.end_row = quote.row;
                    }
                }
                pos = quote.next;
                row = quote.row;
                tokens.extend(quote.made);
                continue;
            }
            if let Some(said) = prefix_conflict(&src, pos, table) { return Err(said); }
            if table.scanner.flag_ext_lexical_string_adjacent && src[pos] == '\\' && src.get(pos + 1) == Some(&'\n') {
                if pos + 2 == src.len() && table.syntax_exceptions { ended.1 += 1; return Err(continued_past_end(&tokens, ended)); }
                pos += 2;
                row += 1;
                continue;
            }
        }
        let c = src[pos];
        let mut carried = false;
        for mark in table.scanner.strings_ext_lexical_line_continuation.as_slice() {
            if written_at(&src, pos, mark) {
                let mut end = pos + mark.chars().count();
                if src.get(end) == Some(&'\r') {
                    end += 1;
                }
                if src.get(end) == Some(&'\n') {
                    if end + 1 == src.len() && table.syntax_exceptions { ended.1 = at_column(end); return Err(continued_past_end(&tokens, ended)); }
                    pos = end + 1;
                    row += 1;
                    carried = true;
                    break;
                }
            }
        }
        if carried {
            continue;
        }
        if c == '\n' {
            tokens.push(tok(Shape::LineEnd, "\n".into(), row));
            row += 1;
            pos += 1;
            at_bol = true;
            continue;
        }
        if c == '\u{a0}' && table.scanner.has_any_ext_lexical_string_format_errors {
            return Err(table.scanner.strings_ext_lexical_string_format_errors.as_slice()[15].clone());
        }
        if c.is_whitespace() {
            pos += 1;
            continue;
        }
        if table.spells("ext.lexical.line_continuation", &c.to_string()) {
            pos += 1;
            if src.get(pos) == Some(&'\r') { pos += 1; }
            if src.get(pos) != Some(&'\n') {
                ended.1 = at_column(pos);
                if pos == src.len() && table.syntax_exceptions { return Err(continued_past_end(&tokens, ended)); }
                return Err(table.scanner.single_ext_lexical_line_continuation_amiss.as_deref().unwrap_or_default().to_string());
            }
            row += 1;
            pos += 1;
            continue;
        }
        if quotes.contains(&c) {
            let delimiter = table.scanner.strings_ext_lexical_string_long.as_slice().iter().find(|word| {
                let letters: Vec<char> = word.chars().collect();
                letters.iter().all(|letter| *letter == c)
                    && src.get(pos..pos + letters.len()) == Some(letters.as_slice())
            });
            let length = delimiter.map_or(1, |word| word.chars().count());
            let is_raw = raw.contains(&c);
            let woven = weaving.contains(&c);
            let slash = Backslash {
                letters: if is_raw { &[] } else { &escapes },
                ends: Some(c),
                numbered: if is_raw { None } else { numbered },
                unbracketed: if is_raw { None } else { unbracketed },
                eights: !is_raw && eights,
                in_bytes,
                open: number_open,
                shut: number_close,
                amiss: badly,
                beyond: too_far,
                sigil: if woven { prefix } else { None },
            };
            // Positions in s that were escaped, so never open a variable.
            let mut plain: Vec<usize> = Vec::new();
            let (mut s, mut k, mut closed) = (String::new(), pos + length, false);
            while k < src.len() {
                let d = src[k];
                if d == '\\' && k + 1 < src.len() {
                    if !is_raw && table.spells("ext.lexical.line_continuation", "\\") {
                        let end = k + 1 + usize::from(src[k + 1] == '\r');
                        if src.get(end) == Some(&'\n') {
                            row += 1;
                            k = end + 1;
                            continue;
                        }
                    }
                    k = slash.reads(&src, k, &mut s, &mut plain)?;
                    continue;
                }
                let ends_here = src.get(k..k + length).map_or(false, |tail| tail.iter().all(|letter| *letter == c));
                if ends_here {
                    k += length;
                    closed = true;
                    break;
                }
                k += 1;
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
        if c.is_ascii_digit() || ((table.scanner.flag_ext_lexical_number_point_open || table.scanner.flag_ext_lexical_number_point_bare || table.scanner.flag_ext_lexical_number_point_edge || table.scanner.flag_ext_lexical_number_point_open_plain)
            && Some(c) == point && src.get(pos + 1).map_or(false, char::is_ascii_digit))
        {
            let strict = table.scanner.flag_ext_lexical_number_separator_strict;
            let mut k = numeral_end(&src, pos, table);
            let mut spelling: String = src[pos..k].iter().collect();
            if strict && check_numeral(&spelling, table).is_err() && table.syntax_exceptions {
                for word in ["else", "not", "and", "or", "in", "if", "is", "for"] {
                    if let Some(beginning) = spelling.strip_suffix(word) {
                        if !beginning.is_empty() && check_numeral(beginning, table).is_ok()
                            && !(beginning == "0" && word == "or")
                            && !((beginning.starts_with("0x") || beginning.starts_with("0X")) && matches!(word, "and" | "else")) {
                            let kept = beginning.len();
                            k -= word.len();
                            spelling.truncate(kept);
                            break;
                        }
                    }
                }
            }
            if strict {
                if let Err(mut words) = check_numeral(&spelling, table) {
                    if table.syntax_exceptions {
                        ended.1 = column.get() + failed_digit(&spelling, &words);
                        if table.scanner.single_ext_lexical_number_amiss.as_deref() == Some(words.as_str()) { words = "SyntaxError: invalid decimal literal".to_string(); }
                    }
                    return Err(words);
                }
            }
            tokens.push(tok(Shape::Numeral, spelling, row));
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
        // A string folded over lines (ext.lexical.heredoc): what its
        // lines say is one piece of text, read as a quoted string is
        // read, save that the quote marks in it stand for themselves,
        // the body being ended by its label and not by a mark. A label
        // in raw quotes asks for a body that spells only itself.
        if folding.map_or(false, |mark| written_at(&src, pos, mark)) {
            let ahead: String = src[pos..].iter().collect();
            let Some((body, bare, reach)) = folded(&ahead, table) else {
                return Err("Unterminated string over lines".to_string());
            };
            let opened = row;
            row += ahead[..reach].matches('\n').count() as u32;
            pos += ahead[..reach].chars().count();
            if bare {
                tokens.push(tok(Shape::Quote, body, opened));
                continue;
            }
            let slash = Backslash {
                letters: &unquoted,
                ends: None,
                numbered,
                unbracketed,
                eights,
                in_bytes,
                open: number_open,
                shut: number_close,
                amiss: badly,
                beyond: too_far,
                sigil: prefix,
            };
            let folds: Vec<char> = body.chars().collect();
            let (mut said, mut plain, mut k) = (String::new(), Vec::new(), 0);
            while k < folds.len() {
                if folds[k] == '\\' && k + 1 < folds.len() {
                    k = slash.reads(&folds, k, &mut said, &mut plain)?;
                    continue;
                }
                said.push(folds[k]);
                k += 1;
            }
            weave(&said, &plain, table, opened, &mut tokens)?;
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
            let routed = !table.scanner.strings_ext_builtin_print_redirect.as_slice().is_empty();
            for name in table.continued_builtins(&s) {
                if name.len() > s.len() && name.starts_with(s.as_str()) {
                    // With the printer routed through the module, a stream
                    // name is a chain of members to be followed one at a
                    // time, and so is the writer spelled through it.
                    if routed && (table.prims.get(name) == Some(&crate::form::Prim::Echo)
                        || table.spells("ext.builtin.print.file.output", name) || table.spells("ext.builtin.print.file.error", name)) { continue; }
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
            // Python folds every identifier to NFKC for binding, but its
            // own words are matched against the spelling the text wrote,
            // so the token keeps both when they differ.
            let mut raw = None;
            if !s.is_ascii() && table.scanner.flag_ext_lexical_identifier_normalized {
                let folded = crate::unicode::collated(&s, 2);
                if folded != s { raw = Some(std::mem::replace(&mut s, folded)); }
            }
            // Only a folding definition needs the lowercased identifier buffer.
            if fold_id || fold_kw {
                let low = s.to_lowercase();
                if fold_id || table.keywords.contains(&low) { s = low; }
            }
            tokens.push(tok_spelled(Shape::Bare, s, raw, row));
            pos = k;
            continue;
        }
        let found = punctuation.get(&c).and_then(|options| {
            options.iter().find_map(|(sign, letters)| {
                (letters.len() <= 8 && src.get(pos..pos + letters.len()) == Some(letters.as_slice())).then_some(*sign)
            })
        });
        let Some(sym) = found.cloned() else {
            return Err(told_of_character(table, c, row));
        };
        pos += sym.chars().count();
        tokens.push(tok(Shape::Sign, sym, row));
    }
    if table.scanner.flag_ext_syntax_call_bind_names {
        let mut nesting: usize = 0;
        let mut joined = Vec::with_capacity(tokens.len());
        for token in tokens {
            match token.shape {
                Shape::Sign if table.spells("syntax.call.open", &token.lexeme) => nesting += 1,
                Shape::Sign if table.spells("syntax.call.close", &token.lexeme) => nesting = nesting.saturating_sub(1),
                Shape::Lead | Shape::LineEnd if nesting != 0 => continue,
                _ => {}
            }
            joined.push(token);
        }
        tokens = joined;
    }
    // Brackets may stand open two hundred deep and no deeper, as the
    // reference's reader has it; the one past that is refused, for a
    // table with syntax words of its own.
    if table.syntax_exceptions {
        let mut standing = 0usize;
        for item in tokens.iter().filter(|item| item.shape == Shape::Sign) {
            let text = item.lexeme.as_str();
            if [")", "]", "}"].contains(&text) { standing = standing.saturating_sub(1); continue; }
            if !["(", "[", "{"].contains(&text) { continue; }
            standing += 1;
            if standing > 200 {
                *ended = (item.row, item.column);
                return Err("SyntaxError: too many nested parentheses".to_owned());
            }
        }
    }
    column.set(at_column(src.len()));
    tokens.push(tok(Shape::Finish, "EOF".into(), row));
    if table.has_any("ext.builtin.exceptions.traceback") {
        let mut byte_rows = Vec::new();
        for line in source.split('\n') {
            let mut total = 0;
            let mut points = vec![1];
            for character in line.chars() { total += character.len_utf8(); points.push(total + 1); }
            byte_rows.push(points);
        }
        for item in tokens.iter_mut() {
            let beginning = item.row.saturating_sub(first) as usize;
            item.utf8_column = byte_rows.get(beginning).and_then(|points| points.get(item.column.saturating_sub(1)))
                .copied().unwrap_or(item.column);
            if item.end_column != 0 {
                item.utf8_end = byte_rows.get(item.end_row.saturating_sub(first) as usize)
                    .and_then(|points| points.get(item.end_column - 1)).copied().unwrap_or(item.end_column);
            }
        }
    }
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
/// Whether a mark stands written at that place in a run of letters.
fn written_at(cs: &[char], at: usize, mark: &str) -> bool {
    !mark.is_empty() && cs.len() >= at + mark.chars().count() && cs[at..].iter().zip(mark.chars()).all(|(c, m)| *c == m)
}

/// Where a mark next stands from that place on, and nothing where it
/// stands nowhere after it.
fn next_written(cs: &[char], from: usize, mark: &str) -> Option<usize> {
    (from..cs.len()).find(|at| written_at(cs, *at, mark))
}

fn pieces(s: &str, plain: &[usize], table: &Table) -> Result<Vec<Piece>, String> {
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
            // One step beyond the binding comes with it, which is the
            // whole of this shorter way of writing: a place asked for
            // in brackets, or a member asked for after the member mark.
            // Anything longer wants the braces that take code entire.
            let mut said: Option<String> = None;
            let mut took_step = false;
            if let (Some(shut), Some(opener)) = (table.scanner.single_op_index_close.as_deref(), table.scanner.single_op_index_open.as_deref()) {
                if written_at(&cs, end, opener) {
                    let after = end + opener.chars().count();
                    if let Some(stop) = next_written(&cs, after, shut) {
                        let inside: String = cs[after..stop].iter().collect();
                        let numeric = |w: &str| !w.is_empty() && w.chars().all(|c| c.is_ascii_digit());
                        let all_digits = numeric(&inside) || inside.strip_prefix('-').map_or(false, numeric);
                        let a_name = inside.chars().next().map_or(false, |c| Some(c) == sigil);
                        let a_word = inside.chars().next().map_or(false, |c| table.begins_name(c))
                            && inside.chars().all(|c| table.extends_name(c));
                        // Anything else asked for between the brackets
                        // is beyond what the shorter writing reaches,
                        // and a language having words for that stops
                        // rather than let the brackets stand as letters.
                        if let (false, Some(words)) = (all_digits || a_name || a_word, told_of_key(table)) {
                            return Err(words);
                        }
                        let beyond = stop + shut.chars().count();
                        if all_digits || a_name {
                            end = beyond;
                            took_step = true;
                        } else if a_word {
                            // A word written bare between the brackets
                            // asks for the text it spells, not a name,
                            // so it is handed on written as text.
                            let quotes = table.scanner.strings_lexical_raw_quotes.as_slice();
                            let any = table.scanner.strings_lexical_string_quotes.as_slice();
                            if let Some(mark) = quotes.first().or_else(|| any.first()) {
                                let held: String = cs[at..end].iter().collect();
                                said = Some(format!("{held}{opener}{mark}{inside}{mark}{shut}"));
                                end = beyond;
                                took_step = true;
                            }
                        }
                    }
                }
            }
            if !took_step {
                if let Some(mark) = table.scanner.single_ext_op_member.as_deref() {
                    if written_at(&cs, end, mark) {
                        let after = end + mark.chars().count();
                        if cs.get(after).map_or(false, |c| table.begins_name(*c)) {
                            let mut beyond = after + 1;
                            while beyond < cs.len() && table.extends_name(cs[beyond]) {
                                beyond += 1;
                            }
                            end = beyond;
                        }
                    }
                }
            }
            out.push(Piece::Text(std::mem::take(&mut text)));
            out.push(Piece::Code(said.unwrap_or_else(|| cs[at..end].iter().collect())));
            at = end;
            continue;
        }
        text.push(cs[at]);
        at += 1;
    }
    out.push(Piece::Text(text));
    Ok(out)
}

/// What a language says of a key asked for between the brackets of a
/// name woven into text where the shorter writing will not have it.
/// Nothing where the language says nothing, the brackets then standing
/// as the letters they are written with.
fn told_of_key(table: &Table) -> Option<String> {
    let opening = table.scanner.single_ext_system_reading_unexpected.as_deref()?;
    let words = table.scanner.single_ext_lexical_interpolating_index_amiss.as_deref()?;
    Some(format!("{opening} {words}"))
}

/// What a language says of a character it has no reading for at all.
/// A language naming such characters by their number names it that
/// way, a character no one can show being no use in a complaint.
fn told_of_character(table: &Table, c: char, row: u32) -> String {
    if c.is_control() && table.syntax_exceptions { return format!("SyntaxError: invalid non-printable character U+{:04X}", u32::from(c)); }
    if table.syntax_exceptions && c as u32 > 127 { return format!("SyntaxError: invalid character '{}' (U+{:04X})", c, u32::from(c)); }
    match (table.scanner.single_ext_system_reading_unexpected.as_deref(), table.scanner.single_ext_system_reading_unexpected_character.as_deref()) {
        (Some(opening), Some(named)) => format!("{opening} {named}{:02X}", c as u32),
        _ => format!("Unexpected character '{c}' at row {row}"),
    }
}

/// The tokens of a woven string: a bracketed concatenation starting
/// from the empty string, so the result is text whatever is woven in.
fn weave(s: &str, plain: &[usize], table: &Table, row: u32, tokens: &mut Vec<Token>) -> Result<(), String> {
    let cut = pieces(s, plain, table)?;
    let sign = |text: &str| Token { utf8_column: 0, utf8_end: 0, end_row: 0, end_column: 0, column: 1, shape: Shape::Sign, lexeme: text.to_string(), raw: None, span: 0, row };
    let quote = |text: String| Token { utf8_column: 0, utf8_end: 0, end_row: 0, end_column: 0, column: 1, shape: Shape::Quote, lexeme: text, raw: None, span: 0, row };
    if !cut.iter().any(|p| matches!(p, Piece::Code(_))) {
        tokens.push(quote(s.to_string()));
        return Ok(());
    }
    let (Some(open), Some(close), Some(join)) = (table.scanner.single_syntax_group_open.as_deref(), table.scanner.single_syntax_group_close.as_deref(), table.scanner.single_op_concat.as_deref()) else {
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

/// Separators have a place between figures, and one may follow a base
/// mark. Keep them until this asking is over, lest a bad spelling pass.
pub fn check_numeral(word: &str, table: &Table) -> Result<(), String> {
    let failure = |label: &str| table.single(label).or_else(|| table.scanner.single_ext_lexical_number_amiss.as_deref())
        .map(str::to_owned).unwrap_or_else(|| format!("Invalid number: {}", word));
    let breaks = table.letters("ext.lexical.number.separator");
    let digits = |run: &str, base: u32, prefixed: bool| {
        let mut previous = prefixed;
        let mut count = 0;
        for c in run.chars() {
            if c.is_digit(base) { previous = true; count += 1; }
            else if breaks.contains(&c) && previous { previous = false; }
            else { return false; }
        }
        previous && count > 0
    };
    for (label, base, complaint, bad_digit) in [
        ("lexical.number.hex_prefix", 16, "ext.lexical.number.amiss.hex", ""),
        ("ext.lexical.number.octal_prefix", 8, "ext.lexical.number.amiss.octal", "ext.lexical.number.amiss.octal.digit"),
        ("ext.lexical.number.binary_prefix", 2, "ext.lexical.number.amiss.binary", "ext.lexical.number.amiss.binary.digit"),
    ] {
        for opening in table.strings(label) {
            let Some(tail) = word.strip_prefix(opening.as_str()) else { continue };
            let mut last_was_break = false;
            for letter in tail.chars() {
                if letter.is_digit(base) { last_was_break = false; continue; }
                if breaks.contains(&letter) && !last_was_break { last_was_break = true; continue; }
                if !bad_digit.is_empty() && letter.is_ascii_digit() {
                    let pieces = table.strings(bad_digit);
                    if pieces.len() == 2 {
                        return Err([pieces[0].as_str(), &letter.to_string(), pieces[1].as_str()].concat());
                    }
                }
                return Err(failure(complaint));
            }
            return digits(tail, base, true).then_some(()).ok_or_else(|| failure(complaint));
        }
    }
    let mut end = word.len();
    let has_imaginary = word.chars().next_back().map_or(false, |last| {
        if table.letters("ext.lexical.number.imaginary").contains(&last) { end -= last.len_utf8(); true } else { false }
    });
    let decimal = &word[..end];
    let exponents = table.letters("ext.lexical.number.exponent");
    let mut parts = decimal.splitn(2, |c| exponents.contains(&c));
    let front = parts.next().unwrap_or("");
    let power = parts.next();
    let bad = || failure("ext.lexical.number.amiss");
    if let Some(p) = power {
        let p = p.strip_prefix('+').or_else(|| p.strip_prefix('-')).unwrap_or(p);
        if !digits(p, 10, false) { return Err(bad()); }
    }
    let dot = table.letter("lexical.number.decimal_point").and_then(|p| front.find(p).map(|n| (n, p)));
    match dot {
        Some((n, p)) => {
            let a = &front[..n];
            let b = &front[n + p.len_utf8()..];
            if a.is_empty() && b.is_empty() || !(a.is_empty() || digits(a, 10, false)) || !(b.is_empty() || digits(b, 10, false)) {
                return Err(bad());
            }
        }
        None if !digits(front, 10, false) => return Err(bad()),
        _ => (),
    }
    if !has_imaginary && power.is_none() && dot.is_none() && decimal.starts_with('0') {
        let nonzero = decimal.chars().any(|c| matches!(c, '1'..='9'));
        if nonzero && table.scanner.single_ext_lexical_number_amiss_leading_zero.as_deref().is_some() {
            return Err(failure("ext.lexical.number.amiss.leading_zero"));
        }
    }
    Ok(())
}

/// Keep the scanner cursor when a text operation cannot produce tokens.
pub fn scan_position(source: &str, table: &Table) -> Result<Vec<Token>, (String, u32, usize)> {
    scan_with_gate(source, table, None)
}

pub fn scan_with_gate<'a>(source: &str, table: &'a Table, gate: Option<NameGate<'a>>) -> Result<Vec<Token>, (String, u32, usize)> {
    if source.contains('\0') && table.syntax_exceptions { return Err(("SyntaxError: source code string cannot contain null bytes".to_owned(), 0, 0)); }
    if table.scanner.flag_ext_lexical_template {
        return scan_at(source, table).map_err(|(message, row)| (message, row, 1));
    }
    let mut cursor = (1, 1);
    let mut read = scan_code_from(source, table, 1, &mut cursor, gate, &mut false).map_err(|message| (message, cursor.0, cursor.1))?;
    read.retain(|item| item.shape != Shape::EscapeNotice);
    Ok(read)
}


fn failed_digit(spelling: &str, complaint: &str) -> usize {
    if complaint.contains("leading zeros") { return 0; }
    let chars = spelling.chars().collect::<Vec<_>>();
    let (base, skip) = match chars.get(..2) {
        Some(['0', 'b' | 'B']) => (2, 2),
        Some(['0', 'o' | 'O']) => (8, 2),
        Some(['0', 'x' | 'X']) => (16, 2),
        _ => (10, 0),
    };
    let mut index = skip;
    while index < chars.len() {
        let c = chars[index];
        let next = chars.get(index + 1);
        if c == '_' {
            if !next.map_or(false, |n| n.is_digit(base) || base < 10 && n.is_ascii_digit()) { return index; }
        } else if !c.is_digit(base) {
            if skip != 0 {
                return if base != 16 && c.is_ascii_digit() { index } else { index.saturating_sub(1) };
            }
            let exponent = matches!(c, 'e' | 'E') && next.map_or(false, |n| n.is_ascii_digit() || matches!(n, '+' | '-'));
            if !exponent && !matches!(c, '.' | '+' | '-') { return index.saturating_sub(1); }
        }
        index += 1;
    }
    index.saturating_sub(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Keep template markers and statement endings consistent after request settings clear labels.
    #[test]
    fn cleared_labels_refresh_scanner_metadata() {
        let mut table = Table::parse(include_str!("../../../langs/php.json")).unwrap();
        let source = "<? echo 7; ?>";
        assert!(scan(source, &table).unwrap().iter().any(|token| token.shape == Shape::Numeral));
        table.put_by("ext.lexical.prologue.brief");
        let tokens = scan(source, &table).unwrap();
        assert!(tokens.iter().any(|token| token.shape == Shape::Quote && token.lexeme == source));
        for source in ["<?php echo 7; ?>", "<?= 7 ?>"] {
            assert!(scan(source, &table).unwrap().iter().any(|token| token.shape == Shape::Numeral));
        }
        table.put_by("ext.lexical.prologue.echo");
        assert!(scan("<?= 7 ?>", &table).unwrap().iter().any(|token| token.shape == Shape::Quote && token.lexeme == "<?= 7 ?>"));
        assert!(table.separates(";"));
        table.put_by("stmt.terminator");
        assert!(!table.separates(";"));
    }

    // Keep notices aligned with the numbers the scanner actually consumes.
    #[test]
    fn numeric_notices_follow_token_boundaries() {
        let table = Table::parse(include_str!("../../../langs/python.json")).unwrap();
        for (source, number, kind) in [
            ("1e-0or 2", "1e-0", "decimal"),
            ("1e+0or 2", "1e+0", "decimal"),
            ("1or...", "1", "decimal"),
            ("...5and 2", "5", "decimal"),
            (".5and 2", ".5", "decimal"),
            (".5if True else 0", ".5", "decimal"),
            ("0b1and 2", "0b1", "binary"),
            ("0x1for x in []", "0x1f", "hexadecimal"),
            ("1jif 1 else 2", "1j", "imaginary"),
        ] {
            assert_eq!(escape_notices(source, &table), vec![(format!("invalid {kind} literal"), 1)], "{source}");
            let tokens = scan_at(source, &table).unwrap();
            let first_number = tokens.iter().find(|token| token.shape == Shape::Numeral).unwrap();
            assert_eq!(first_number.lexeme, number, "{source}");
        }
        for (source, row) in [("f'{1e-0or 2}'", 1), ("f'{1or...}'", 1), ("f'''
{1e-0or 2}
'''", 2)] {
            assert_eq!(escape_notices(source, &table), vec![("invalid decimal literal".to_owned(), row)], "{source}");
        }
        for source in ["'1e-0or 2'", "# 1or...", "name1or", "é1or = 2", "α1or = 2", "1e-0 or 2"] {
            assert!(escape_notices(source, &table).is_empty(), "{source}");
        }
    }
}

// Store scanner records, including indentation and source extents, in native images.
pub(crate) fn save_tokens(words: &[Token]) -> String {
    serde_json::json!(words.iter().map(|word| (
        word.shape as u8, &word.lexeme, &word.raw, word.span, word.row,
        word.column, word.end_column, word.end_row, word.utf8_column, word.utf8_end
    )).collect::<Vec<_>>()).to_string()
}

pub(crate) fn restore_tokens(serialized: &str) -> Result<Vec<Token>, String> {
    let malformed = || String::from("ValueError: invalid native token image");
    let json: serde_json::Value = serde_json::from_str(serialized).map_err(|_| malformed())?;
    let tags = [Shape::Bare, Shape::Quoted, Shape::Numeral, Shape::Quote, Shape::ByteQuote, Shape::Woven, Shape::WovenEnd, Shape::Field, Shape::FieldText, Shape::Unheld, Shape::CharacterRow, Shape::EscapeNotice, Shape::Sign, Shape::LineEnd, Shape::Lead, Shape::Open, Shape::Close, Shape::Finish];
    let mut words = Vec::new();
    for item in json.as_array().ok_or_else(malformed)? {
        let parts = item.as_array().ok_or_else(malformed)?;
        if parts.len() != 10 { return Err(malformed()); }
        let integer = |slot: usize| -> Result<usize, String> {
            parts[slot].as_u64().and_then(|value| value.try_into().ok()).ok_or_else(malformed)
        };
        let tag = integer(0)?;
        let spelling = parts[1].as_str().ok_or_else(malformed)?.to_owned();
        let original = match &parts[2] {
            serde_json::Value::Null => None,
            other => Some(other.as_str().ok_or_else(malformed)?.to_owned()),
        };
        words.push(Token { shape: *tags.get(tag).ok_or_else(malformed)?, lexeme: spelling, raw: original,
            span: integer(3)?, row: integer(4)?.try_into().map_err(|_| malformed())?,
            utf8_column: integer(8)?, utf8_end: integer(9)?, column: integer(5)?, end_column: integer(6)?, end_row: integer(7)?.try_into().map_err(|_| malformed())? });
    }
    if words.last().map_or(true, |last| last.shape != Shape::Finish) { return Err(malformed()); }
    Ok(words)
}
