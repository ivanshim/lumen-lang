// Scanning: text to tokens, by the definition's spellings. The prologue
// and comments go first, outside strings, line ends kept. Then each token
// is a string, a number, a quoted name, a word, a symbol, a line end or
// the indentation opening a line.

use crate::lang::Lang;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    Instr,
    /// A name between name quotes: data for the word after it.
    Quoted,
    Numeral,
    Quote,
    StringBegin,
    StringEnd,
    StringField,
    StringFault,
    Sign,
    LineEnd,
    /// The indentation of a line that has something on it.
    Lead,
    /// Block boundaries the shaping pass adds for an indented language.
    Open,
    Close,
    Finish,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub shape: Shape,
    pub lexeme: String,
    pub width: usize,
    pub row: usize,
    pub column: usize,
}

impl Token {
    pub fn is_lexeme(&self, shape: Shape, text: &str) -> bool {
        self.shape == shape && self.lexeme == text
    }
}

/// Where a marker stands in some text, found however it is written
/// where the language says its markers are known that way. Markers are
/// written in letters that have a case, so lowering them leaves every
/// place in the text where it was.
fn marker_at(text: &str, marker: &str, folded: bool) -> Option<usize> {
    match folded {
        true => text.to_ascii_lowercase().find(&marker.to_ascii_lowercase()),
        false => text.find(marker),
    }
}

fn drop_prologue<'a>(source: &'a str, lang: &Lang) -> &'a str {
    let Some(prologue) = &lang.prologue else { return source };
    // An import the reader knows must reach it whole, even where the
    // old prologue named that same import.
    if prologue.split_whitespace().next().map_or(false, |word| Lang::spells(&lang.import_words, word)) {
        return source;
    }
    let lead = source.len() - source.trim_start().len();
    let opens = match lang.prologue_folded {
        true => source[lead..].to_ascii_lowercase().starts_with(&prologue.to_ascii_lowercase()),
        false => source[lead..].starts_with(prologue.as_str()),
    };
    if !source[..lead].contains('\n') && opens {
        &source[lead + prologue.len()..]
    } else {
        source
    }
}

/// A closing marker at the very end (ext.lexical.epilogue) is dropped.
fn drop_epilogue<'a>(source: &'a str, lang: &Lang) -> &'a str {
    let body = source.trim_end();
    for word in &lang.epilogue {
        if let Some(kept) = body.strip_suffix(word.as_str()) {
            return kept;
        }
    }
    source
}

fn drop_comments(source: &str, lang: &Lang) -> String {
    if !lang.long_quotes.is_empty() { return source.to_string(); }
    if lang.line_comments.is_empty() && lang.block_comments.is_empty() {
        return source.to_string();
    }
    let mut kept = String::with_capacity(source.len());
    let mut ahead = source;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    while let Some(c) = ahead.chars().next() {
        let w = c.len_utf8();
        if quote.is_none() {
            if let Some(mark) = lang.long_quotes.iter().find(|mark| ahead.starts_with(mark.as_str())) {
                let mut reach = mark.len();
                while reach < ahead.len() {
                    let rest = &ahead[reach..];
                    if rest.starts_with(mark.as_str()) { reach += mark.len(); break; }
                    let ch = rest.chars().next().expect("text remains");
                    reach += ch.len_utf8();
                    if ch == '\\' {
                        if let Some(next) = ahead[reach..].chars().next() { reach += next.len_utf8(); }
                    }
                }
                kept.push_str(&ahead[..reach]);
                ahead = &ahead[reach..];
                continue;
            }
        }
        match quote {
            Some(opener) => {
                kept.push(c);
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == opener {
                    quote = None;
                }
                ahead = &ahead[w..];
            }
            None if lang.heredoc.as_deref().map_or(false, |mark| ahead.starts_with(mark)) => {
                // A string written over lines says what it spells,
                // comment marks and quote marks and all, so the whole
                // of it is carried over untouched. One left unclosed is
                // carried over whole too, for the scanner to speak of.
                let over = heredoc_at(ahead, lang).map_or(ahead.len(), |here| here.done);
                kept.push_str(&ahead[..over]);
                ahead = &ahead[over..];
            }
            None if lang.quotes.contains(&c) => {
                quote = Some(c);
                kept.push(c);
                ahead = &ahead[w..];
            }
            None => {
                if let Some((open, close)) = lang.block_comments.iter().find(|(o, _)| ahead.starts_with(o.as_str())) {
                    let body = &ahead[open.len()..];
                    let end = body.find(close.as_str()).map_or(body.len(), |at| at + close.len());
                    kept.extend(body[..end].chars().filter(|c| *c == '\n'));
                    ahead = &body[end..];
                } else if lang.line_comments.iter().any(|m| ahead.starts_with(m.as_str())) {
                    if lang.line_continuations.iter().any(|mark| kept.ends_with(mark.as_str())) {
                        kept.push(' ');
                    }
                    ahead = ahead.find('\n').map_or("", |at| &ahead[at..]);
                } else {
                    kept.push(c);
                    ahead = &ahead[w..];
                }
            }
        }
    }
    kept
}

/// What a backslash may do in a run of text: the letters that stand for
/// characters of their own, the mark that closes the run — which may
/// always be written escaped — whether a character may be named by its
/// number, and whether an escaped sigil is a sigil and opens no name.
struct Escapes<'a> {
    letters: &'a [char],
    quote: Option<char>,
    numbered: bool,
    woven: bool,
}

/// Where the parts of a string written over lines lie, counted in bytes
/// from the mark that opened it: the body, the place the body ends,
/// which is before the line end the closing label's line begins after,
/// and the place past that label, where the reading goes on.
struct Heredoc {
    raw: bool,
    indent: usize,
    body: usize,
    ended: usize,
    done: usize,
}

/// The string a mark opens where this text begins, if one is opened and
/// its label stands alone again further down. The label is a name, or a
/// name in quotes; quotes the language calls raw make a body that says
/// what it spells and no more.
fn heredoc_at(text: &str, lang: &Lang) -> Option<Heredoc> {
    let blank = |s: &str| s.len() - s.trim_start_matches([' ', '\t']).len();
    let mark = lang.heredoc.as_deref()?;
    if !text.starts_with(mark) {
        return None;
    }
    let mut at = mark.len() + blank(&text[mark.len()..]);
    let quote = text[at..].chars().next().filter(|c| lang.quotes.contains(c));
    at += quote.map_or(0, char::len_utf8);
    let from = at;
    for (step, c) in text[from..].char_indices() {
        let fits = match step {
            0 => lang.begins_name(c),
            _ => lang.extends_name(c),
        };
        if !fits {
            break;
        }
        at = from + step + c.len_utf8();
    }
    let label = &text[from..at];
    if label.is_empty() {
        return None;
    }
    if let Some(q) = quote {
        if !text[at..].starts_with(q) {
            return None;
        }
        at += q.len_utf8();
    }
    at += blank(&text[at..]);
    let after = text[at..].strip_prefix("\r\n").or_else(|| text[at..].strip_prefix('\n'))?;
    let body = text.len() - after.len();
    let mut line = body;
    loop {
        let indent = blank(&text[line..]);
        let word = &text[line + indent..];
        // The label ends the body where nothing goes on from it: a
        // longer word that merely begins with it is a word of the body.
        if word.starts_with(label) && word[label.len()..].chars().next().map_or(true, |c| !lang.extends_name(c)) {
            let over = &text[body..line];
            let ended = over.strip_suffix('\n').map_or(over, |cut| cut.strip_suffix('\r').unwrap_or(cut));
            let done = line + indent + label.len();
            let raw = quote.map_or(false, |q| lang.raw_quotes.contains(&q));
            return Some(Heredoc { raw, indent, body, ended: body + ended.len(), done });
        }
        line += text[line..].find('\n')? + 1;
    }
}

struct Cursor<'a> {
    lang: &'a Lang,
    text: Vec<char>,
    at: usize,
    row: usize,
    column: usize,
    out: Vec<Token>,
}

impl<'a> Cursor<'a> {
    fn look(&self, ahead: usize) -> Option<char> {
        self.text.get(self.at + ahead).copied()
    }

    fn step(&mut self) -> char {
        let c = self.text[self.at];
        self.at += 1;
        if c == '\n' {
            self.row += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        c
    }

    fn push(&mut self, shape: Shape, lexeme: String, width: usize, row: usize, column: usize) {
        self.out.push(Token { shape, lexeme, width, row, column });
    }

    /// The indentation of a line; a blank line is skipped whole. Returns
    /// whether the line was blank.
    fn indentation(&mut self) -> bool {
        let mut width = 0;
        let mut j = self.at;
        while let Some(&c) = self.text.get(j) {
            width += match c {
                ' ' => 1,
                '\t' => self.lang.indent_width,
                _ => break,
            };
            j += 1;
        }
        let end = self.text[j..].iter().position(|c| *c == '\n').map_or(self.text.len(), |p| j + p);
        if self.text[j..end].iter().all(|c| c.is_whitespace())
            || self.lang.line_comments.iter().any(|mark| at_word(&self.text, j, mark)) {
            while self.at < end {
                self.step();
            }
            if self.at < self.text.len() {
                self.step();
            }
            return true;
        }
        let (line, col) = (self.row, self.column);
        self.push(Shape::Lead, String::new(), width, line, col);
        while self.at < j {
            self.step();
        }
        false
    }

    /// The character named by the bare number after the escape letter,
    /// read in sixteens, two digits at most. Nothing at all where no
    /// digit follows, since then the letter names no character and is
    /// kept as written.
    fn numbered_bare(&mut self) -> Option<char> {
        let mut number = 0u32;
        let mut digits = 0;
        while digits < 2 {
            let Some(c) = self.look(0).filter(char::is_ascii_hexdigit) else { break };
            number = number * 16 + c.to_digit(16).expect("a digit in sixteens");
            digits += 1;
            self.step();
        }
        match digits {
            0 => None,
            _ => char::from_u32(number),
        }
    }

    /// The character an escape names by its number: the opening
    /// bracket is where this begins, the number is written in sixteens,
    /// and the closing bracket ends it. Nothing where the number names
    /// no character of its own — half of a pair standing for one
    /// character between them is a number without a character, and a
    /// kernel whose text is made of characters cannot hold it, so the
    /// escape is left as it was written.
    fn codepoint(&mut self) -> Result<(u32, Option<char>, String), String> {
        let amiss = || self.lang.codepoint_amiss.clone().unwrap_or_else(|| "Bad character number".to_string());
        let open = self.lang.codepoint_open.expect("the escape has brackets");
        let close = self.lang.codepoint_close.ok_or_else(amiss)?;
        let mut written = String::from(open);
        self.step();
        let mut digits = String::new();
        loop {
            let Some(c) = self.look(0) else { return Err(amiss()) };
            written.push(c);
            if c == close {
                self.step();
                break;
            }
            if !c.is_ascii_hexdigit() {
                return Err(amiss());
            }
            digits.push(c);
            self.step();
        }
        if digits.is_empty() {
            return Err(amiss());
        }
        // A number of any length may be written, leading noughts and
        // all, so one too long to hold is one beyond the last character.
        let beyond = || self.lang.codepoint_beyond.clone().unwrap_or_else(|| "Character number too large".to_string());
        let Ok(number) = u32::from_str_radix(digits.trim_start_matches('0'), 16).or_else(|_| match digits.chars().all(|d| d == '0') {
            true => Ok(0),
            false => Err(()),
        }) else {
            return Err(beyond());
        };
        if number > 0x10FFFF {
            return Err(beyond());
        }
        Ok((number, char::from_u32(number), written))
    }

    /// One escape, from the backslash to the end of what it names: the
    /// character it stands for goes into the text and the reading goes
    /// on after it. A letter the language says nothing of keeps its
    /// backslash, since text nobody spoke for is text as it was written.
    fn escape(&mut self, how: &Escapes, s: &mut String, shielded: &mut Vec<usize>) -> Result<(), String> {
        self.step();
        let next = self.step();
        if how.numbered && self.lang.line_continuations.iter().any(|mark| mark == "\\") {
            if next == '\n' {
                return Ok(());
            }
            if next == '\r' && self.look(0) == Some('\n') {
                self.step();
                return Ok(());
            }
        }
        if how.woven && Some(next) == self.lang.sigil {
            // An escaped sigil is just the sigil.
            shielded.push(s.chars().count());
            s.push(next);
            return Ok(());
        }
        // A character named by its number: the letter, the number
        // written in sixteens between its brackets, and the character
        // of that number in its place.
        if how.numbered && Some(next) == self.lang.codepoint_letter && self.look(0) == self.lang.codepoint_open {
            let (number, made, written) = self.codepoint()?;
            // Where text is bytes, what the number names is written out
            // in the bytes that spell it, and a number naming half of a
            // pair is spelled the same way as any other, since the text
            // is bytes and no letter need answer to it.
            if self.lang.text_is_bytes {
                for byte in spelled_bytes(number) {
                    shielded.push(s.chars().count());
                    s.push(char::from(byte));
                }
                return Ok(());
            }
            match made {
                Some(made) => {
                    shielded.push(s.chars().count());
                    s.push(made);
                }
                None => {
                    s.push('\\');
                    s.push(next);
                    s.push_str(&written);
                }
            }
            return Ok(());
        }
        // A character named by a run of figures in eights, up to
        // three of them, the backslash itself beginning the run. A
        // number past the widest a character of one byte holds is
        // taken by its low eight bits, as the reference takes it.
        if how.numbered && self.lang.octal_escapes && next.is_digit(8) {
            let mut number = next.to_digit(8).expect("a figure in eights");
            let mut figures = 1;
            while figures < 3 {
                let Some(c) = self.look(0).and_then(|c| c.to_digit(8)) else { break };
                number = number * 8 + c;
                figures += 1;
                self.step();
            }
            shielded.push(s.chars().count());
            s.push(char::from_u32(if self.lang.codepoint_digits.is_some() { number } else { number & 0xFF }).expect("a character numbered in eights"));
            return Ok(());
        }
        // The same by number, but written bare: one figure in sixteens
        // or two, with no brackets about them. A letter with no figure
        // after it names no character and stands for itself.
        if how.numbered && Some(next) == self.lang.byte_letter {
            match self.numbered_bare() {
                Some(made) => {
                    shielded.push(s.chars().count());
                    s.push(made);
                }
                None => {
                    s.push('\\');
                    s.push(next);
                }
            }
            return Ok(());
        }
        if next == '\\' || Some(next) == how.quote || how.letters.contains(&next) {
            s.push(match next {
                'n' => '\n',
                't' => '\t',
                'r' => '\r',
                '0' => '\0',
                'a' => '\u{7}',
                'b' => '\u{8}',
                'f' => '\u{c}',
                'v' => '\u{b}',
                c => c,
            });
        } else {
            s.push('\\');
            s.push(next);
        }
        Ok(())
    }

    fn string_open(&self) -> Option<(usize, String, bool, bool)> {
        let lang = self.lang;
        let mut count = 0;
        let (mut raw, mut bytes, mut plain, mut format) = (false, false, false, false);
        while let Some(c) = self.look(count) {
            if lang.quotes.contains(&c) { break; }
            if count == 2 { return None; }
            if lang.raw_prefixes.contains(&c) && !raw { raw = true; }
            else if lang.byte_prefixes.contains(&c) && !bytes { bytes = true; }
            else if lang.plain_prefixes.contains(&c) && !plain { plain = true; }
            else if lang.format_prefixes.contains(&c) && !format { format = true; }
            else { return None; }
            count += 1;
        }
        if count == 2 && (!raw || plain || bytes == format) { return None; }
        let quote = self.look(count).filter(|c| lang.quotes.contains(c))?;
        let mark = lang.long_quotes.iter().filter(|q| at_word(&self.text, self.at + count, q))
            .max_by_key(|q| q.len()).cloned().unwrap_or_else(|| quote.to_string());
        Some((count, mark, raw, format))
    }

    fn string_words(&self) -> String {
        self.lang.string_amiss.clone().unwrap_or_else(|| "Invalid string literal".into())
    }

    fn string_text(&mut self, text: &mut String, fault: &mut bool, line: usize, col: usize) {
        if *fault {
            self.push(Shape::StringFault, self.lang.escape_unavailable.clone().unwrap_or_else(|| "Unicode escape cannot be represented".into()), 0, line, col);
        } else {
            self.push(Shape::Quote, std::mem::take(text), 0, line, col);
        }
        text.clear();
        *fault = false;
    }

    /// A prefixed or long string, with each field kept apart from its
    /// text until the assembler has read the expression it holds.
    fn rich_string(&mut self, prefix: usize, mark: &str, raw: bool, format: bool) -> Result<(), String> {
        let (line, col) = (self.row, self.column);
        let bytes = self.text[self.at..self.at + prefix].iter().any(|c| self.lang.byte_prefixes.contains(c));
        for _ in 0..prefix + mark.chars().count() { self.step(); }
        if format { self.push(Shape::StringBegin, String::new(), 0, line, col); }
        let (mut text, mut fault) = (String::new(), false);
        loop {
            if at_word(&self.text, self.at, mark) {
                for _ in mark.chars() { self.step(); }
                break;
            }
            let Some(c) = self.look(0) else {
                return Err(if mark.chars().count() > 1 && !format { format!("Unterminated {} string", mark.chars().next().unwrap()) } else { self.string_words() });
            };
            if bytes && !c.is_ascii() { return Err(self.string_words()); }
            if c == '\n' && mark.chars().count() == 1 { return Err(self.string_words()); }
            if format && (c == '{' || c == '}') {
                if self.look(1) == Some(c) {
                    self.step(); self.step(); text.push(c);
                } else if c == '{' {
                    self.string_text(&mut text, &mut fault, line, col);
                    self.string_field(raw)?;
                } else { return Err(self.string_words()); }
            } else if c == '\\' {
                self.rich_escape(raw, format, bytes, &mut text, &mut fault)?;
            } else { text.push(self.step()); }
        }
        self.string_text(&mut text, &mut fault, line, col);
        if format { self.push(Shape::StringEnd, String::new(), 0, line, col); }
        Ok(())
    }

    fn rich_escape(&mut self, raw: bool, format: bool, bytes: bool, text: &mut String, fault: &mut bool) -> Result<(), String> {
        let Some(next) = self.look(1) else { return Err(self.string_words()); };
        if raw {
            text.push(self.step());
            if !format || !matches!(next, '{' | '}') { text.push(self.step()); }
            return Ok(());
        }
        let lang = self.lang;
        if bytes && [lang.named_letter, lang.codepoint_letter, lang.wide_letter].contains(&Some(next)) {
            text.push(self.step()); text.push(self.step());
            return Ok(());
        }
        if bytes && lang.octal_escapes && next.is_digit(8) {
            self.step();
            let mut number = 0;
            for _ in 0..3 {
                let Some(digit) = self.look(0).and_then(|c| c.to_digit(8)) else { break; };
                number = number * 8 + digit;
                self.step();
            }
            text.push(char::from((number & 255) as u8));
            return Ok(());
        }
        if Some(next) == lang.named_letter && self.look(2) == Some('{') {
            self.step(); self.step(); self.step();
            while self.look(0).map_or(false, |c| c != '}') { self.step(); }
            if self.look(0).is_none() { return Err(self.string_words()); }
            self.step(); *fault = true;
            return Ok(());
        }
        let digits = if Some(next) == lang.codepoint_letter { lang.codepoint_digits }
            else if Some(next) == lang.wide_letter { lang.wide_digits } else { None };
        if let Some(digits) = digits {
            self.step(); self.step();
            let mut number = 0u32;
            for _ in 0..digits {
                let d = self.look(0).and_then(|c| c.to_digit(16))
                    .ok_or_else(|| lang.codepoint_amiss.clone().unwrap_or_else(|| self.string_words()))?;
                number = number.checked_mul(16).and_then(|n| n.checked_add(d))
                    .ok_or_else(|| lang.codepoint_beyond.clone().unwrap_or_else(|| self.string_words()))?;
                self.step();
            }
            if number > 0x10ffff { return Err(lang.codepoint_beyond.clone().unwrap_or_else(|| self.string_words())); }
            match char::from_u32(number) { Some(c) => text.push(c), None => *fault = true }
            return Ok(());
        }
        if Some(next) == lang.byte_letter {
            if let Some(digits) = lang.byte_digits {
                if !(0..digits).all(|n| self.look(n + 2).map_or(false, |c| c.is_ascii_hexdigit())) {
                    return Err(lang.codepoint_amiss.clone().unwrap_or_else(|| self.string_words()));
                }
            }
        }
        if format && matches!(next, '{' | '}') { text.push(self.step()); return Ok(()); }
        if lang.continued_strings && matches!(next, '\n' | '\r') {
            self.step(); self.step();
            if next == '\r' && self.look(0) == Some('\n') { self.step(); }
            return Ok(());
        }
        let letters: Vec<char> = lang.escape_letters.iter().chain(&lang.control_escapes).copied().collect();
        let how = Escapes { letters: &letters, quote: None, numbered: true, woven: false };
        self.escape(&how, text, &mut Vec::new())
    }

    fn field_space(&mut self) -> String {
        let mut kept = String::new();
        loop {
            if self.look(0).map_or(false, |c| c.is_whitespace()) { kept.push(self.step()); }
            else if self.lang.line_comments.iter().any(|m| at_word(&self.text, self.at, m)) {
                while self.look(0).map_or(false, |c| c != '\n') { self.step(); }
            } else { break; }
        }
        kept
    }

    fn string_field(&mut self, raw: bool) -> Result<(), String> {
        let (line, col) = (self.row, self.column);
        self.step();
        let mut expression = String::new();
        let mut brackets = Vec::new();
        loop {
            let Some(c) = self.look(0) else { return Err(self.string_words()); };
            if let Some((prefix, mark, is_raw, format)) = self.string_open() {
                let saved = self.out.len();
                let began = self.at;
                self.rich_string(prefix, &mark, is_raw, format)?;
                self.out.truncate(saved);
                expression.extend(self.text[began..self.at].iter());
                continue;
            }
            if self.lang.line_comments.iter().any(|m| at_word(&self.text, self.at, m)) {
                while self.look(0).map_or(false, |x| x != '\n') { self.step(); }
                continue;
            }
            if brackets.is_empty() && (matches!(c, '}' | ':') || c == '!' && self.look(1) != Some('=')
                || c == '=' && self.look(1) != Some('=') && !matches!(self.text.get(self.at.wrapping_sub(1)), Some('!' | '<' | '>' | '='))) { break; }
            match c {
                '(' => brackets.push(')'), '[' => brackets.push(']'), '{' => brackets.push('}'),
                ')' | ']' | '}' => { if brackets.pop() != Some(c) { return Err(self.string_words()); } }
                _ => {}
            }
            expression.push(self.step());
        }
        if expression.trim().is_empty() { return Err(self.string_words()); }
        let debug = self.look(0) == Some('=');
        if debug {
            self.step();
            let shown = format!("{}={}", expression, self.field_space());
            self.push(Shape::Quote, shown, 0, line, col);
        }
        let mut conversion = String::new();
        if self.look(0) == Some('!') {
            self.step();
            let Some(c @ ('r' | 's' | 'a')) = self.look(0) else { return Err(self.string_words()); };
            conversion.push(c); self.step();
            self.field_space();
        } else if debug && self.look(0) != Some(':') { conversion.push('r'); }
        self.push(Shape::StringField, conversion, 0, line, col);
        let group = self.lang.grouping.as_ref().ok_or_else(|| self.string_words())?;
        self.push(Shape::Sign, group.open.clone(), 0, line, col);
        let mut inner = Cursor { lang: self.lang, text: expression.chars().collect(), at: 0, row: line, column: col, out: Vec::new() };
        inner.run(false)?;
        self.out.extend(inner.out.into_iter().filter(|t| !matches!(t.shape, Shape::Lead | Shape::LineEnd)));
        self.push(Shape::Sign, group.close.clone(), 0, line, col);
        self.push(Shape::StringBegin, String::new(), 0, line, col);
        let (mut text, mut fault) = (String::new(), false);
        if self.look(0) == Some(':') {
            self.step();
            loop {
                match self.look(0) {
                    Some('}') => break,
                    Some('{') => { self.string_text(&mut text, &mut fault, line, col); self.string_field(raw)?; }
                    Some('\\') => self.rich_escape(raw, true, false, &mut text, &mut fault)?,
                    Some(_) => text.push(self.step()),
                    None => return Err(self.string_words()),
                }
            }
        }
        if self.look(0) != Some('}') { return Err(self.string_words()); }
        self.step();
        self.string_text(&mut text, &mut fault, line, col);
        self.push(Shape::StringEnd, String::new(), 0, line, col);
        Ok(())
    }

    fn string(&mut self, quote: char) -> Result<(), String> {
        let (line, col) = (self.row, self.column);
        let width = self.lang.long_quotes.iter().find(|mark| {
            mark.chars().all(|ch| ch == quote)
                && mark.chars().enumerate().all(|(i, ch)| self.look(i) == Some(ch))
        }).map_or(1, |mark| mark.chars().count());
        for _ in 0..width { self.step(); }
        let raw = self.lang.raw_quotes.contains(&quote);
        let woven = self.lang.interpolating.contains(&quote);
        let how = Escapes {
            letters: match raw {
                true => &[],
                false => &self.lang.escape_letters,
            },
            quote: Some(quote),
            numbered: !raw,
            woven,
        };
        let mut s = String::new();
        // Char positions in s that came escaped: text, never code.
        let mut shielded: Vec<usize> = Vec::new();
        loop {
            let Some(c) = self.look(0) else { return Err(format!("Unterminated {} string", quote)) };
            if c == '\\' && self.look(1).is_some() {
                self.escape(&how, &mut s, &mut shielded)?;
                continue;
            }
            if c == quote && (0..width).all(|i| self.look(i) == Some(quote)) {
                for _ in 0..width { self.step(); }
                break;
            }
            self.step();
            s.push(c);
        }
        if woven {
            return self.woven(s, &shielded, line, col);
        }
        self.push(Shape::Quote, s, 0, line, col);
        Ok(())
    }

    /// A string written over lines (ext.lexical.heredoc): the mark, a
    /// label, and a body running to the line that label stands on
    /// again. What the closing label is written in front of is written
    /// in front of every line of the body and belongs to none of them,
    /// so it comes off. A label in raw quotes makes a body that stands
    /// as it is written, weaving nothing in and reading no escapes.
    fn heredoc(&mut self) -> Result<(), String> {
        let (line, col) = (self.row, self.column);
        let tail: String = self.text[self.at..].iter().collect();
        let Some(here) = heredoc_at(&tail, self.lang) else {
            return Err("Unterminated string over lines".to_string());
        };
        let far = |bytes: usize| self.at + tail[..bytes].chars().count();
        let (from, to, done) = (far(here.body), far(here.ended), far(here.done));
        while self.at < from {
            self.step();
        }
        // The quote marks are no escapes here: the body is ended by its
        // label and not by a mark, so a mark in it stands for itself.
        let letters: Vec<char> =
            self.lang.escape_letters.iter().copied().filter(|c| !self.lang.quotes.contains(c)).collect();
        let how = Escapes { letters: &letters, quote: None, numbered: true, woven: true };
        let mut s = String::new();
        let mut shielded: Vec<usize> = Vec::new();
        let mut fresh = true;
        while self.at < to {
            if fresh {
                fresh = false;
                let mut wide = here.indent;
                while wide > 0 && self.at < to && matches!(self.look(0), Some(' ') | Some('\t')) {
                    self.step();
                    wide -= 1;
                }
                continue;
            }
            let c = self.look(0).expect("the body ends where the closing label begins");
            // A backslash at the end of a line says nothing: the line
            // end after it opens a line like any other, and that line
            // gives up its indentation with the rest.
            if !here.raw && c == '\\' && self.at + 1 < to && self.look(1).map_or(false, |n| n != '\n') {
                self.escape(&how, &mut s, &mut shielded)?;
                continue;
            }
            self.step();
            s.push(c);
            fresh = c == '\n';
        }
        while self.at < done {
            self.step();
        }
        if here.raw {
            self.push(Shape::Quote, s, 0, line, col);
            return Ok(());
        }
        self.woven(s, &shielded, line, col)
    }

    /// A string that weaves values in (ext.lexical.interpolating_quotes):
    /// `$name`, `$name[i]` and `{$expr}` inside it become code, and the
    /// whole becomes a bracketed concatenation of its parts, starting
    /// from an empty string so the result is always text.
    /// Whether the mark is written at that place in the run.
    fn woven(&mut self, s: String, shielded: &[usize], line: usize, col: usize) -> Result<(), String> {
        let lang = self.lang;
        let chars: Vec<char> = s.chars().collect();
        let opens_var = |j: usize| {
            chars.get(j).copied() == lang.sigil && !shielded.contains(&j) && chars.get(j + 1).map_or(false, |n| lang.begins_name(*n))
        };
        // (is code, text)
        let mut parts: Vec<(bool, String)> = Vec::new();
        let mut text = String::new();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            if c == '{' && opens_var(i + 1) {
                let mut depth = 0;
                let mut close = None;
                for (j, ch) in chars.iter().enumerate().skip(i) {
                    match ch {
                        '{' => depth += 1,
                        '}' if depth == 1 => {
                            close = Some(j);
                            break;
                        }
                        '}' => depth -= 1,
                        _ => {}
                    }
                }
                if let Some(end) = close {
                    parts.push((false, std::mem::take(&mut text)));
                    parts.push((true, chars[i + 1..end].iter().collect()));
                    i = end + 1;
                    continue;
                }
            } else if opens_var(i) {
                let mut j = i + 1;
                while j < chars.len() && lang.extends_name(chars[j]) {
                    j += 1;
                }
                // One step past the binding is woven in as well, which
                // is as far as this shorter way of writing reaches: a
                // place named in brackets, or a member named after the
                // mark for one. Anything further wants the brackets
                // that take a whole piece of code.
                let mut written: Option<String> = None;
                let mut stepped = false;
                if let Some(index) = lang.index_brackets.clone() {
                    if at_word(&chars, j, &index.open) {
                        let from = j + index.open.chars().count();
                        if let Some(end) = word_at(&chars, from, &index.close) {
                            let inside: String = chars[from..end].iter().collect();
                            let opens = |f: fn(&Lang, char) -> bool| inside.chars().next().map_or(false, |c| f(lang, c));
                            let counted = |w: &str| !w.is_empty() && w.chars().all(|c| c.is_ascii_digit());
                            let digits = counted(&inside) || inside.strip_prefix('-').map_or(false, counted);
                            let binding = inside.chars().next().map_or(false, |c| Some(c) == lang.sigil);
                            let bare = opens(Lang::begins_name) && inside.chars().all(|c| lang.extends_name(c));
                            // A key of any other making is more than the
                            // shorter writing takes, and a language that
                            // says so stops rather than leave the
                            // brackets standing as letters.
                            if let (false, Some(said)) = (digits || binding || bare, lang.woven_index_amiss()) {
                                return Err(said);
                            }
                            let past = end + index.close.chars().count();
                            if digits || binding {
                                j = past;
                                stepped = true;
                            } else if bare {
                                // A bare word between the brackets
                                // stands for the text it spells and not
                                // for a name, so it is written as text.
                                if let Some(quote) = lang.raw_quotes.first().or_else(|| lang.quotes.first()) {
                                    let named: String = chars[i..j].iter().collect();
                                    written = Some(format!("{named}{}{quote}{inside}{quote}{}", index.open, index.close));
                                    j = past;
                                    stepped = true;
                                }
                            }
                        }
                    }
                }
                if !stepped {
                    if let Some(mark) = lang.member_mark.clone() {
                        if at_word(&chars, j, &mark) {
                            let after = j + mark.chars().count();
                            if chars.get(after).map_or(false, |c| lang.begins_name(*c)) {
                                let mut past = after + 1;
                                while past < chars.len() && lang.extends_name(chars[past]) {
                                    past += 1;
                                }
                                j = past;
                            }
                        }
                    }
                }
                parts.push((false, std::mem::take(&mut text)));
                parts.push((true, written.unwrap_or_else(|| chars[i..j].iter().collect())));
                i = j;
                continue;
            }
            text.push(c);
            i += 1;
        }
        parts.push((false, text));
        if !parts.iter().any(|(code, _)| *code) {
            self.push(Shape::Quote, s, 0, line, col);
            return Ok(());
        }
        let (Some(group), Some(join)) = (lang.grouping.as_ref(), lang.concat.as_ref()) else {
            return Err("String interpolation needs syntax.group and op.concat".to_string());
        };
        self.push(Shape::Sign, group.open.clone(), 0, line, col);
        self.push(Shape::Quote, String::new(), 0, line, col);
        for (code, part) in parts {
            if !code && part.is_empty() {
                continue;
            }
            self.push(Shape::Sign, join.clone(), 0, line, col);
            if !code {
                self.push(Shape::Quote, part, 0, line, col);
                continue;
            }
            let mut inner = Cursor { lang, text: part.chars().collect(), at: 0, row: line, column: col, out: Vec::new() };
            inner.run(false)?;
            self.out.append(&mut inner.out);
        }
        self.push(Shape::Sign, group.close.clone(), 0, line, col);
        Ok(())
    }

    fn number(&mut self) -> Result<(), String> {
        let (line, col) = (self.row, self.column);
        let lang = self.lang;
        let mut s = String::new();
        let broken = |c: &char| lang.digit_separators.contains(c);
        while let Some(c) = self.look(0).filter(|c| c.is_ascii_digit() || broken(c)) {
            s.push(c);
            self.step();
        }
        // A number may be written in a base of its own, after a digit
        // and a letter that name the base: `0x1f`, `0b1011`, `0o17`.
        let in_base = lang.base_prefixes.iter().find(|(prefix, base)| {
            let mut it = prefix.chars();
            let (Some(digit), Some(letter)) = (it.next(), it.next()) else { return false };
            s.len() == 1 && s.starts_with(digit) && self.look(0) == Some(letter)
                && (lang.number_strict || self.look(1).map_or(false, |c| c.is_digit(*base))
                    || (lang.separator_after_prefix && self.look(1).map_or(false, |c| broken(&c))
                        && self.look(2).map_or(false, |c| c.is_digit(*base))))
        });
        if let Some((prefix, base)) = in_base.cloned() {
            s.push(prefix.chars().nth(1).expect("a letter after the digit"));
            self.step();
            while let Some(c) = self.look(0).filter(|c| c.is_digit(base) || broken(c) || (lang.number_strict && lang.extends_name(*c))) {
                s.push(c);
                self.step();
            }
        } else if lang.base_mark.is_some() && self.look(0) == lang.base_mark {
            s.push(self.step());
            while let Some(c) = self.look(0) {
                let digit_next = self.look(1).map_or(false, |n| n.is_ascii_alphanumeric());
                let part = c.is_ascii_alphanumeric()
                    || (Some(c) == lang.point && digit_next)
                    || (Some(c) == lang.exponent_mark && digit_next);
                if !part {
                    break;
                }
                s.push(c);
                self.step();
            }
        } else {
            if lang.point.is_some() && self.look(0) == lang.point && (lang.bare_number_point || lang.number_point_edge || self.look(1).map_or(false, |c| c.is_ascii_digit())) {
                s.push(self.step());
                while let Some(c) = self.look(0).filter(|c| c.is_ascii_digit() || broken(c)) {
                    s.push(c);
                    self.step();
                }
            }
            // A decimal exponent: the letter, a sign perhaps, digits.
            let letter = self.look(0).filter(|c| lang.exponent_letters.contains(c));
            let signed = matches!(self.look(1), Some('+') | Some('-'));
            let digits_at = if signed { 2 } else { 1 };
            if letter.is_some() && (lang.number_strict || self.look(digits_at).map_or(false, |c| c.is_ascii_digit())) {
                for _ in 0..digits_at {
                    s.push(self.step());
                }
                while let Some(c) = self.look(0).filter(|c| c.is_ascii_digit() || broken(c)) {
                    s.push(c);
                    self.step();
                }
            }
        }
        if self.look(0).map_or(false, |c| lang.imaginary_letters.contains(&c)) {
            s.push(self.step());
        }
        if lang.number_strict {
            while self.look(0).map_or(false, |c| lang.extends_name(c)) {
                s.push(self.step());
            }
            number_spelling(&s, lang)?;
        }
        self.push(Shape::Numeral, s, 0, line, col);
        Ok(())
    }

    fn word(&mut self, prefixed: bool) {
        let (line, col) = (self.row, self.column);
        let lang = self.lang;
        let mut s = String::new();
        if prefixed {
            s.push(self.step());
        }
        while let Some(c) = self.look(0).filter(|c| lang.extends_name(*c)) {
            s.push(c);
            self.step();
        }
        // A builtin may go on with symbols and more words (println!,
        // console.log): the longest spelled in the definition wins.
        let mut extra = 0;
        for name in lang.builtins.keys().chain(lang.print_file_error.iter()).chain(lang.print_file_output.iter()) {
            if name.len() <= s.len() || !name.starts_with(s.as_str()) {
                continue;
            }
            let tail: Vec<char> = name[s.len()..].chars().collect();
            let fits = tail.iter().enumerate().all(|(i, c)| self.look(i) == Some(*c));
            let ends = self.look(tail.len()).map_or(true, |c| !lang.extends_name(c));
            if fits && ends && tail.len() > extra {
                extra = tail.len();
            }
        }
        for _ in 0..extra {
            s.push(self.step());
        }
        let lowered = s.to_lowercase();
        if lang.names_folded || (lang.keywords_folded && lang.keywords.contains(&lowered)) {
            s = lowered;
        }
        self.push(Shape::Instr, s, 0, line, col);
    }

    fn quoted_name(&mut self, quote: char) -> Result<(), String> {
        let (line, col) = (self.row, self.column);
        self.step();
        let mut s = String::new();
        let mut closed = false;
        while let Some(c) = self.look(0) {
            if c == '\n' {
                break;
            }
            self.step();
            if c == quote {
                closed = true;
                break;
            }
            s.push(c);
        }
        let lang = self.lang;
        let shaped = s.starts_with(|c| lang.begins_name(c)) && s.chars().all(|c| lang.extends_name(c));
        if !closed || !shaped {
            return Err(format!("Expected a name between {} quotes at {}:{}", quote, line, col));
        }
        self.push(Shape::Quoted, s, 0, line, col);
        Ok(())
    }

    fn symbol(&mut self) -> Result<(), String> {
        let (line, col) = (self.row, self.column);
        let window: String = self.text[self.at..].iter().take(8).collect();
        let Some(sym) = self.lang.symbols.iter().find(|s| window.starts_with(s.as_str())).cloned() else {
            return Err(self.lang.stopped_at_character(self.text[self.at], line, col));
        };
        for _ in sym.chars() {
            self.step();
        }
        self.push(Shape::Sign, sym, 0, line, col);
        Ok(())
    }
}

impl<'a> Cursor<'a> {
    /// Tokens to the end of the text.
    fn run(&mut self, mut at_line_start: bool) -> Result<(), String> {
        let lang = self.lang;
        while self.at < self.text.len() {
            if at_line_start {
                at_line_start = self.indentation();
                if at_line_start {
                    continue;
                }
            }
            if !lang.long_quotes.is_empty() {
                if lang.line_comments.iter().any(|m| at_word(&self.text, self.at, m)) {
                    while self.look(0).map_or(false, |c| c != '\n') { self.step(); }
                    continue;
                }
                if let Some((prefix, mark, raw, format)) = self.string_open() {
                    self.rich_string(prefix, &mark, raw, format)?;
                    continue;
                }
                if lang.adjacent_strings && self.look(0) == Some('\\') && self.look(1) == Some('\n') {
                    self.step(); self.step();
                    continue;
                }
            }
            let c = self.text[self.at];
            let joined = lang.line_continuations.iter().find_map(|mark| {
                if !at_word(&self.text, self.at, mark) {
                    return None;
                }
                let width = mark.chars().count();
                match (self.look(width), self.look(width + 1)) {
                    (Some('\n'), _) => Some(width + 1),
                    (Some('\r'), Some('\n')) => Some(width + 2),
                    _ => None,
                }
            });
            if let Some(width) = joined {
                for _ in 0..width {
                    self.step();
                }
                continue;
            }
            if c == '\n' {
                let (line, col) = (self.row, self.column);
                self.step();
                self.push(Shape::LineEnd, "\n".to_string(), 0, line, col);
                at_line_start = true;
            } else if c == ' ' || c == '\t' || c == '\r' {
                self.step();
            } else if lang.heredoc.as_deref().map_or(false, |mark| at_word(&self.text, self.at, mark)) {
                self.heredoc()?;
            } else if lang.line_continuations.iter().any(|m| at_word(&self.text, self.at, m)) {
                let mark = lang.line_continuations.iter().find(|m| at_word(&self.text, self.at, m)).unwrap();
                for _ in mark.chars() { self.step(); }
                if self.look(0) == Some('\r') { self.step(); }
                if self.look(0) != Some('\n') { return Err(lang.continuation_amiss.first().cloned().unwrap_or_default()); }
                self.step();
            } else if lang.quotes.contains(&c) {
                self.string(c)?;
            } else if c.is_ascii_digit()
                || ((lang.bare_number_point || lang.number_point_edge) && Some(c) == lang.point && self.look(1).map_or(false, |d| d.is_ascii_digit()))
            {
                self.number()?;
            } else if lang.quote_for_names == Some(c) {
                self.quoted_name(c)?;
            } else if lang.name_leads.contains(&c) && self.look(1).map_or(false, |n| lang.begins_name(n)) {
                // A sign standing before a name, saying nothing: PHP's `\Error`.
                self.step();
                self.word(false);
            } else if lang.begins_name(c) {
                self.word(false);
            } else if lang.sigil == Some(c) && self.look(1).map_or(false, |n| lang.begins_name(n)) {
                self.word(true);
            } else {
                self.symbol()?;
            }
        }
        Ok(())
    }
}

pub fn lex(source: &str, lang: &Lang) -> Result<Vec<Token>, String> {
    lex_at(source, lang).map_err(|(said, _)| said)
}

/// The same, telling besides which line the reading stopped on, which a
/// language with a word for such a stopping names.
pub fn lex_at(source: &str, lang: &Lang) -> Result<Vec<Token>, (String, usize)> {
    let mut out = match lang.template {
        true => woven_source(source, lang)?,
        false => {
            let text = drop_comments(drop_epilogue(drop_prologue(source, lang), lang), lang);
            let mut cur = Cursor { lang, text: text.chars().collect(), at: 0, row: 1, column: 1, out: Vec::new() };
            if let Err(said) = cur.run(true) {
                return Err((said, cur.row));
            }
            cur.out
        }
    };
    if lang.bind_names {
        if let Some(call) = &lang.calling {
            let mut depth = 0usize;
            out.retain(|token| {
                if token.is_lexeme(Shape::Sign, &call.open) { depth += 1; }
                else if token.is_lexeme(Shape::Sign, &call.close) { depth = depth.saturating_sub(1); }
                depth == 0 || !matches!(token.shape, Shape::Lead | Shape::LineEnd)
            });
        }
    }
    out.push(Token { shape: Shape::Finish, lexeme: "EOF".to_string(), width: 0, row: 1, column: 1 });
    Ok(out)
}

/// A source that is text with code in it (ext.lexical.template): what
/// stands between the prologue and the epilogue is read as code, and
/// everything else is written out as it stands, as though the program
/// had said so itself.
/// The bytes that spell a character's number, by the rule that spells
/// every one of them: a number under a hundred and twenty-eight stands
/// alone, and each wider band is written with one leading byte saying
/// how many follow. Half of a pair standing for one character between
/// them is spelled here like any other number, the reference spelling
/// it so where text is bytes.
fn spelled_bytes(number: u32) -> Vec<u8> {
    match number {
        n if n < 0x80 => vec![n as u8],
        n if n < 0x800 => vec![0xC0 | (n >> 6) as u8, 0x80 | (n & 0x3F) as u8],
        n if n < 0x10000 => vec![
            0xE0 | (n >> 12) as u8,
            0x80 | ((n >> 6) & 0x3F) as u8,
            0x80 | (n & 0x3F) as u8,
        ],
        n => vec![
            0xF0 | (n >> 18) as u8,
            0x80 | ((n >> 12) & 0x3F) as u8,
            0x80 | ((n >> 6) & 0x3F) as u8,
            0x80 | (n & 0x3F) as u8,
        ],
    }
}

/// Where a run of code ends: the first closing marker that is not
/// standing inside something spelling it out. One written between
/// quotes, or in a string laid over lines, is part of what that string
/// says and ends nothing, so the run cannot simply be looked through
/// for the marker. A block comment shields it in the same way.
///
/// A comment running to the end of its line does not shield it: the
/// reference ends the run at a marker written in one, and only the
/// line end saves what follows. That is why such a comment is read up
/// to whichever comes first.
fn code_ends_at(after: &str, closing: &str, lang: &Lang) -> Option<usize> {
    let mut at = 0;
    while at < after.len() {
        let rest = &after[at..];
        if rest.starts_with(closing) {
            return Some(at);
        }
        if lang.heredoc.as_deref().map_or(false, |mark| rest.starts_with(mark)) {
            if let Some(held) = heredoc_at(rest, lang) {
                at += held.done;
                continue;
            }
        }
        if let Some((open, close)) = lang.block_comments.iter().find(|(o, _)| rest.starts_with(o.as_str())) {
            let body = &rest[open.len()..];
            at += open.len() + body.find(close.as_str()).map_or(body.len(), |p| p + close.len());
            continue;
        }
        if lang.line_comments.iter().any(|m| rest.starts_with(m.as_str())) {
            let line = &rest[..rest.find('\n').map_or(rest.len(), |p| p + 1)];
            match line.find(closing) {
                Some(p) => return Some(at + p),
                None => at += line.len(),
            }
            continue;
        }
        let c = rest.chars().next().expect("a character");
        if lang.quotes.contains(&c) {
            at += quoted_width(rest, c);
            continue;
        }
        at += c.len_utf8();
    }
    None
}

/// How far a string written between marks reaches, counted from the
/// mark that opened it: a mark shielded by a backslash is a mark no
/// longer, and one never closed reaches to the end of what there is.
fn quoted_width(text: &str, quote: char) -> usize {
    let mut at = quote.len_utf8();
    while at < text.len() {
        let c = text[at..].chars().next().expect("a character");
        at += c.len_utf8();
        if c == quote {
            return at;
        }
        if c == '\\' {
            at += text[at..].chars().next().map_or(0, char::len_utf8);
        }
    }
    text.len()
}

fn woven_source(source: &str, lang: &Lang) -> Result<Vec<Token>, (String, usize)> {
    let opening = lang.prologue.clone().ok_or_else(|| ("A template needs lexical.prologue".to_string(), 0))?;
    let closing = lang.epilogue.first().cloned();
    let telling = lang
        .builtins
        .iter()
        .find(|(_, native)| **native == crate::code::Builtin::Tell)
        .map(|(word, _)| word.clone())
        .ok_or_else(|| ("A template needs a builtin that writes what it is given".to_string(), 0))?;
    let ending = lang.stmt_ends.first().cloned().unwrap_or_else(|| ";".to_string());
    let mut out: Vec<Token> = Vec::new();
    let told = |text: &str, out: &mut Vec<Token>| {
        if text.is_empty() {
            return;
        }
        out.push(Token { shape: Shape::Instr, lexeme: telling.clone(), width: 0, row: 1, column: 1 });
        out.push(Token { shape: Shape::Quote, lexeme: text.to_string(), width: 0, row: 1, column: 1 });
        out.push(Token { shape: Shape::Sign, lexeme: ending.clone(), width: 0, row: 1, column: 1 });
    };
    let mut rest = source;
    let mut row = 1;
    loop {
        // Either marker may open a run of code, whichever stands first.
        // The short one says the run is a thing to be written out, and
        // the word that writes it is put before it.
        // Whichever marker stands first opens the run. Where two stand
        // in the same place, the longer of them is the one meant: a
        // marker that is the opening of another says nothing on its own.
        let mut found: Option<(usize, String, bool)> = None;
        let markers = [
            (Some(opening.clone()), false),
            (lang.prologue_echo.clone(), true),
            (lang.prologue_brief.clone(), false),
        ];
        for (mark, writes) in markers.into_iter() {
            let Some(mark) = mark else { continue };
            let Some(at) = marker_at(rest, &mark, lang.prologue_folded) else { continue };
            let better = match &found {
                None => true,
                Some((was, seen, _)) => at < *was || (at == *was && mark.len() > seen.len()),
            };
            if better {
                found = Some((at, mark, writes));
            }
        }
        let Some((at, mark, writes)) = found else { break };
        told(&rest[..at], &mut out);
        row += rest[..at].matches('\n').count();
        let after = &rest[at + mark.len()..];
        let (code, tail) = match closing.as_ref().and_then(|e| code_ends_at(after, e, lang)) {
            Some(end) => (&after[..end], &after[end + closing.as_ref().map_or(0, String::len)..]),
            None => (after, ""),
        };
        let text = drop_comments(code, lang);
        let mut cur = Cursor { lang, text: text.chars().collect(), at: 0, row, column: 1, out: Vec::new() };
        if let Err(said) = cur.run(true) {
            return Err((said, cur.row));
        }
        // A run of code stands as its own statement, however it ended.
        if writes {
            out.push(Token { shape: Shape::Instr, lexeme: telling.clone(), width: 0, row, column: 1 });
        }
        out.append(&mut cur.out);
        out.push(Token { shape: Shape::Sign, lexeme: ending.clone(), width: 0, row: cur.row, column: 1 });
        row = cur.row;
        // One line end straight after the closing marker is PHP's to eat.
        // Eaten or not, the line it ended is a line of the page and is
        // counted, so that what a complaint says of a line names the
        // line the program was written on.
        let shorter = tail.strip_prefix('\n').unwrap_or_else(|| tail.strip_prefix("\r\n").unwrap_or(tail));
        if shorter.len() != tail.len() {
            row += 1;
        }
        rest = shorter;
    }
    told(rest, &mut out);
    Ok(out)
}

/// Whether a mark stands written at that place in a run of characters.
fn at_word(chars: &[char], at: usize, mark: &str) -> bool {
    !mark.is_empty() && chars.len() >= at + mark.chars().count() && chars[at..].iter().zip(mark.chars()).all(|(c, m)| *c == m)
}

/// Where a mark next stands, from that place onwards; nothing where it
/// stands nowhere after it.
fn word_at(chars: &[char], from: usize, mark: &str) -> Option<usize> {
    (from..chars.len()).find(|at| at_word(chars, *at, mark))
}

/// Check the places where digits and separating marks may stand before
/// the marks are taken away. The words of a complaint belong to the table.
pub fn number_spelling(text: &str, lang: &Lang) -> Result<(), String> {
    let amiss = || lang.number_amiss.clone().unwrap_or_else(|| format!("Invalid number: {}", text));
    let separated = |s: &str, radix: u32, after_prefix: bool| {
        let chars: Vec<char> = s.chars().collect();
        !chars.is_empty() && chars.iter().enumerate().all(|(i, c)| {
            c.is_digit(radix) || (lang.digit_separators.contains(c)
                && (i == 0 && after_prefix || i > 0 && chars[i - 1].is_digit(radix))
                && chars.get(i + 1).map_or(false, |d| d.is_digit(radix)))
        })
    };
    for (prefix, radix) in &lang.base_prefixes {
        if let Some(body) = text.strip_prefix(prefix.as_str()) {
            let (plain, pieces) = match radix {
                2 => (&lang.binary_amiss, &lang.binary_digit_amiss),
                8 => (&lang.octal_amiss, &lang.octal_digit_amiss),
                _ => (&lang.hex_amiss, &lang.binary_digit_amiss),
            };
            let mut may_separate = true;
            for c in body.chars() {
                if c.is_digit(*radix) {
                    may_separate = true;
                } else if lang.digit_separators.contains(&c) && may_separate {
                    may_separate = false;
                } else {
                    if *radix < 10 && pieces.len() == 2 && c.is_ascii_digit() {
                        return Err(format!("{}{}{}", pieces[0], c, pieces[1]));
                    }
                    return Err(plain.clone().unwrap_or_else(amiss));
                }
            }
            return if separated(body, *radix, true) { Ok(()) } else { Err(plain.clone().unwrap_or_else(amiss)) };
        }
    }
    let imaginary = text.chars().last().filter(|c| lang.imaginary_letters.contains(c));
    let bare = imaginary.map_or(text, |c| &text[..text.len() - c.len_utf8()]);
    let (mantissa, power) = match bare.find(|c| lang.exponent_letters.contains(&c)) {
        Some(at) => (&bare[..at], Some(&bare[at + 1..])),
        None => (bare, None),
    };
    if let Some(exponent) = power {
        if !separated(exponent.trim_start_matches(['+', '-']), 10, false)
            || exponent.starts_with("++") || exponent.starts_with("--") || exponent.starts_with("+-") || exponent.starts_with("-+") {
            return Err(amiss());
        }
    }
    let point = lang.point.and_then(|c| mantissa.find(c).map(|at| (at, c.len_utf8())));
    if let Some((at, width)) = point {
        let (left, right) = (&mantissa[..at], &mantissa[at + width..]);
        if left.is_empty() && right.is_empty()
            || !left.is_empty() && !separated(left, 10, false)
            || !right.is_empty() && !separated(right, 10, false) {
            return Err(amiss());
        }
    } else if !separated(mantissa, 10, false) {
        return Err(amiss());
    }
    if point.is_none() && power.is_none() && imaginary.is_none() && bare.starts_with('0')
        && bare.chars().any(|c| c.is_ascii_digit() && c != '0') {
        if let Some(words) = &lang.number_leading_zero {
            return Err(words.clone());
        }
    }
    Ok(())
}
