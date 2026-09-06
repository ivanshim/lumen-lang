// Scanning: source text to tokens, by the definition's spellings.
//
// Comments and the prologue are removed first, outside strings and with
// line ends kept so positions stay right. Then each token is a string
// (unescaped here), a number, a quoted name, a word, a symbol, a line end
// or the indentation that opens a line.

use crate::definition::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tk {
    Word,
    /// A name between name quotes, given as data.
    Name,
    Number,
    Text,
    Symbol,
    Eol,
    /// Indentation of a non-blank line; `width` carries its size.
    Indent,
    /// Block delimiters the layout pass synthesises for an indented language.
    Open,
    Close,
    End,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: Tk,
    pub text: String,
    pub width: usize,
    pub line: usize,
    pub col: usize,
}

impl Token {
    pub fn is(&self, kind: Tk, text: &str) -> bool {
        self.kind == kind && self.text == text
    }
}

fn without_prologue<'a>(source: &'a str, lang: &Language) -> &'a str {
    let Some(prologue) = &lang.prologue else { return source };
    let lead = source.len() - source.trim_start().len();
    if source[..lead].contains('\n') || !source[lead..].starts_with(prologue.as_str()) {
        return source;
    }
    &source[lead + prologue.len()..]
}

fn without_comments(source: &str, lang: &Language) -> String {
    if lang.line_comments.is_empty() && lang.block_comments.is_empty() {
        return source.to_string();
    }
    let mut out = String::with_capacity(source.len());
    let mut rest = source;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    while let Some(c) = rest.chars().next() {
        let step = c.len_utf8();
        if let Some(q) = quote {
            out.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == q {
                quote = None;
            }
            rest = &rest[step..];
            continue;
        }
        if lang.quotes.contains(&c) {
            quote = Some(c);
            out.push(c);
            rest = &rest[step..];
            continue;
        }
        if let Some((open, close)) = lang.block_comments.iter().find(|(open, _)| rest.starts_with(open.as_str())) {
            let body = &rest[open.len()..];
            let end = body.find(close.as_str()).map_or(body.len(), |at| at + close.len());
            out.extend(body[..end].chars().filter(|c| *c == '\n'));
            rest = &body[end..];
            continue;
        }
        if lang.line_comments.iter().any(|m| rest.starts_with(m.as_str())) {
            rest = rest.find('\n').map_or("", |at| &rest[at..]);
            continue;
        }
        out.push(c);
        rest = &rest[step..];
    }
    out
}

struct Scanner<'a> {
    lang: &'a Language,
    chars: Vec<char>,
    at: usize,
    line: usize,
    col: usize,
    tokens: Vec<Token>,
}

impl<'a> Scanner<'a> {
    fn peek(&self, ahead: usize) -> Option<char> {
        self.chars.get(self.at + ahead).copied()
    }

    fn take(&mut self) -> char {
        let c = self.chars[self.at];
        self.at += 1;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        c
    }

    fn emit(&mut self, kind: Tk, text: String, line: usize, col: usize) {
        self.tokens.push(Token { kind, text, width: 0, line, col });
    }

    fn line_start(&mut self) -> bool {
        let mut width = 0;
        let mut j = self.at;
        while let Some(c) = self.chars.get(j) {
            match c {
                ' ' => width += 1,
                '\t' => width += self.lang.indent,
                _ => break,
            }
            j += 1;
        }
        let end = self.chars[j..].iter().position(|c| *c == '\n').map_or(self.chars.len(), |p| j + p);
        if self.chars[j..end].iter().all(|c| c.is_whitespace()) {
            // A blank line: nothing at all, not even a line end.
            while self.at < end {
                self.take();
            }
            if self.at < self.chars.len() {
                self.take();
            }
            return true;
        }
        let (line, col) = (self.line, self.col);
        self.tokens.push(Token { kind: Tk::Indent, text: String::new(), width, line, col });
        while self.at < j {
            self.take();
        }
        false
    }

    fn string(&mut self, quote: char) -> Result<(), String> {
        let (line, col) = (self.line, self.col);
        self.take();
        let raw = self.lang.raw_quotes.contains(&quote);
        let mut text = String::new();
        loop {
            let Some(c) = self.peek(0) else { return Err(format!("Unterminated {} string", quote)) };
            if c == '\\' {
                if let Some(next) = self.peek(1) {
                    self.take();
                    self.take();
                    let known = next == '\\' || next == quote || (!raw && self.lang.escape_letters.contains(&next));
                    if known {
                        text.push(match next {
                            'n' => '\n',
                            't' => '\t',
                            'r' => '\r',
                            '0' => '\0',
                            other => other,
                        });
                    } else {
                        text.push('\\');
                        text.push(next);
                    }
                    continue;
                }
            }
            self.take();
            if c == quote {
                break;
            }
            text.push(c);
        }
        self.emit(Tk::Text, text, line, col);
        Ok(())
    }

    fn number(&mut self) {
        let (line, col) = (self.line, self.col);
        let lang = self.lang;
        let mut text = String::new();
        while let Some(c) = self.peek(0).filter(char::is_ascii_digit) {
            text.push(c);
            self.take();
        }
        let hex = lang.hex_prefix.as_ref().and_then(|p| {
            let mut it = p.chars();
            let (digit, letter) = (it.next()?, it.next()?);
            let fits = text.len() == 1 && text.starts_with(digit) && self.peek(0) == Some(letter);
            (fits && self.peek(1).map_or(false, |c| c.is_ascii_hexdigit())).then_some(letter)
        });
        if let Some(letter) = hex {
            text.push(letter);
            self.take();
            while let Some(c) = self.peek(0).filter(char::is_ascii_hexdigit) {
                text.push(c);
                self.take();
            }
        } else if lang.base_mark.is_some() && self.peek(0) == lang.base_mark {
            text.push(self.take());
            while let Some(c) = self.peek(0) {
                let digit_follows = self.peek(1).map_or(false, |n| n.is_ascii_alphanumeric());
                let inside = c.is_ascii_alphanumeric()
                    || (Some(c) == lang.point && digit_follows)
                    || (Some(c) == lang.exponent_mark && digit_follows);
                if !inside {
                    break;
                }
                text.push(c);
                self.take();
            }
        } else if lang.point.is_some() && self.peek(0) == lang.point && self.peek(1).map_or(false, |c| c.is_ascii_digit()) {
            text.push(self.take());
            while let Some(c) = self.peek(0).filter(char::is_ascii_digit) {
                text.push(c);
                self.take();
            }
        }
        self.emit(Tk::Number, text, line, col);
    }

    fn word(&mut self, prefixed: bool) {
        let (line, col) = (self.line, self.col);
        let lang = self.lang;
        let mut text = String::new();
        if prefixed {
            text.push(self.take());
        }
        while let Some(c) = self.peek(0).filter(|c| lang.word_continues(*c)) {
            text.push(c);
            self.take();
        }
        // A builtin name may continue with symbols and further words
        // (println!, console.log): take the longest one spelled out here.
        let mut longest = 0;
        for name in lang.builtins.keys() {
            if name.len() <= text.len() || !name.starts_with(text.as_str()) {
                continue;
            }
            let tail: Vec<char> = name[text.len()..].chars().collect();
            let matches = tail.iter().enumerate().all(|(i, c)| self.peek(i) == Some(*c));
            let clean_end = self.peek(tail.len()).map_or(true, |c| !lang.word_continues(c));
            if matches && clean_end && tail.len() > longest {
                longest = tail.len();
            }
        }
        for _ in 0..longest {
            text.push(self.take());
        }
        let lowered = text.to_lowercase();
        if (lang.fold_keywords && lang.reserved.contains(&lowered)) || lang.fold_names {
            text = lowered;
        }
        self.emit(Tk::Word, text, line, col);
    }

    fn quoted_name(&mut self, quote: char) -> Result<(), String> {
        let (line, col) = (self.line, self.col);
        self.take();
        let mut text = String::new();
        let mut closed = false;
        while let Some(c) = self.peek(0) {
            if c == '\n' {
                break;
            }
            self.take();
            if c == quote {
                closed = true;
                break;
            }
            text.push(c);
        }
        let lang = self.lang;
        let shaped = text.starts_with(|c| lang.word_starts(c)) && text.chars().all(|c| lang.word_continues(c));
        if !closed || !shaped {
            return Err(format!("Expected a name between {} quotes at {}:{}", quote, line, col));
        }
        self.emit(Tk::Name, text, line, col);
        Ok(())
    }

    fn symbol(&mut self) -> Result<(), String> {
        let (line, col) = (self.line, self.col);
        let ahead: String = self.chars[self.at..].iter().take(8).collect();
        match self.lang.symbols.iter().find(|s| ahead.starts_with(s.as_str())) {
            Some(symbol) => {
                let symbol = symbol.clone();
                for _ in symbol.chars() {
                    self.take();
                }
                self.emit(Tk::Symbol, symbol, line, col);
                Ok(())
            }
            None => Err(format!("Unexpected character '{}' at {}:{}", self.chars[self.at], line, col)),
        }
    }
}

pub fn scan(source: &str, lang: &Language) -> Result<Vec<Token>, String> {
    let source = without_comments(without_prologue(source, lang), lang);
    let mut s = Scanner { lang, chars: source.chars().collect(), at: 0, line: 1, col: 1, tokens: Vec::new() };
    let mut fresh_line = true;
    while s.at < s.chars.len() {
        if fresh_line {
            fresh_line = false;
            if s.line_start() {
                fresh_line = true;
                continue;
            }
        }
        let c = s.chars[s.at];
        if c == '\n' {
            let (line, col) = (s.line, s.col);
            s.take();
            s.emit(Tk::Eol, "\n".to_string(), line, col);
            fresh_line = true;
        } else if c == ' ' || c == '\t' || c == '\r' {
            s.take();
        } else if lang.quotes.contains(&c) {
            s.string(c)?;
        } else if c.is_ascii_digit() {
            s.number();
        } else if lang.name_quote == Some(c) {
            s.quoted_name(c)?;
        } else if lang.word_starts(c) {
            s.word(false);
        } else if lang.variable_prefix == Some(c) && s.peek(1).map_or(false, |n| lang.word_starts(n)) {
            s.word(true);
        } else {
            s.symbol()?;
        }
    }
    let (line, col) = (s.line, s.col);
    s.emit(Tk::End, "EOF".to_string(), line, col);
    Ok(s.tokens)
}
