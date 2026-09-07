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

fn drop_prologue<'a>(source: &'a str, lang: &Lang) -> &'a str {
    let Some(prologue) = &lang.prologue else { return source };
    let lead = source.len() - source.trim_start().len();
    if !source[..lead].contains('\n') && source[lead..].starts_with(prologue.as_str()) {
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
    if lang.line_comments.is_empty() && lang.block_comments.is_empty() {
        return source.to_string();
    }
    let mut kept = String::with_capacity(source.len());
    let mut ahead = source;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    while let Some(c) = ahead.chars().next() {
        let w = c.len_utf8();
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
        if self.text[j..end].iter().all(|c| c.is_whitespace()) {
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

    fn string(&mut self, quote: char) -> Result<(), String> {
        let (line, col) = (self.row, self.column);
        self.step();
        let raw = self.lang.raw_quotes.contains(&quote);
        let woven = self.lang.interpolating.contains(&quote);
        let mut s = String::new();
        // Char positions in s that came escaped: text, never code.
        let mut shielded: Vec<usize> = Vec::new();
        loop {
            let Some(c) = self.look(0) else { return Err(format!("Unterminated {} string", quote)) };
            if c == '\\' {
                if let Some(next) = self.look(1) {
                    self.step();
                    self.step();
                    if woven && Some(next) == self.lang.sigil {
                        // An escaped sigil is just the sigil.
                        shielded.push(s.chars().count());
                        s.push(next);
                        continue;
                    }
                    if next == '\\' || next == quote || (!raw && self.lang.escape_letters.contains(&next)) {
                        s.push(match next {
                            'n' => '\n',
                            't' => '\t',
                            'r' => '\r',
                            '0' => '\0',
                            c => c,
                        });
                    } else {
                        s.push('\\');
                        s.push(next);
                    }
                    continue;
                }
            }
            self.step();
            if c == quote {
                break;
            }
            s.push(c);
        }
        if woven {
            return self.woven(s, &shielded, line, col);
        }
        self.push(Shape::Quote, s, 0, line, col);
        Ok(())
    }

    /// A string that weaves values in (ext.lexical.interpolating_quotes):
    /// `$name`, `$name[i]` and `{$expr}` inside it become code, and the
    /// whole becomes a bracketed concatenation of its parts, starting
    /// from an empty string so the result is always text.
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
                // A simple index: digits or a variable up to the bracket.
                if chars.get(j) == Some(&'[') {
                    if let Some(width) = chars[j..].iter().position(|c| *c == ']') {
                        let inner: String = chars[j + 1..j + width].iter().collect();
                        let simple = !inner.is_empty()
                            && (inner.chars().all(|c| c.is_ascii_digit()) || inner.starts_with(|c| Some(c) == lang.sigil));
                        if simple {
                            j += width + 1;
                        }
                    }
                }
                parts.push((false, std::mem::take(&mut text)));
                parts.push((true, chars[i..j].iter().collect()));
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

    fn number(&mut self) {
        let (line, col) = (self.row, self.column);
        let lang = self.lang;
        let mut s = String::new();
        while let Some(c) = self.look(0).filter(char::is_ascii_digit) {
            s.push(c);
            self.step();
        }
        let hex_letter = lang.hex_prefix.as_ref().and_then(|p| {
            let mut it = p.chars();
            let (digit, letter) = (it.next()?, it.next()?);
            let here = s.len() == 1 && s.starts_with(digit) && self.look(0) == Some(letter);
            (here && self.look(1).map_or(false, |c| c.is_ascii_hexdigit())).then_some(letter)
        });
        if let Some(letter) = hex_letter {
            s.push(letter);
            self.step();
            while let Some(c) = self.look(0).filter(char::is_ascii_hexdigit) {
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
            if lang.point.is_some() && self.look(0) == lang.point && self.look(1).map_or(false, |c| c.is_ascii_digit()) {
                s.push(self.step());
                while let Some(c) = self.look(0).filter(char::is_ascii_digit) {
                    s.push(c);
                    self.step();
                }
            }
            // A decimal exponent: the letter, a sign perhaps, digits.
            let letter = self.look(0).filter(|c| lang.exponent_letters.contains(c));
            let signed = matches!(self.look(1), Some('+') | Some('-'));
            let digits_at = if signed { 2 } else { 1 };
            if letter.is_some() && self.look(digits_at).map_or(false, |c| c.is_ascii_digit()) {
                for _ in 0..digits_at {
                    s.push(self.step());
                }
                while let Some(c) = self.look(0).filter(char::is_ascii_digit) {
                    s.push(c);
                    self.step();
                }
            }
        }
        self.push(Shape::Numeral, s, 0, line, col);
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
        for name in lang.builtins.keys() {
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
            return Err(format!("Unexpected character '{}' at {}:{}", self.text[self.at], line, col));
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
            let c = self.text[self.at];
            if c == '\n' {
                let (line, col) = (self.row, self.column);
                self.step();
                self.push(Shape::LineEnd, "\n".to_string(), 0, line, col);
                at_line_start = true;
            } else if c == ' ' || c == '\t' || c == '\r' {
                self.step();
            } else if lang.quotes.contains(&c) {
                self.string(c)?;
            } else if c.is_ascii_digit() {
                self.number();
            } else if lang.quote_for_names == Some(c) {
                self.quoted_name(c)?;
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
    let text = drop_comments(drop_epilogue(drop_prologue(source, lang), lang), lang);
    let mut cur = Cursor { lang, text: text.chars().collect(), at: 0, row: 1, column: 1, out: Vec::new() };
    cur.run(true)?;
    let (line, col) = (cur.row, cur.column);
    cur.push(Shape::Finish, "EOF".to_string(), 0, line, col);
    Ok(cur.out)
}
