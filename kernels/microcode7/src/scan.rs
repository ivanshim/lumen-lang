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
    Woven,
    WovenEnd,
    Field,
    Unheld,
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
    let any_case = table.flag("ext.lexical.prologue.folded");
    if let Some(p) = table.single("lexical.prologue") {
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
    if let Some(cut) = table.strings("ext.lexical.epilogue").iter().find_map(|e| text.trim_end().strip_suffix(e.as_str())) {
        text = cut;
    }
    if !table.strings("ext.lexical.string.long").is_empty() { return text.to_string(); }
    let lines = table.strings("lexical.comment_line");
    let opens = table.strings("lexical.comment_block.open");
    let closes = table.strings("lexical.comment_block.close");
    let quotes = table.letters("lexical.string_quotes");
    let folding = table.single("ext.lexical.heredoc");
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
        } else if let Some(delimiter) = table.strings("ext.lexical.string.long").iter().find(|d| ahead.starts_with(d.as_str())) {
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
    let opened = text.strip_prefix(table.single("ext.lexical.heredoc")?)?.trim_start_matches([' ', '\t']);
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
    let lines = table.strings("lexical.comment_line");
    let opens = table.strings("lexical.comment_block.open");
    let shuts = table.strings("lexical.comment_block.close");
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
    if !table.flag("ext.lexical.template") {
        return scan_code_marking(source, table, 1);
    }
    let opening = table.single("lexical.prologue").ok_or_else(|| ("A template needs lexical.prologue".to_string(), 0))?;
    let closing = table.single("ext.lexical.epilogue");
    let telling = table
        .prims
        .iter()
        .find(|(_, op)| **op == crate::form::Prim::Tell)
        .map(|(word, _)| word.clone())
        .ok_or_else(|| ("A template needs a builtin that writes what it is given".to_string(), 0))?;
    let ending = table.single("stmt.terminator").unwrap_or(";").to_string();
    let mut out: Vec<Token> = Vec::new();
    let says = |text: &str, row: u32, out: &mut Vec<Token>| {
        if text.is_empty() {
            return;
        }
        out.push(Token { shape: Shape::Bare, lexeme: telling.clone(), span: 0, row });
        out.push(Token { shape: Shape::Quote, lexeme: text.to_string(), span: 0, row });
        out.push(Token { shape: Shape::Sign, lexeme: ending.clone(), span: 0, row });
    };
    let mut rest = source;
    // The rows of the page are counted through the weave, so that what
    // a run of code says of itself names the page's own lines.
    let mut row: u32 = 1;
    let briefly = table.single("ext.lexical.prologue.echo");
    let folded = table.flag("ext.lexical.prologue.folded");
    // A shorter marker still, which the run may or may not be given.
    let shortly = table.single("ext.lexical.prologue.brief");
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
            out.push(Token { shape: Shape::Bare, lexeme: telling.clone(), span: 0, row });
        }
        out.append(&mut inside);
        // Each run of code stands as a statement, however it ended.
        out.push(Token { shape: Shape::Sign, lexeme: ending.clone(), span: 0, row: ended });
        row += code.matches('\n').count() as u32;
        // One line end straight after the closing marker belongs to it.
        let shorter = tail.strip_prefix('\n').unwrap_or_else(|| tail.strip_prefix("\r\n").unwrap_or(tail));
        if shorter.len() != tail.len() {
            row += 1;
        }
        rest = shorter;
    }
    says(rest, row, &mut out);
    out.push(Token { shape: Shape::Finish, lexeme: "EOF".into(), span: 0, row });
    Ok(out)
}

fn scan_code(source: &str, table: &Table) -> Result<Vec<Token>, String> {
    let mut ended = 1;
    scan_code_from(source, table, 1, &mut ended)
}

/// The same, begun at a given row, so that a run of code woven into a
/// page names the rows of the page and not its own.
fn scan_code_marking(source: &str, table: &Table, first: u32) -> Result<Vec<Token>, (String, u32)> {
    let mut ended = first;
    scan_code_from(source, table, first, &mut ended).map_err(|said| (said, ended))
}

/// The marks and letters opening a string, found before names are cut.
fn quoted_start(src: &[char], offset: usize, table: &Table) -> Option<(usize, Vec<char>, bool, bool)> {
    let quotes = table.letters("lexical.string_quotes");
    let mut flags = 0u8;
    let mut begin = offset;
    while !quotes.contains(src.get(begin)?) {
        if begin - offset >= 2 { return None; }
        let letter = src[begin].to_string();
        let kind = ["raw", "bytes", "plain", "format"].iter().position(|part|
            table.spells(&format!("ext.lexical.string.prefix.{part}"), &letter))?;
        let bit = 1 << kind;
        if flags & bit != 0 { return None; }
        flags |= bit;
        begin += 1;
    }
    if begin - offset == 2 && flags != 3 && flags != 9 { return None; }
    let ending = table.strings("ext.lexical.string.long").iter()
        .map(|s| s.chars().collect::<Vec<_>>()).filter(|s| src[begin..].starts_with(s))
        .max_by_key(Vec::len).unwrap_or_else(|| vec![src[begin]]);
    Some((begin + ending.len(), ending, flags & 1 != 0, flags & 8 != 0))
}

struct Quotation<'a> {
    source: &'a [char],
    next: usize,
    table: &'a Table,
    row: u32,
    made: Vec<Token>,
}

impl Quotation<'_> {
    fn here(&self) -> Option<char> { self.source.get(self.next).copied() }
    fn forward(&mut self, count: usize) {
        let stop = (self.next + count).min(self.source.len());
        self.row += self.source[self.next..stop].iter().filter(|&&c| c == '\n').count() as u32;
        self.next = stop;
    }
    fn bad(&self) -> String {
        self.table.single("ext.lexical.string.amiss").unwrap_or("Invalid string literal").to_owned()
    }
    fn token(&mut self, kind: Shape, text: String) {
        self.made.push(Token { row: self.row, shape: kind, lexeme: text, span: 0 });
    }
    fn flush(&mut self, text: &mut String, missing: &mut bool) {
        let kind = if *missing { Shape::Unheld } else { Shape::Quote };
        let value = if *missing {
            self.table.single("ext.lexical.escape.unavailable").unwrap_or("Unicode escape cannot be represented").to_owned()
        } else { std::mem::take(text) };
        self.token(kind, value);
        text.clear();
        *missing = false;
    }
    fn literal(&mut self, body: usize, end: &[char], raw: bool, fields: bool) -> Result<(), String> {
        let bytes = self.source[self.next..body - end.len()].iter().any(|c|
            self.table.spells("ext.lexical.string.prefix.bytes", &c.to_string()));
        self.forward(body - self.next);
        if fields { self.token(Shape::Woven, String::new()); }
        let mut saved = String::new();
        let mut missing = false;
        while !self.source[self.next..].starts_with(end) {
            let ch = self.here().ok_or_else(|| self.bad())?;
            match ch {
                '\n' if end.len() == 1 => return Err(self.bad()),
                '\\' => self.slash(raw, fields, bytes, &mut saved, &mut missing)?,
                '{' | '}' if fields => {
                    if self.source.get(self.next + 1) == Some(&ch) {
                        saved.push(ch);
                        self.forward(2);
                    } else {
                        if ch == '}' { return Err(self.bad()); }
                        self.flush(&mut saved, &mut missing);
                        self.field(raw)?;
                    }
                }
                _ => { saved.push(ch); self.forward(1); }
            }
        }
        self.forward(end.len());
        self.flush(&mut saved, &mut missing);
        if fields { self.token(Shape::WovenEnd, String::new()); }
        Ok(())
    }
    fn slash(&mut self, raw: bool, fields: bool, bytes: bool, text: &mut String, missing: &mut bool) -> Result<(), String> {
        let begin = self.next;
        let ch = *self.source.get(begin + 1).ok_or_else(|| self.bad())?;
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
        if table.spells("ext.lexical.escape.named", &letter) && self.source.get(begin + 2) == Some(&'{') {
            self.forward(3);
            while self.here().map_or(false, |c| c != '}') { self.forward(1); }
            if self.here().is_none() { return Err(self.bad()); }
            self.forward(1);
            *missing = true;
            return Ok(());
        }
        let count = ["ext.lexical.escape.codepoint", "ext.lexical.escape.codepoint.wide"].iter()
            .find(|key| table.spells(key, &letter)).and_then(|key| table.count(&format!("{key}.digits")));
        if let Some(count) = count {
            let amiss = table.single("ext.lexical.escape.codepoint.amiss").unwrap_or("Invalid Unicode escape");
            let beyond = table.single("ext.lexical.escape.codepoint.beyond").unwrap_or("Unicode code point out of range");
            let mut value = 0u32;
            for place in begin + 2..begin + 2 + count {
                let digit = self.source.get(place).and_then(|c| c.to_digit(16)).ok_or_else(|| amiss.to_owned())?;
                value = value.checked_mul(16).and_then(|n| n.checked_add(digit)).ok_or_else(|| beyond.to_owned())?;
            }
            if value > 0x10ffff { return Err(beyond.to_owned()); }
            if let Some(ch) = char::from_u32(value) { text.push(ch); } else { *missing = true; }
            self.forward(count + 2);
            return Ok(());
        }
        if table.spells("ext.lexical.escape.byte", &letter) {
            if let Some(width) = table.count("ext.lexical.escape.byte.digits") {
                if (begin + 2..begin + 2 + width).any(|at| !self.source.get(at).map_or(false, char::is_ascii_hexdigit)) {
                    return Err(table.single("ext.lexical.escape.codepoint.amiss").unwrap_or("Invalid Unicode escape").to_owned());
                }
            }
        }
        let mut letters = table.letters("lexical.string_escapes");
        letters.extend(table.letters("ext.lexical.escape.controls"));
        if table.flag("ext.lexical.escape.continued") && (ch == '\r' || ch == '\n') {
            self.forward(2);
            if ch == '\r' && self.here() == Some('\n') { self.forward(1); }
            return Ok(());
        }
        // A wide numbered alphabet keeps all three octal figures.
        if !bytes && table.flag("ext.lexical.escape.octal") && table.count("ext.lexical.escape.codepoint.digits").is_some() && ch.is_digit(8) {
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
            unbracketed: table.letter("ext.lexical.escape.byte"), eights: table.flag("ext.lexical.escape.octal"),
            in_bytes: table.flag("ext.system.text.bytes"), open: table.letter("ext.lexical.escape.codepoint.open"),
            shut: table.letter("ext.lexical.escape.codepoint.close"),
            amiss: table.single("ext.lexical.escape.codepoint.amiss").unwrap_or("Invalid Unicode escape"),
            beyond: table.single("ext.lexical.escape.codepoint.beyond").unwrap_or("Unicode code point out of range"), sigil: None,
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
            let comment = self.table.strings("lexical.comment_line").iter().any(|word|
                self.source[self.next..].starts_with(&word.chars().collect::<Vec<_>>()));
            if !comment { break; }
            while self.here().map_or(false, |c| c != '\n') { self.forward(1); }
        }
        whitespace
    }
    fn field(&mut self, raw: bool) -> Result<(), String> {
        self.forward(1);
        let origin = self.next;
        let mut nesting = Vec::new();
        let mut comments = Vec::new();
        loop {
            let ch = self.here().ok_or_else(|| self.bad())?;
            if let Some((body, end, bare, woven)) = quoted_start(self.source, self.next, self.table) {
                let previous = self.made.len();
                self.literal(body, &end, bare, woven)?;
                self.made.truncate(previous);
                continue;
            }
            if self.table.strings("lexical.comment_line").iter().any(|s| self.source[self.next..].starts_with(&s.chars().collect::<Vec<_>>())) {
                let begins = self.next;
                while self.here().map_or(false, |c| c != '\n') { self.forward(1); }
                comments.push(begins..self.next);
                continue;
            }
            let after = self.source.get(self.next + 1).copied();
            let before = self.next.checked_sub(1).and_then(|i| self.source.get(i)).copied();
            if nesting.is_empty() {
                let equals = ch == '=' && after != Some('=') && !matches!(before, Some('=' | '!' | '<' | '>'));
                if equals || ch == ':' || ch == '}' || ch == '!' && after != Some('=') { break; }
            }
            if let Some(index) = ['(', '[', '{'].iter().position(|&c| c == ch) {
                nesting.push([')', ']', '}'][index]);
            } else if [')', ']', '}'].contains(&ch) && nesting.pop() != Some(ch) { return Err(self.bad()); }
            self.forward(1);
        }
        let code: String = (origin..self.next).filter(|i| !comments.iter().any(|r| r.contains(i))).map(|i| self.source[i]).collect();
        if code.trim().is_empty() { return Err(self.bad()); }
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
            let ch = self.here().filter(|c| ['a', 'r', 's'].contains(c)).ok_or_else(|| self.bad())?;
            self.forward(1);
            self.between_field_marks();
            ch.to_string()
        } else if debugging && self.here() != Some(':') { "r".to_owned() } else { String::new() };
        self.token(Shape::Field, convert);
        let left = self.table.single("syntax.group.open").ok_or_else(|| self.bad())?.to_owned();
        let right = self.table.single("syntax.group.close").ok_or_else(|| self.bad())?.to_owned();
        self.token(Shape::Sign, left);
        let tokens = scan_code(&code, self.table)?;
        self.made.extend(tokens.into_iter().filter(|t| !matches!(t.shape, Shape::Lead | Shape::LineEnd | Shape::Finish)));
        self.token(Shape::Sign, right);
        self.token(Shape::Woven, String::new());
        let mut specification = String::new();
        let mut missing = false;
        if self.here() == Some(':') {
            self.forward(1);
            while self.here() != Some('}') {
                match self.here().ok_or_else(|| self.bad())? {
                    '{' => { self.flush(&mut specification, &mut missing); self.field(raw)?; }
                    '\\' => self.slash(raw, true, false, &mut specification, &mut missing)?,
                    c => { specification.push(c); self.forward(1); }
                }
            }
        }
        if self.here() != Some('}') { return Err(self.bad()); }
        self.forward(1);
        self.flush(&mut specification, &mut missing);
        self.token(Shape::WovenEnd, String::new());
        Ok(())
    }
}

fn scan_code_from(source: &str, table: &Table, first: u32, ended: &mut u32) -> Result<Vec<Token>, String> {
    let text = drop_comments(source, table);
    let src: Vec<char> = text.chars().collect();
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
    let badly = table.single("ext.lexical.escape.codepoint.amiss").unwrap_or("Bad character number");
    let too_far = table.single("ext.lexical.escape.codepoint.beyond").unwrap_or("Character number too large");
    // The letter beginning a character named by a bare number instead.
    let unbracketed = table.letter("ext.lexical.escape.byte");
    // Whether figures in eights after a backslash name a character too.
    let eights = table.flag("ext.lexical.escape.octal");
    // Whether what is kept is bytes rather than the letters they spell.
    let in_bytes = table.flag("ext.system.text.bytes");
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
    let folding = table.single("ext.lexical.heredoc");
    // The escapes a folded string reads: those of a quoted one, less the
    // quote marks, which have nothing to be shielded from there.
    let unquoted: Vec<char> = escapes.iter().copied().filter(|e| !quotes.contains(e)).collect();
    let quote_name = table.letter("lexical.name_quote");
    let unit = table.count("block.indent_size").unwrap_or(4);
    let fold_kw = table.flag("lexical.keywords_case_insensitive");
    let fold_id = table.flag("identifier.case_insensitive");
    let mut tokens: Vec<Token> = Vec::new();
    let tok = |kind: Shape, text: String, row: u32| Token { shape: kind, lexeme: text, span: 0, row: row };
    let (mut pos, mut row, mut at_bol) = (0usize, first, true);
    while pos < src.len() {
        // The row the reading has reached is kept where the caller can
        // see it, so that a reading that stops names the right line.
        *ended = row;
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
        if !table.strings("ext.lexical.string.long").is_empty() {
            if table.strings("lexical.comment_line").iter().any(|m| src[pos..].starts_with(&m.chars().collect::<Vec<_>>())) {
                while pos < src.len() && src[pos] != '\n' { pos += 1; }
                continue;
            }
            if let Some((body, end, raw, fields)) = quoted_start(&src, pos, table) {
                let mut quote = Quotation { source: &src, next: pos, row, table, made: Vec::new() };
                quote.literal(body, &end, raw, fields)?;
                pos = quote.next;
                row = quote.row;
                tokens.extend(quote.made);
                continue;
            }
            if table.flag("ext.lexical.string.adjacent") && src[pos] == '\\' && src.get(pos + 1) == Some(&'\n') {
                pos += 2;
                row += 1;
                continue;
            }
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
            let delimiter = table.strings("ext.lexical.string.long").iter().find(|word| {
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
            return Err(told_of_character(table, c, row));
        };
        pos += sym.chars().count();
        tokens.push(tok(Shape::Sign, sym, row));
    }
    if table.flag("ext.syntax.call.bind_names") {
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
            if let (Some(shut), Some(opener)) = (table.single("op.index.close"), table.single("op.index.open")) {
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
                            let quotes = table.strings("lexical.raw_quotes");
                            let any = table.strings("lexical.string_quotes");
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
                if let Some(mark) = table.single("ext.op.member") {
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
    let opening = table.single("ext.system.reading.unexpected")?;
    let words = table.single("ext.lexical.interpolating.index.amiss")?;
    Some(format!("{opening} {words}"))
}

/// What a language says of a character it has no reading for at all.
/// A language naming such characters by their number names it that
/// way, a character no one can show being no use in a complaint.
fn told_of_character(table: &Table, c: char, row: u32) -> String {
    match (table.single("ext.system.reading.unexpected"), table.single("ext.system.reading.unexpected.character")) {
        (Some(opening), Some(named)) => format!("{opening} {named}{:02X}", c as u32),
        _ => format!("Unexpected character '{c}' at row {row}"),
    }
}

/// The tokens of a woven string: a bracketed concatenation starting
/// from the empty string, so the result is text whatever is woven in.
fn weave(s: &str, plain: &[usize], table: &Table, row: u32, tokens: &mut Vec<Token>) -> Result<(), String> {
    let cut = pieces(s, plain, table)?;
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
