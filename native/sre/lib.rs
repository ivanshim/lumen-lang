// Compiles a regular expression into SRE code, following CPython 3.14.8 Lib/re/_parser.py and
// Lib/re/_compiler.py (PSF License). Only patterns that parse without any error or warning are
// handled here; every other case returns None so that the Python compiler decides, with its own
// exceptions and warnings.

mod cases;

pub struct Compiled {
    pub flags: i64,
    pub code: Vec<u32>,
    pub groups: usize,
    pub names: Vec<(String, u32)>,
    pub indexgroup: Vec<Option<String>>,
}

/// Case folding of the host runtime, so that compiled code agrees with `_sre.unicode_tolower`.
pub struct Case<'a> {
    pub lower: &'a dyn Fn(u32) -> u32,
    pub cased: &'a dyn Fn(u32) -> bool,
}

type Fail = ();
type R<T> = Result<T, Fail>;

const MAXREPEAT: u128 = 4294967295;
const MAXGROUPS: usize = 1073741823;
const MAXWIDTH: u128 = 1 << 64;
const MAXCODE: u128 = 4294967295;
const DEEPEST: usize = 60;

const IGNORECASE: u32 = 2;
const LOCALE: u32 = 4;
const MULTILINE: u32 = 8;
const DOTALL: u32 = 16;
const UNICODE: u32 = 32;
const VERBOSE: u32 = 64;
const DEBUG: u32 = 128;
const ASCII: u32 = 256;
const TYPE_FLAGS: u32 = ASCII | LOCALE | UNICODE;

mod op {
    pub const FAILURE: u32 = 0;
    pub const SUCCESS: u32 = 1;
    pub const ANY: u32 = 2;
    pub const ANY_ALL: u32 = 3;
    pub const ASSERT: u32 = 4;
    pub const ASSERT_NOT: u32 = 5;
    pub const AT: u32 = 6;
    pub const BRANCH: u32 = 7;
    pub const CATEGORY: u32 = 8;
    pub const CHARSET: u32 = 9;
    pub const BIGCHARSET: u32 = 10;
    pub const GROUPREF: u32 = 11;
    pub const GROUPREF_EXISTS: u32 = 12;
    pub const IN: u32 = 13;
    pub const INFO: u32 = 14;
    pub const JUMP: u32 = 15;
    pub const LITERAL: u32 = 16;
    pub const MARK: u32 = 17;
    pub const MAX_UNTIL: u32 = 18;
    pub const MIN_UNTIL: u32 = 19;
    pub const NOT_LITERAL: u32 = 20;
    pub const NEGATE: u32 = 21;
    pub const RANGE: u32 = 22;
    pub const REPEAT: u32 = 23;
    pub const REPEAT_ONE: u32 = 24;
    pub const MIN_REPEAT_ONE: u32 = 26;
    pub const ATOMIC_GROUP: u32 = 27;
    pub const POSSESSIVE_REPEAT: u32 = 28;
    pub const POSSESSIVE_REPEAT_ONE: u32 = 29;
    pub const GROUPREF_IGNORE: u32 = 30;
    pub const IN_IGNORE: u32 = 31;
    pub const LITERAL_IGNORE: u32 = 32;
    pub const NOT_LITERAL_IGNORE: u32 = 33;
    pub const GROUPREF_UNI_IGNORE: u32 = 38;
    pub const IN_UNI_IGNORE: u32 = 39;
    pub const LITERAL_UNI_IGNORE: u32 = 40;
    pub const NOT_LITERAL_UNI_IGNORE: u32 = 41;
    pub const RANGE_UNI_IGNORE: u32 = 42;
}

const AT_BEGINNING: u32 = 0;
const AT_BEGINNING_LINE: u32 = 1;
const AT_BEGINNING_STRING: u32 = 2;
const AT_BOUNDARY: u32 = 3;
const AT_NON_BOUNDARY: u32 = 4;
const AT_END: u32 = 5;
const AT_END_LINE: u32 = 6;
const AT_END_STRING: u32 = 7;
const AT_UNI_BOUNDARY: u32 = 10;
const AT_UNI_NON_BOUNDARY: u32 = 11;

#[derive(Clone, Copy, PartialEq)]
enum Kind { Max, Min, Possessive }

#[derive(Clone, PartialEq, Debug)]
enum Member { Negate, Lit(u32), Range(u32, u32), RangeUni(u32, u32), Cat(u32), Charset(Vec<u32>), Big(Vec<u32>) }

#[derive(Clone)]
enum Node {
    Lit(u32),
    NotLit(u32),
    Any,
    In(Vec<Member>),
    At(u32),
    Branch(Vec<Pat>),
    Sub { group: Option<u32>, add: u32, del: u32, body: Pat },
    Rep { kind: Kind, min: u32, max: u32, item: Pat },
    Atomic(Pat),
    Look { behind: bool, negate: bool, body: Pat },
    Failure,
    GroupRef(u32),
    Cond { group: u32, yes: Pat, no: Option<Pat> },
}

type Width = (u128, u128);

#[derive(Clone, Default)]
struct Pat {
    data: Vec<Node>,
    width: std::cell::Cell<Option<Width>>,
}

impl Pat {
    fn of(data: Vec<Node>) -> Pat { Pat { data, width: std::cell::Cell::new(None) } }
    fn push(&mut self, node: Node) { self.width.set(None); self.data.push(node); }
}

fn same_node(a: &Node, b: &Node) -> bool {
    match (a, b) {
        (Node::Lit(x), Node::Lit(y)) | (Node::NotLit(x), Node::NotLit(y)) => x == y,
        (Node::Any, Node::Any) | (Node::Failure, Node::Failure) => true,
        (Node::In(x), Node::In(y)) => x == y,
        (Node::At(x), Node::At(y)) | (Node::GroupRef(x), Node::GroupRef(y)) => x == y,
        _ => false,
    }
}

fn widths(pat: &Pat, groups: &[Option<Width>]) -> Width {
    if let Some(known) = pat.width.get() { return known; }
    let (mut lo, mut hi) = (0u128, 0u128);
    for node in &pat.data {
        match node {
            Node::Branch(items) => {
                let (mut i, mut j) = (MAXWIDTH, 0u128);
                for item in items {
                    let (l, h) = widths(item, groups);
                    i = i.min(l);
                    j = j.max(h);
                }
                lo += i;
                hi += j;
            }
            Node::Atomic(body) | Node::Sub { body, .. } => {
                let (i, j) = widths(body, groups);
                lo += i;
                hi += j;
            }
            Node::Rep { min, max, item, .. } => {
                let (i, j) = widths(item, groups);
                lo += i * u128::from(*min);
                if u128::from(*max) == MAXREPEAT && j != 0 { hi = MAXWIDTH; } else { hi += j * u128::from(*max); }
            }
            Node::Any | Node::In(_) | Node::Lit(_) | Node::NotLit(_) => { lo += 1; hi += 1; }
            Node::GroupRef(group) => {
                let (i, j) = groups[*group as usize].unwrap_or((0, 0));
                lo += i;
                hi += j;
            }
            Node::Cond { yes, no, .. } => {
                let (mut i, mut j) = widths(yes, groups);
                if let Some(other) = no {
                    let (l, h) = widths(other, groups);
                    i = i.min(l);
                    j = j.max(h);
                } else { i = 0; }
                lo += i;
                hi += j;
            }
            Node::At(_) | Node::Look { .. } | Node::Failure => {}
        }
    }
    let found = (lo.min(MAXWIDTH), hi.min(MAXWIDTH));
    pat.width.set(Some(found));
    found
}

#[derive(Clone, Copy, PartialEq)]
struct Tk { esc: bool, c: u32 }

struct Parser {
    s: Vec<u32>,
    index: usize,
    next: Option<Tk>,
    flags: u32,
    names: Vec<(String, u32)>,
    widths: Vec<Option<Width>>,
    behind: Option<usize>,
    cond_refs: Vec<u32>,
}

fn ch(c: char) -> u32 { c as u32 }
fn is_digit(c: u32) -> bool { (48..=57).contains(&c) }
fn is_oct(c: u32) -> bool { (48..=55).contains(&c) }
fn is_hex(c: u32) -> bool { is_digit(c) || (97..=102).contains(&c) || (65..=70).contains(&c) }
fn is_letter(c: u32) -> bool { (97..=122).contains(&c) || (65..=90).contains(&c) }
fn plain(t: Option<Tk>, wanted: char) -> bool { matches!(t, Some(Tk { esc: false, c }) if c == ch(wanted)) }

impl Parser {
    fn advance(&mut self) -> R<()> {
        let Some(&first) = self.s.get(self.index) else { self.next = None; return Ok(()); };
        let mut index = self.index;
        let mut token = Tk { esc: false, c: first };
        if first == 92 {
            index += 1;
            let Some(&second) = self.s.get(index) else { return Err(()); };
            token = Tk { esc: true, c: second };
        }
        self.index = index + 1;
        self.next = Some(token);
        Ok(())
    }
    fn get(&mut self) -> R<Option<Tk>> {
        let this = self.next;
        self.advance()?;
        Ok(this)
    }
    fn eat(&mut self, wanted: char) -> R<bool> {
        if plain(self.next, wanted) { self.advance()?; Ok(true) } else { Ok(false) }
    }
    fn tell(&self) -> usize {
        self.index - match self.next { None => 0, Some(t) => if t.esc { 2 } else { 1 } }
    }
    fn seek(&mut self, index: usize) -> R<()> { self.index = index; self.advance() }
    fn take_while(&mut self, count: usize, accept: fn(u32) -> bool) -> R<Vec<u32>> {
        let mut got = Vec::new();
        for _ in 0..count {
            match self.next {
                Some(Tk { esc: false, c }) if accept(c) => { got.push(c); self.advance()?; }
                _ => break,
            }
        }
        Ok(got)
    }
    /// Reads a name up to `end`; a name holding an escape or a non-ASCII letter is left to Python.
    fn name_until(&mut self, end: char) -> R<String> {
        let mut result = String::new();
        loop {
            let Some(t) = self.get()? else { return Err(()); };
            if t.esc { return Err(()); }
            if t.c == ch(end) { break; }
            if t.c >= 128 { return Err(()); }
            result.push(t.c as u8 as char);
        }
        Ok(result)
    }
    fn group_known(&self, group: u32) -> bool {
        (group as usize) < self.widths.len() && self.widths[group as usize].is_some()
    }
    fn lookbehind_ok(&self, group: u32) -> R<()> {
        if let Some(limit) = self.behind {
            if !self.group_known(group) || group as usize >= limit { return Err(()); }
        }
        Ok(())
    }
    fn identifier(name: &str) -> bool {
        let mut letters = name.chars();
        matches!(letters.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
            && letters.all(|c| c.is_ascii_alphanumeric() || c == '_')
    }

    fn class_escape(&mut self, esc: u32, text: bool) -> R<Member> {
        let simple = match esc {
            97 => Some(7), 98 => Some(8), 102 => Some(12), 110 => Some(10), 114 => Some(13),
            116 => Some(9), 118 => Some(11), 92 => Some(92), _ => None,
        };
        if let Some(code) = simple { return Ok(Member::Lit(code)); }
        let category = match esc {
            100 => Some(0), 68 => Some(1), 115 => Some(2), 83 => Some(3), 119 => Some(4), 87 => Some(5), _ => None,
        };
        if let Some(code) = category { return Ok(Member::Cat(code)); }
        if let Some(value) = self.numbered_escape(esc, text, true)? { return Ok(Member::Lit(value)); }
        if is_digit(esc) { return Err(()); }
        if is_letter(esc) { return Err(()); }
        Ok(Member::Lit(esc))
    }

    /// The `\x`, `\u`, `\U` and octal forms, shared by both escape readers.
    fn numbered_escape(&mut self, esc: u32, text: bool, in_class: bool) -> R<Option<u32>> {
        let (digits, accept): (usize, fn(u32) -> bool) = match esc {
            120 => (2, is_hex),
            117 if text => (4, is_hex),
            85 if text => (8, is_hex),
            _ => {
                if esc == 78 && text { return Err(()); }
                if esc == 48 || (in_class && is_oct(esc)) {
                    let more = self.take_while(2, is_oct)?;
                    let mut value = esc - 48;
                    for d in more { value = value * 8 + (d - 48); }
                    if in_class && value > 0o377 { return Err(()); }
                    return Ok(Some(value));
                }
                return Ok(None);
            }
        };
        let got = self.take_while(digits, accept)?;
        if got.len() != digits { return Err(()); }
        let mut value: u64 = 0;
        for d in got { value = value * 16 + u64::from(char::from_u32(d).and_then(|c| c.to_digit(16)).unwrap_or(0)); }
        if esc == 85 && value > 0x10ffff { return Err(()); }
        Ok(Some(value as u32))
    }

    fn escape(&mut self, esc: u32, text: bool) -> R<Node> {
        let found = match esc {
            65 => Some(Node::At(AT_BEGINNING_STRING)),
            98 => Some(Node::At(AT_BOUNDARY)),
            66 => Some(Node::At(AT_NON_BOUNDARY)),
            100 => Some(Node::In(vec![Member::Cat(0)])),
            68 => Some(Node::In(vec![Member::Cat(1)])),
            115 => Some(Node::In(vec![Member::Cat(2)])),
            83 => Some(Node::In(vec![Member::Cat(3)])),
            119 => Some(Node::In(vec![Member::Cat(4)])),
            87 => Some(Node::In(vec![Member::Cat(5)])),
            122 | 90 => Some(Node::At(AT_END_STRING)),
            97 => Some(Node::Lit(7)), 102 => Some(Node::Lit(12)), 110 => Some(Node::Lit(10)),
            114 => Some(Node::Lit(13)), 116 => Some(Node::Lit(9)), 118 => Some(Node::Lit(11)),
            92 => Some(Node::Lit(92)),
            _ => None,
        };
        if let Some(node) = found { return Ok(node); }
        if let Some(value) = self.numbered_escape(esc, text, false)? { return Ok(Node::Lit(value)); }
        if is_digit(esc) {
            let mut digits = vec![esc];
            if let Some(Tk { esc: false, c }) = self.next {
                if is_digit(c) {
                    self.advance()?;
                    digits.push(c);
                    if is_oct(esc) && is_oct(c) {
                        if let Some(Tk { esc: false, c: third }) = self.next {
                            if is_oct(third) {
                                self.advance()?;
                                let value = (esc - 48) * 64 + (c - 48) * 8 + (third - 48);
                                if value > 0o377 { return Err(()); }
                                return Ok(Node::Lit(value));
                            }
                        }
                    }
                }
            }
            let mut group = 0u32;
            for d in digits { group = group * 10 + (d - 48); }
            if (group as usize) < self.widths.len() {
                if !self.group_known(group) { return Err(()); }
                self.lookbehind_ok(group)?;
                return Ok(Node::GroupRef(group));
            }
            return Err(());
        }
        if is_letter(esc) { return Err(()); }
        Ok(Node::Lit(esc))
    }

    fn parse_sub(&mut self, mut verbose: bool, nested: usize, text: bool) -> R<Pat> {
        if nested > DEEPEST { return Err(()); }
        let mut items: Vec<Pat> = Vec::new();
        loop {
            let first = nested == 0 && items.is_empty();
            let item = self.parse(verbose, nested + 1, first, text)?;
            items.push(item);
            if !self.eat('|')? { break; }
            if nested == 0 { verbose = self.flags & VERBOSE != 0; }
        }
        if items.len() == 1 { return Ok(items.pop().unwrap()); }

        let mut sub = Pat::default();
        // all alternatives start the same way: move that part out of the branch
        loop {
            let mut prefix: Option<usize> = None;
            let mut shared = true;
            for (n, item) in items.iter().enumerate() {
                if item.data.is_empty() { shared = false; break; }
                match prefix {
                    None => prefix = Some(n),
                    Some(first) => if !same_node(&item.data[0], &items[first].data[0]) { shared = false; break; },
                }
            }
            if !shared { break; }
            let moved = items[0].data[0].clone();
            for item in items.iter_mut() { item.data.remove(0); item.width.set(None); }
            sub.push(moved);
        }
        // a branch of single characters is a character set
        let mut set: Vec<Member> = Vec::new();
        let mut all_sets = true;
        for item in &items {
            if item.data.len() != 1 { all_sets = false; break; }
            match &item.data[0] {
                Node::Lit(c) => set.push(Member::Lit(*c)),
                Node::In(members) if members[0] != Member::Negate => set.extend(members.iter().cloned()),
                _ => { all_sets = false; break; }
            }
        }
        if all_sets {
            sub.push(Node::In(unique(set)));
            return Ok(sub);
        }
        sub.push(Node::Branch(items));
        Ok(sub)
    }

    fn parse(&mut self, mut verbose: bool, nested: usize, first: bool, text: bool) -> R<Pat> {
        let mut sub = Pat::default();
        loop {
            let Some(this) = self.next else { break };
            if !this.esc && (this.c == ch('|') || this.c == ch(')')) { break; }
            self.advance()?;
            if verbose {
                if !this.esc && matches!(this.c, 32 | 9 | 10 | 13 | 11 | 12) { continue; }
                if !this.esc && this.c == ch('#') {
                    loop {
                        match self.get()? {
                            None => break,
                            Some(Tk { esc: false, c: 10 }) => break,
                            _ => {}
                        }
                    }
                    continue;
                }
            }
            if this.esc {
                let node = self.escape(this.c, text)?;
                sub.push(node);
                continue;
            }
            let c = this.c;
            if !matches!(char::from_u32(c), Some('.' | '[' | '{' | '(' | ')' | '*' | '+' | '?' | '^' | '$' | '|')) {
                sub.push(Node::Lit(c));
                continue;
            }
            match char::from_u32(c).unwrap() {
                '[' => {
                    let node = self.class(text)?;
                    sub.push(node);
                }
                '*' | '+' | '?' | '{' => {
                    let here = self.tell();
                    let (min, max): (u128, u128);
                    match char::from_u32(c).unwrap() {
                        '?' => { min = 0; max = 1; }
                        '*' => { min = 0; max = MAXREPEAT; }
                        '+' => { min = 1; max = MAXREPEAT; }
                        _ => {
                            if plain(self.next, '}') { sub.push(Node::Lit(c)); continue; }
                            let (mut lo, mut hi) = (String::new(), String::new());
                            while let Some(Tk { esc: false, c: d }) = self.next {
                                if !is_digit(d) { break; }
                                lo.push(d as u8 as char);
                                self.advance()?;
                            }
                            if self.eat(',')? {
                                while let Some(Tk { esc: false, c: d }) = self.next {
                                    if !is_digit(d) { break; }
                                    hi.push(d as u8 as char);
                                    self.advance()?;
                                }
                            } else { hi = lo.clone(); }
                            if !self.eat('}')? {
                                sub.push(Node::Lit(c));
                                self.seek(here)?;
                                continue;
                            }
                            let number = |digits: &str| -> R<u128> {
                                let mut value: u128 = 0;
                                for d in digits.chars() {
                                    value = (value * 10 + u128::from(d as u8 - b'0')).min(1 << 70);
                                }
                                if value >= MAXREPEAT { Err(()) } else { Ok(value) }
                            };
                            let mut low = 0;
                            let mut high = MAXREPEAT;
                            if !lo.is_empty() { low = number(&lo)?; }
                            if !hi.is_empty() {
                                high = number(&hi)?;
                                if high < low { return Err(()); }
                            }
                            min = low;
                            max = high;
                        }
                    }
                    let Some(last) = sub.data.last() else { return Err(()) };
                    if matches!(last, Node::At(_) | Node::Rep { .. }) { return Err(()); }
                    let item = match last {
                        Node::Sub { group: None, add: 0, del: 0, body } => body.clone(),
                        other => Pat::of(vec![other.clone()]),
                    };
                    let kind = if self.eat('?')? { Kind::Min } else if self.eat('+')? { Kind::Possessive } else { Kind::Max };
                    sub.width.set(None);
                    let at = sub.data.len() - 1;
                    sub.data[at] = Node::Rep { kind, min: min as u32, max: max as u32, item };
                }
                '.' => sub.push(Node::Any),
                '(' => self.group(&mut sub, &mut verbose, nested, first, text)?,
                '^' => sub.push(Node::At(AT_BEGINNING)),
                '$' => sub.push(Node::At(AT_END)),
                _ => return Err(()),
            }
        }
        // unpack non-capturing groups
        let mut i = sub.data.len();
        while i > 0 {
            i -= 1;
            if let Node::Sub { group: None, add: 0, del: 0, body } = &sub.data[i] {
                let inner = body.data.clone();
                sub.data.splice(i..i + 1, inner);
                sub.width.set(None);
            }
        }
        Ok(sub)
    }

    fn class(&mut self, text: bool) -> R<Node> {
        let mut set: Vec<Member> = Vec::new();
        if plain(self.next, '[') { return Err(()); }
        let negate = self.eat('^')?;
        loop {
            let Some(this) = self.get()? else { return Err(()) };
            let one;
            if !this.esc && this.c == ch(']') && !set.is_empty() { break; }
            if this.esc {
                one = self.class_escape(this.c, text)?;
            } else {
                if !set.is_empty() && matches!(char::from_u32(this.c), Some('-' | '&' | '~' | '|')) && plain(self.next, char::from_u32(this.c).unwrap()) {
                    return Err(());
                }
                one = Member::Lit(this.c);
            }
            if self.eat('-')? {
                let Some(that) = self.get()? else { return Err(()) };
                if !that.esc && that.c == ch(']') {
                    set.push(one);
                    set.push(Member::Lit(ch('-')));
                    break;
                }
                let other = if that.esc { self.class_escape(that.c, text)? } else {
                    if that.c == ch('-') { return Err(()); }
                    Member::Lit(that.c)
                };
                let (Member::Lit(lo), Member::Lit(hi)) = (&one, &other) else { return Err(()) };
                if hi < lo { return Err(()); }
                set.push(Member::Range(*lo, *hi));
            } else {
                set.push(one);
            }
        }
        let mut set = unique(set);
        if set.len() == 1 {
            if let Member::Lit(c) = set[0] {
                return Ok(if negate { Node::NotLit(c) } else { Node::Lit(c) });
            }
        }
        if negate { set.insert(0, Member::Negate); }
        Ok(Node::In(set))
    }

    fn open_group(&mut self, name: Option<String>) -> R<u32> {
        let gid = self.widths.len();
        self.widths.push(None);
        if self.widths.len() > MAXGROUPS { return Err(()); }
        if let Some(name) = name {
            if self.names.iter().any(|(known, _)| *known == name) { return Err(()); }
            self.names.push((name, gid as u32));
        }
        Ok(gid as u32)
    }

    /// One `(`: pushes whatever it makes onto `sub`.
    fn group(&mut self, sub: &mut Pat, verbose: &mut bool, nested: usize, first: bool, text: bool) -> R<()> {
        let mut capture = true;
        let mut atomic = false;
        let mut name: Option<String> = None;
        let mut add = 0u32;
        let mut del = 0u32;
        if self.eat('?')? {
            let Some(Tk { esc: false, c: kind }) = self.get()? else { return Err(()) };
            match char::from_u32(kind).unwrap_or('\0') {
                'P' => {
                    if self.eat('<')? {
                        let found = self.name_until('>')?;
                        if !Self::identifier(&found) { return Err(()); }
                        name = Some(found);
                    } else if self.eat('=')? {
                        let found = self.name_until(')')?;
                        if !Self::identifier(&found) { return Err(()); }
                        let Some(&(_, gid)) = self.names.iter().find(|(known, _)| *known == found) else { return Err(()) };
                        if !self.group_known(gid) { return Err(()); }
                        self.lookbehind_ok(gid)?;
                        sub.push(Node::GroupRef(gid));
                        return Ok(());
                    } else { return Err(()); }
                }
                ':' => capture = false,
                '#' => {
                    loop {
                        if self.next.is_none() { return Err(()); }
                        if matches!(self.get()?, Some(Tk { esc: false, c }) if c == ch(')')) { break; }
                    }
                    return Ok(());
                }
                '=' | '!' | '<' => {
                    let mut kind = kind;
                    let mut behind = false;
                    let mut saved: Option<Option<usize>> = None;
                    if kind == ch('<') {
                        let Some(Tk { esc: false, c: second }) = self.get()? else { return Err(()) };
                        if second != ch('=') && second != ch('!') { return Err(()); }
                        kind = second;
                        behind = true;
                        saved = Some(self.behind);
                        if self.behind.is_none() { self.behind = Some(self.widths.len()); }
                    }
                    let body = self.parse_sub(*verbose, nested + 1, text)?;
                    if let Some(old) = saved { if old.is_none() { self.behind = None; } }
                    if !self.eat(')')? { return Err(()); }
                    if kind == ch('=') {
                        sub.push(Node::Look { behind, negate: false, body });
                    } else if !body.data.is_empty() {
                        sub.push(Node::Look { behind, negate: true, body });
                    } else {
                        sub.push(Node::Failure);
                    }
                    return Ok(());
                }
                '(' => {
                    let reference = self.name_until(')')?;
                    let group;
                    if !reference.is_empty() && reference.bytes().all(|b| b.is_ascii_digit()) {
                        let mut value: usize = 0;
                        for b in reference.bytes() { value = (value * 10 + usize::from(b - b'0')).min(1 << 40); }
                        if value == 0 || value >= MAXGROUPS { return Err(()); }
                        group = value as u32;
                        if !self.cond_refs.contains(&group) { self.cond_refs.push(group); }
                    } else {
                        if !Self::identifier(&reference) { return Err(()); }
                        let Some(&(_, gid)) = self.names.iter().find(|(known, _)| *known == reference) else { return Err(()) };
                        group = gid;
                    }
                    self.lookbehind_ok(group)?;
                    let yes = self.parse(*verbose, nested + 1, false, text)?;
                    let no = if self.eat('|')? {
                        let other = self.parse(*verbose, nested + 1, false, text)?;
                        if plain(self.next, '|') { return Err(()); }
                        Some(other)
                    } else { None };
                    if !self.eat(')')? { return Err(()); }
                    sub.push(Node::Cond { group, yes, no });
                    return Ok(());
                }
                '>' => { capture = false; atomic = true; }
                'i' | 'm' | 's' | 'x' | 'a' | 'u' | '-' => {
                    match self.flags_group(kind, text)? {
                        None => {
                            if !first || !sub.data.is_empty() { return Err(()); }
                            *verbose = self.flags & VERBOSE != 0;
                            return Ok(());
                        }
                        Some((on, off)) => { add = on; del = off; capture = false; }
                    }
                }
                _ => return Err(()),
            }
        }
        let group = if capture { Some(self.open_group(name)?) } else { None };
        let inner_verbose = (*verbose || add & VERBOSE != 0) && del & VERBOSE == 0;
        let body = self.parse_sub(inner_verbose, nested + 1, text)?;
        if !self.eat(')')? { return Err(()); }
        if let Some(gid) = group { self.widths[gid as usize] = Some(widths(&body, &self.widths)); }
        if atomic { sub.push(Node::Atomic(body)); } else { sub.push(Node::Sub { group, add, del, body }); }
        Ok(())
    }

    /// The inline flags after `(?`; `None` for global flags, which are added to the pattern's own.
    fn flags_group(&mut self, first: u32, text: bool) -> R<Option<(u32, u32)>> {
        fn flag_of(c: u32) -> Option<u32> {
            match char::from_u32(c)? { 'i' => Some(IGNORECASE), 'm' => Some(MULTILINE), 's' => Some(DOTALL), 'x' => Some(VERBOSE), 'a' => Some(ASCII), 'u' => Some(UNICODE), _ => None }
        }
        let mut current = first;
        let mut on = 0u32;
        let mut off = 0u32;
        if current != ch('-') {
            loop {
                let flag = flag_of(current).ok_or(())?;
                if !text && flag == UNICODE { return Err(()); }
                on |= flag;
                if flag & TYPE_FLAGS != 0 && on & TYPE_FLAGS != flag { return Err(()); }
                let Some(Tk { esc: false, c }) = self.get()? else { return Err(()) };
                current = c;
                if current == ch(')') || current == ch('-') || current == ch(':') { break; }
                if flag_of(current).is_none() { return Err(()); }
            }
        }
        if current == ch(')') {
            self.flags |= on;
            return Ok(None);
        }
        if current == ch('-') {
            let Some(Tk { esc: false, c }) = self.get()? else { return Err(()) };
            current = c;
            flag_of(current).ok_or(())?;
            loop {
                let flag = flag_of(current).ok_or(())?;
                if flag & TYPE_FLAGS != 0 { return Err(()); }
                off |= flag;
                let Some(Tk { esc: false, c }) = self.get()? else { return Err(()) };
                current = c;
                if current == ch(':') { break; }
                if flag_of(current).is_none() { return Err(()); }
            }
        }
        if on & off != 0 { return Err(()); }
        Ok(Some((on, off)))
    }
}

fn unique(items: Vec<Member>) -> Vec<Member> {
    let mut out: Vec<Member> = Vec::with_capacity(items.len());
    for item in items { if !out.contains(&item) { out.push(item); } }
    out
}

struct Fold<'a> { unicode: bool, case: &'a Case<'a> }

impl Fold<'_> {
    fn lower(&self, c: u32) -> u32 {
        if self.unicode { (self.case.lower)(c) } else if (65..=90).contains(&c) { c + 32 } else { c }
    }
    fn cased(&self, c: u32) -> bool {
        if self.unicode { (self.case.cased)(c) } else { is_letter(c) }
    }
    fn extra(&self, c: u32) -> Option<&'static [u32]> {
        if self.unicode { cases::extra(c) } else { None }
    }
}

fn set_bit(map: &mut [u8], at: u32) -> bool {
    match map.get_mut(at as usize) { Some(slot) => { *slot = 1; true } None => false }
}

fn bitmap(bits: &[u8]) -> Vec<u32> {
    bits.chunks(32).map(|chunk| chunk.iter().enumerate().fold(0u32, |word, (n, &b)| word | (u32::from(b) << n))).collect()
}

fn find(map: &[u8], wanted: u8, from: usize) -> Option<usize> {
    map.get(from..)?.iter().position(|&b| b == wanted).map(|n| n + from)
}

fn optimize_charset(charset: &[Member], fold: Option<&Fold>) -> (Vec<Member>, bool) {
    enum Step { Done, Index, Return(Vec<Member>) }
    let mut out: Vec<Member> = Vec::new();
    let mut tail: Vec<Member> = Vec::new();
    let mut map = vec![0u8; 256];
    let mut hascased = false;
    for member in charset {
        let mut current = member.clone();
        loop {
            let step = match &mut current {
                Member::Lit(av) => match fold {
                    Some(f) => {
                        *av = f.lower(*av);
                        if !set_bit(&mut map, *av) { Step::Index } else {
                            let mut step = Step::Done;
                            if let Some(more) = f.extra(*av) {
                                for &k in more { if !set_bit(&mut map, k) { step = Step::Index; break; } }
                            }
                            if matches!(step, Step::Done) && !hascased && f.cased(*av) { hascased = true; }
                            step
                        }
                    }
                    None => if set_bit(&mut map, *av) { Step::Done } else { Step::Index },
                },
                Member::Range(lo, hi) => {
                    let (lo, hi) = (*lo, *hi);
                    let mut step = Step::Done;
                    match fold {
                        Some(f) => {
                            'run: for c in lo..=hi {
                                let i = f.lower(c);
                                if !set_bit(&mut map, i) { step = Step::Index; break; }
                                if let Some(more) = f.extra(i) {
                                    for &k in more { if !set_bit(&mut map, k) { step = Step::Index; break 'run; } }
                                }
                            }
                            if matches!(step, Step::Done) && !hascased { hascased = (lo..=hi).any(|c| f.cased(c)); }
                        }
                        None => for i in lo..=hi { if !set_bit(&mut map, i) { step = Step::Index; break; } },
                    }
                    step
                }
                Member::Negate => { out.push(Member::Negate); Step::Done }
                Member::Cat(av) if !tail.is_empty() && tail.contains(&Member::Cat(*av ^ 1)) => {
                    Step::Return(if out.is_empty() { vec![Member::Negate] } else { Vec::new() })
                }
                other => { tail.push(other.clone()); Step::Done }
            };
            match step {
                Step::Done => break,
                Step::Return(found) => return (found, false),
                Step::Index => {
                    if map.len() == 256 { map.resize(65536, 0); continue; }
                    if let Some(f) = fold {
                        match &current {
                            Member::Range(lo, hi) => {
                                if f.unicode { current = Member::RangeUni(*lo, *hi); }
                                hascased = true;
                            }
                            Member::Lit(av) => { if !hascased && f.cased(*av) { hascased = true; } }
                            _ => {}
                        }
                    }
                    tail.push(current);
                    break;
                }
            }
        }
    }
    // compress the character map into at most two runs
    let mut runs: Option<Vec<(usize, usize)>> = Some(Vec::new());
    let mut q = 0usize;
    loop {
        let Some(p) = find(&map, 1, q) else { break };
        if runs.as_ref().unwrap().len() >= 2 { runs = None; break; }
        match find(&map, 0, p) {
            None => { runs.as_mut().unwrap().push((p, map.len())); break; }
            Some(end) => { runs.as_mut().unwrap().push((p, end)); q = end; }
        }
    }
    if let Some(runs) = runs {
        for (p, q) in runs {
            if q - p == 1 { out.push(Member::Lit(p as u32)); } else { out.push(Member::Range(p as u32, (q - 1) as u32)); }
        }
        out.extend(tail);
        if hascased || out.len() < charset.len() { return (out, hascased); }
        return (charset.to_vec(), hascased);
    }
    if map.len() == 256 {
        out.push(Member::Charset(bitmap(&map)));
        out.extend(tail);
        return (out, hascased);
    }
    let mut seen: Vec<&[u8]> = Vec::new();
    let mut mapping = vec![0u8; 256];
    let mut data: Vec<u8> = Vec::new();
    for i in (0..65536).step_by(256) {
        let chunk = &map[i..i + 256];
        match seen.iter().position(|known| *known == chunk) {
            Some(block) => mapping[i / 256] = block as u8,
            None => { mapping[i / 256] = seen.len() as u8; seen.push(chunk); data.extend_from_slice(chunk); }
        }
    }
    let mut words: Vec<u32> = vec![seen.len() as u32];
    words.extend(mapping.chunks(4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])));
    words.extend(bitmap(&data));
    out.push(Member::Big(words));
    out.extend(tail);
    (out, hascased)
}

fn combine(flags: u32, add: u32, del: u32) -> u32 {
    let flags = if add & TYPE_FLAGS != 0 { flags & !TYPE_FLAGS } else { flags };
    (flags | add) & !del
}

struct Compiler<'a> { code: Vec<u32>, case: &'a Case<'a>, groups: Vec<Option<Width>> }

fn simple(pat: &Pat) -> bool {
    if pat.data.len() != 1 { return false; }
    match &pat.data[0] {
        Node::Sub { group, body, .. } => group.is_none() && simple(body),
        Node::Lit(_) | Node::NotLit(_) | Node::Any | Node::In(_) => true,
        _ => false,
    }
}

impl<'a> Compiler<'a> {
    fn fold(&self, flags: u32) -> Option<Fold<'a>> {
        if flags & IGNORECASE != 0 { Some(Fold { unicode: flags & UNICODE != 0, case: self.case }) } else { None }
    }
    fn emit(&mut self, word: u32) { self.code.push(word); }
    fn hole(&mut self) -> usize { self.code.push(0); self.code.len() - 1 }
    fn patch(&mut self, at: usize) { self.code[at] = (self.code.len() - at) as u32; }

    fn charset(&mut self, charset: &[Member], flags: u32) {
        for member in charset {
            match member {
                Member::Negate => self.emit(op::NEGATE),
                Member::Lit(c) => { self.emit(op::LITERAL); self.emit(*c); }
                Member::Range(lo, hi) => { self.emit(op::RANGE); self.emit(*lo); self.emit(*hi); }
                Member::RangeUni(lo, hi) => { self.emit(op::RANGE_UNI_IGNORE); self.emit(*lo); self.emit(*hi); }
                Member::Charset(words) => { self.emit(op::CHARSET); self.code.extend_from_slice(words); }
                Member::Big(words) => { self.emit(op::BIGCHARSET); self.code.extend_from_slice(words); }
                Member::Cat(c) => { self.emit(op::CATEGORY); self.emit(if flags & UNICODE != 0 { c + 10 } else { *c }); }
            }
        }
        self.emit(op::FAILURE);
    }

    fn pattern(&mut self, pat: &Pat, flags: u32) -> R<()> {
        let fold = self.fold(flags);
        for node in &pat.data {
            match node {
                Node::Lit(av) | Node::NotLit(av) => {
                    let literal = matches!(node, Node::Lit(_));
                    let plain_op = if literal { op::LITERAL } else { op::NOT_LITERAL };
                    match &fold {
                        None => { self.emit(plain_op); self.emit(*av); }
                        Some(f) if !f.cased(*av) => { self.emit(plain_op); self.emit(*av); }
                        Some(f) => {
                            let lo = f.lower(*av);
                            if !f.unicode {
                                self.emit(if literal { op::LITERAL_IGNORE } else { op::NOT_LITERAL_IGNORE });
                                self.emit(lo);
                            } else if f.extra(lo).is_none() {
                                self.emit(if literal { op::LITERAL_UNI_IGNORE } else { op::NOT_LITERAL_UNI_IGNORE });
                                self.emit(lo);
                            } else {
                                self.emit(op::IN_UNI_IGNORE);
                                let skip = self.hole();
                                if !literal { self.emit(op::NEGATE); }
                                self.emit(op::LITERAL);
                                self.emit(lo);
                                for &k in f.extra(lo).unwrap() { self.emit(op::LITERAL); self.emit(k); }
                                self.emit(op::FAILURE);
                                self.patch(skip);
                            }
                        }
                    }
                }
                Node::In(set) => {
                    let (charset, hascased) = optimize_charset(set, fold.as_ref());
                    if charset.is_empty() { self.emit(op::FAILURE); }
                    else if charset == [Member::Negate] { self.emit(op::ANY_ALL); }
                    else {
                        if !hascased { self.emit(op::IN); }
                        else if !fold.as_ref().is_some_and(|f| f.unicode) { self.emit(op::IN_IGNORE); }
                        else { self.emit(op::IN_UNI_IGNORE); }
                        let skip = self.hole();
                        self.charset(&charset, flags);
                        self.patch(skip);
                    }
                }
                Node::Any => self.emit(if flags & DOTALL != 0 { op::ANY_ALL } else { op::ANY }),
                Node::Rep { kind, min, max, item } => {
                    let (open, close, one) = match kind {
                        Kind::Min => (op::REPEAT, op::MIN_UNTIL, op::MIN_REPEAT_ONE),
                        Kind::Max => (op::REPEAT, op::MAX_UNTIL, op::REPEAT_ONE),
                        Kind::Possessive => (op::POSSESSIVE_REPEAT, op::SUCCESS, op::POSSESSIVE_REPEAT_ONE),
                    };
                    if simple(item) {
                        self.emit(one);
                        let skip = self.hole();
                        self.emit(*min);
                        self.emit(*max);
                        self.pattern(item, flags)?;
                        self.emit(op::SUCCESS);
                        self.patch(skip);
                    } else {
                        self.emit(open);
                        let skip = self.hole();
                        self.emit(*min);
                        self.emit(*max);
                        self.pattern(item, flags)?;
                        self.patch(skip);
                        self.emit(close);
                    }
                }
                Node::Sub { group, add, del, body } => {
                    if let Some(g) = group { self.emit(op::MARK); self.emit((g - 1) * 2); }
                    self.pattern(body, combine(flags, *add, *del))?;
                    if let Some(g) = group { self.emit(op::MARK); self.emit((g - 1) * 2 + 1); }
                }
                Node::Atomic(body) => {
                    self.emit(op::ATOMIC_GROUP);
                    let skip = self.hole();
                    self.pattern(body, flags)?;
                    self.emit(op::SUCCESS);
                    self.patch(skip);
                }
                Node::Failure => self.emit(op::FAILURE),
                Node::Look { behind, negate, body } => {
                    self.emit(if *negate { op::ASSERT_NOT } else { op::ASSERT });
                    let skip = self.hole();
                    if !behind { self.emit(0); } else {
                        let (lo, hi) = widths(body, &self.groups);
                        if lo > MAXCODE || lo != hi { return Err(()); }
                        self.emit(lo as u32);
                    }
                    self.pattern(body, flags)?;
                    self.emit(op::SUCCESS);
                    self.patch(skip);
                }
                Node::At(av) => {
                    self.emit(op::AT);
                    let mut av = *av;
                    if flags & MULTILINE != 0 {
                        if av == AT_BEGINNING { av = AT_BEGINNING_LINE; } else if av == AT_END { av = AT_END_LINE; }
                    }
                    if flags & UNICODE != 0 {
                        if av == AT_BOUNDARY { av = AT_UNI_BOUNDARY; } else if av == AT_NON_BOUNDARY { av = AT_UNI_NON_BOUNDARY; }
                    }
                    self.emit(av);
                }
                Node::Branch(items) => {
                    self.emit(op::BRANCH);
                    let mut tails = Vec::new();
                    for item in items {
                        let skip = self.hole();
                        self.pattern(item, flags)?;
                        self.emit(op::JUMP);
                        tails.push(self.hole());
                        self.patch(skip);
                    }
                    self.emit(op::FAILURE);
                    for at in tails { self.patch(at); }
                }
                Node::GroupRef(av) => {
                    self.emit(match &fold {
                        None => op::GROUPREF,
                        Some(f) if !f.unicode => op::GROUPREF_IGNORE,
                        Some(_) => op::GROUPREF_UNI_IGNORE,
                    });
                    self.emit(av - 1);
                }
                Node::Cond { group, yes, no } => {
                    self.emit(op::GROUPREF_EXISTS);
                    self.emit(group - 1);
                    let skip_yes = self.hole();
                    self.pattern(yes, flags)?;
                    match no {
                        Some(other) if !other.data.is_empty() => {
                            self.emit(op::JUMP);
                            let skip_no = self.hole();
                            self.code[skip_yes] = (self.code.len() - skip_yes + 1) as u32;
                            self.pattern(other, flags)?;
                            self.patch(skip_no);
                        }
                        _ => self.code[skip_yes] = (self.code.len() - skip_yes + 1) as u32,
                    }
                }
            }
        }
        Ok(())
    }

    fn literal_prefix(&self, pat: &Pat, flags: u32) -> (Vec<u32>, Option<usize>, bool) {
        let fold = self.fold(flags);
        let mut prefix: Vec<u32> = Vec::new();
        let mut skip: Option<usize> = None;
        for node in &pat.data {
            match node {
                Node::Lit(av) => {
                    if fold.as_ref().is_some_and(|f| f.cased(*av)) { return (prefix, skip, false); }
                    prefix.push(*av);
                }
                Node::Sub { group, add, del, body } => {
                    let inner_flags = combine(flags, *add, *del);
                    let (inner, inner_skip, got_all) = self.literal_prefix(body, inner_flags);
                    if skip.is_none() {
                        if group.is_some() { skip = Some(prefix.len()); }
                        else if let Some(s) = inner_skip { skip = Some(prefix.len() + s); }
                    }
                    prefix.extend(inner);
                    if !got_all { return (prefix, skip, false); }
                }
                _ => return (prefix, skip, false),
            }
        }
        (prefix, skip, true)
    }

    fn charset_prefix(&self, pat: &Pat, flags: u32) -> Option<Vec<Member>> {
        let mut pat = pat;
        let mut flags = flags;
        loop {
            let first = pat.data.first()?;
            match first {
                Node::Sub { add, del, body, .. } => { flags = combine(flags, *add, *del); pat = body; }
                _ => break,
            }
        }
        let fold = self.fold(flags);
        match &pat.data[0] {
            Node::Lit(av) => {
                if fold.as_ref().is_some_and(|f| f.cased(*av)) { return None; }
                Some(vec![Member::Lit(*av)])
            }
            Node::Branch(items) => {
                let mut set = Vec::new();
                for item in items {
                    match item.data.first() {
                        Some(Node::Lit(av)) if !fold.as_ref().is_some_and(|f| f.cased(*av)) => set.push(Member::Lit(*av)),
                        _ => return None,
                    }
                }
                Some(set)
            }
            Node::In(set) => {
                if let Some(f) = &fold {
                    for member in set {
                        match member {
                            Member::Lit(av) => if f.cased(*av) { return None; },
                            Member::Range(lo, hi) => {
                                if *hi > 0xffff { return None; }
                                if (*lo..=*hi).any(|c| f.cased(c)) { return None; }
                            }
                            _ => {}
                        }
                    }
                }
                Some(set.clone())
            }
            _ => None,
        }
    }

    fn info(&mut self, pat: &Pat, flags: u32) {
        let (lo, mut hi) = widths(pat, &self.groups);
        if hi > MAXCODE { hi = MAXCODE; }
        if lo == 0 {
            self.code.extend_from_slice(&[op::INFO, 4, 0, lo as u32, hi as u32]);
            return;
        }
        let (mut prefix, mut prefix_skip, got_all) = self.literal_prefix(pat, flags);
        let mut charset: Option<Vec<Member>> = None;
        if prefix.is_empty() {
            if let Some(found) = self.charset_prefix(pat, flags) {
                if !found.is_empty() {
                    let (optimized, _) = optimize_charset(&found, None);
                    if optimized != [Member::Negate] && !optimized.is_empty() { charset = Some(optimized); }
                }
            }
        }
        self.emit(op::INFO);
        let skip = self.hole();
        let mut mask = 0;
        if !prefix.is_empty() {
            mask = 1;
            if prefix_skip.is_none() && got_all { mask |= 2; }
        } else if charset.as_ref().is_some_and(|c| !c.is_empty()) {
            mask |= 4;
        }
        self.emit(mask);
        if lo < MAXCODE { self.emit(lo as u32); } else {
            self.emit(MAXCODE as u32);
            prefix.truncate(MAXCODE as usize);
        }
        self.emit(hi as u32);
        if !prefix.is_empty() {
            self.emit(prefix.len() as u32);
            let skipped = *prefix_skip.get_or_insert(prefix.len());
            self.emit(skipped as u32);
            self.code.extend_from_slice(&prefix);
            let mut table = vec![0u32; prefix.len()];
            for i in 1..prefix.len() {
                let mut idx = table[i - 1] as usize;
                loop {
                    if prefix[i] == prefix[idx] { table[i] = idx as u32 + 1; break; }
                    if idx == 0 { table[i] = 0; break; }
                    idx = table[idx - 1] as usize;
                }
            }
            self.code.extend_from_slice(&table);
        } else if let Some(found) = charset {
            self.charset(&found, flags);
        }
        self.patch(skip);
    }
}

pub fn compile(pattern: &[u32], bytes: bool, flags: i64, case: &Case) -> Option<Compiled> {
    let flags = u32::try_from(flags).ok()?;
    if flags & (LOCALE | DEBUG) != 0 || flags > 0x1ff { return None; }
    if bytes && (flags & UNICODE != 0) { return None; }
    if !bytes && flags & ASCII != 0 && flags & UNICODE != 0 { return None; }
    if pattern.len() > 4_000_000 { return None; }
    let text = !bytes;
    let mut parser = Parser {
        s: pattern.to_vec(), index: 0, next: None, flags, names: Vec::new(),
        widths: vec![None], behind: None, cond_refs: Vec::new(),
    };
    parser.advance().ok()?;
    let body = parser.parse_sub(flags & VERBOSE != 0, 0, text).ok()?;
    if parser.flags & LOCALE != 0 { return None; }
    if text && parser.flags & ASCII == 0 { parser.flags |= UNICODE; }
    if text && parser.flags & ASCII != 0 && parser.flags & UNICODE != 0 { return None; }
    if bytes && parser.flags & UNICODE != 0 { return None; }
    if parser.next.is_some() { return None; }
    if parser.cond_refs.iter().any(|&g| g as usize >= parser.widths.len()) { return None; }
    let state_flags = parser.flags;
    let mut compiler = Compiler { code: Vec::new(), case, groups: parser.widths.clone() };
    let all = state_flags | flags;
    compiler.info(&body, all);
    compiler.pattern(&body, all).ok()?;
    compiler.emit(op::SUCCESS);
    if compiler.code.iter().any(|&w| u128::from(w) > MAXREPEAT) { return None; }
    let groups = parser.widths.len();
    let mut indexgroup = vec![None; groups];
    for (name, gid) in &parser.names { indexgroup[*gid as usize] = Some(name.clone()); }
    Some(Compiled {
        flags: i64::from(flags | state_flags),
        code: compiler.code,
        groups: groups - 1,
        names: parser.names,
        indexgroup,
    })
}
