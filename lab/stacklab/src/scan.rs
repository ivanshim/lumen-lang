// Scanning: text to tokens, by the definition's spellings. The prologue
// and comments go first, outside strings, line ends kept. Then each token
// is a string, a number, a quoted name, a word, a symbol, a line end or
// the indentation opening a line.

use crate::language::Def;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Word,
    /// A name between name quotes: data for the word after it.
    Name,
    Number,
    Text,
    Symbol,
    Eol,
    /// The indentation of a line that has something on it.
    Indent,
    /// Block boundaries the shaping pass adds for an indented language.
    Open,
    Close,
    End,
}

#[derive(Debug, Clone)]
pub struct Tok {
    pub kind: Kind,
    pub text: String,
    pub width: usize,
    pub line: usize,
    pub col: usize,
}

impl Tok {
    pub fn is(&self, kind: Kind, text: &str) -> bool {
        self.kind == kind && self.text == text
    }
}

fn strip_prologue<'a>(source: &'a str, def: &Def) -> &'a str {
    let Some(prologue) = &def.prologue else { return source };
    let lead = source.len() - source.trim_start().len();
    if !source[..lead].contains('\n') && source[lead..].starts_with(prologue.as_str()) {
        &source[lead + prologue.len()..]
    } else {
        source
    }
}

fn strip_comments(source: &str, def: &Def) -> String {
    if def.line_comments.is_empty() && def.block_comments.is_empty() {
        return source.to_string();
    }
    let mut out = String::with_capacity(source.len());
    let mut rest = source;
    let mut inside: Option<char> = None;
    let mut backslashed = false;
    while let Some(c) = rest.chars().next() {
        let width = c.len_utf8();
        match inside {
            Some(q) => {
                out.push(c);
                if backslashed {
                    backslashed = false;
                } else if c == '\\' {
                    backslashed = true;
                } else if c == q {
                    inside = None;
                }
                rest = &rest[width..];
            }
            None if def.quotes.contains(&c) => {
                inside = Some(c);
                out.push(c);
                rest = &rest[width..];
            }
            None => {
                if let Some((open, close)) = def.block_comments.iter().find(|(o, _)| rest.starts_with(o.as_str())) {
                    let body = &rest[open.len()..];
                    let end = body.find(close.as_str()).map_or(body.len(), |at| at + close.len());
                    out.extend(body[..end].chars().filter(|c| *c == '\n'));
                    rest = &body[end..];
                } else if def.line_comments.iter().any(|m| rest.starts_with(m.as_str())) {
                    rest = rest.find('\n').map_or("", |at| &rest[at..]);
                } else {
                    out.push(c);
                    rest = &rest[width..];
                }
            }
        }
    }
    out
}

struct Cursor<'a> {
    def: &'a Def,
    text: Vec<char>,
    at: usize,
    line: usize,
    col: usize,
    out: Vec<Tok>,
}

impl<'a> Cursor<'a> {
    fn look(&self, ahead: usize) -> Option<char> {
        self.text.get(self.at + ahead).copied()
    }

    fn step(&mut self) -> char {
        let c = self.text[self.at];
        self.at += 1;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        c
    }

    fn push(&mut self, kind: Kind, text: String, width: usize, line: usize, col: usize) {
        self.out.push(Tok { kind, text, width, line, col });
    }

    /// The indentation of a line; a blank line is skipped whole. Returns
    /// whether the line was blank.
    fn indentation(&mut self) -> bool {
        let mut width = 0;
        let mut j = self.at;
        while let Some(&c) = self.text.get(j) {
            width += match c {
                ' ' => 1,
                '\t' => self.def.indent,
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
        let (line, col) = (self.line, self.col);
        self.push(Kind::Indent, String::new(), width, line, col);
        while self.at < j {
            self.step();
        }
        false
    }

    fn string(&mut self, quote: char) -> Result<(), String> {
        let (line, col) = (self.line, self.col);
        self.step();
        let raw = self.def.raw_quotes.contains(&quote);
        let mut s = String::new();
        loop {
            let Some(c) = self.look(0) else { return Err(format!("Unterminated {} string", quote)) };
            if c == '\\' {
                if let Some(next) = self.look(1) {
                    self.step();
                    self.step();
                    if next == '\\' || next == quote || (!raw && self.def.escapes.contains(&next)) {
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
        self.push(Kind::Text, s, 0, line, col);
        Ok(())
    }

    fn number(&mut self) {
        let (line, col) = (self.line, self.col);
        let def = self.def;
        let mut s = String::new();
        while let Some(c) = self.look(0).filter(char::is_ascii_digit) {
            s.push(c);
            self.step();
        }
        let hex_letter = def.hex_prefix.as_ref().and_then(|p| {
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
        } else if def.base_mark.is_some() && self.look(0) == def.base_mark {
            s.push(self.step());
            while let Some(c) = self.look(0) {
                let digit_next = self.look(1).map_or(false, |n| n.is_ascii_alphanumeric());
                let part = c.is_ascii_alphanumeric()
                    || (Some(c) == def.point && digit_next)
                    || (Some(c) == def.exponent_mark && digit_next);
                if !part {
                    break;
                }
                s.push(c);
                self.step();
            }
        } else if def.point.is_some() && self.look(0) == def.point && self.look(1).map_or(false, |c| c.is_ascii_digit()) {
            s.push(self.step());
            while let Some(c) = self.look(0).filter(char::is_ascii_digit) {
                s.push(c);
                self.step();
            }
        }
        self.push(Kind::Number, s, 0, line, col);
    }

    fn word(&mut self, prefixed: bool) {
        let (line, col) = (self.line, self.col);
        let def = self.def;
        let mut s = String::new();
        if prefixed {
            s.push(self.step());
        }
        while let Some(c) = self.look(0).filter(|c| def.continues_word(*c)) {
            s.push(c);
            self.step();
        }
        // A builtin may go on with symbols and more words (println!,
        // console.log): the longest spelled in the definition wins.
        let mut extra = 0;
        for name in def.natives.keys() {
            if name.len() <= s.len() || !name.starts_with(s.as_str()) {
                continue;
            }
            let tail: Vec<char> = name[s.len()..].chars().collect();
            let fits = tail.iter().enumerate().all(|(i, c)| self.look(i) == Some(*c));
            let ends = self.look(tail.len()).map_or(true, |c| !def.continues_word(c));
            if fits && ends && tail.len() > extra {
                extra = tail.len();
            }
        }
        for _ in 0..extra {
            s.push(self.step());
        }
        let lowered = s.to_lowercase();
        if def.fold_names || (def.fold_keywords && def.reserved.contains(&lowered)) {
            s = lowered;
        }
        self.push(Kind::Word, s, 0, line, col);
    }

    fn quoted_name(&mut self, quote: char) -> Result<(), String> {
        let (line, col) = (self.line, self.col);
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
        let def = self.def;
        let shaped = s.starts_with(|c| def.starts_word(c)) && s.chars().all(|c| def.continues_word(c));
        if !closed || !shaped {
            return Err(format!("Expected a name between {} quotes at {}:{}", quote, line, col));
        }
        self.push(Kind::Name, s, 0, line, col);
        Ok(())
    }

    fn symbol(&mut self) -> Result<(), String> {
        let (line, col) = (self.line, self.col);
        let window: String = self.text[self.at..].iter().take(8).collect();
        let Some(sym) = self.def.symbols.iter().find(|s| window.starts_with(s.as_str())).cloned() else {
            return Err(format!("Unexpected character '{}' at {}:{}", self.text[self.at], line, col));
        };
        for _ in sym.chars() {
            self.step();
        }
        self.push(Kind::Symbol, sym, 0, line, col);
        Ok(())
    }
}

pub fn scan(source: &str, def: &Def) -> Result<Vec<Tok>, String> {
    let text = strip_comments(strip_prologue(source, def), def);
    let mut cur = Cursor { def, text: text.chars().collect(), at: 0, line: 1, col: 1, out: Vec::new() };
    let mut at_line_start = true;
    while cur.at < cur.text.len() {
        if at_line_start {
            at_line_start = cur.indentation();
            if at_line_start {
                continue;
            }
        }
        let c = cur.text[cur.at];
        if c == '\n' {
            let (line, col) = (cur.line, cur.col);
            cur.step();
            cur.push(Kind::Eol, "\n".to_string(), 0, line, col);
            at_line_start = true;
        } else if c == ' ' || c == '\t' || c == '\r' {
            cur.step();
        } else if def.quotes.contains(&c) {
            cur.string(c)?;
        } else if c.is_ascii_digit() {
            cur.number();
        } else if def.name_quote == Some(c) {
            cur.quoted_name(c)?;
        } else if def.starts_word(c) {
            cur.word(false);
        } else if def.var_prefix == Some(c) && cur.look(1).map_or(false, |n| def.starts_word(n)) {
            cur.word(true);
        } else {
            cur.symbol()?;
        }
    }
    let (line, col) = (cur.line, cur.col);
    cur.push(Kind::End, "EOF".to_string(), 0, line, col);
    Ok(cur.out)
}
