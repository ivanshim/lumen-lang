// The definition, read as data: a table of labels, each with the shape it
// must have, and a few tables derived from it for the stages. Everything a
// language spells comes through here; the kernel names nothing itself.

use std::collections::{HashMap, HashSet};

use serde_json::{Map, Value as Json};

use crate::tree::{Native, Op};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Shape {
    Words,
    Flag,
    Count,
    TextOrNull,
    Tiers,
    Text,
}

use Shape::*;

/// Every label and its shape, in the order the files carry them.
const SCHEMA: &[(&str, Shape)] = &[
    ("format_version", Count), ("language", Text), ("extensions", Words),
    ("lexical.comment_line", Words), ("lexical.comment_block.open", Words), ("lexical.comment_block.close", Words),
    ("lexical.string_quotes", Words), ("lexical.raw_quotes", Words), ("lexical.string_escapes", Words),
    ("lexical.prologue", Words), ("lexical.name_quote", Words),
    ("lexical.number.decimal_point", Words), ("lexical.number.base_marker", Words),
    ("lexical.number.exponent_marker", Words), ("lexical.number.hex_prefix", Words),
    ("lexical.keywords_case_insensitive", Flag),
    ("identifier.unicode", Flag), ("identifier.variable_prefix", Words), ("identifier.case_insensitive", Flag),
    ("block.style", Text), ("block.open", Words), ("block.close", Words), ("block.intro", Words), ("block.indent_size", Count),
    ("stmt.terminator", Words),
    ("syntax.group.open", Words), ("syntax.group.close", Words),
    ("syntax.call.open", Words), ("syntax.call.separator", Words), ("syntax.call.close", Words), ("syntax.call.label", Words),
    ("syntax.array.open", Words), ("syntax.array.separator", Words), ("syntax.array.close", Words),
    ("syntax.map.open", Words), ("syntax.map.separator", Words), ("syntax.map.pair", Words), ("syntax.map.close", Words),
    ("literal.true", Words), ("literal.false", Words), ("literal.null", Words),
    ("op.precedence", Tiers), ("op.right_associative", Words),
    ("op.add", Words), ("op.sub", Words), ("op.mul", Words), ("op.div", Words), ("op.div.result", TextOrNull),
    ("op.quot", Words), ("op.rem", Words), ("op.pow", Words),
    ("op.eq", Words), ("op.ne", Words), ("op.lt", Words), ("op.le", Words), ("op.gt", Words), ("op.ge", Words),
    ("op.and", Words), ("op.or", Words), ("op.not", Words), ("op.negate", Words), ("op.concat", Words), ("op.range", Words),
    ("op.index.open", Words), ("op.index.close", Words), ("op.index.strings", Flag), ("op.pipe", Words),
    ("stmt.assign", Words), ("stmt.let", Words), ("stmt.let.mutable", Words), ("stmt.let.annotation", Words),
    ("stmt.let.type_first", Flag),
    ("stmt.if", Words), ("stmt.elif", Words), ("stmt.else", Words), ("stmt.while", Words), ("stmt.until", Words),
    ("stmt.for", Words), ("stmt.for.in", Words),
    ("stmt.foreach", Words), ("stmt.foreach.as", Words), ("stmt.foreach.pair", Words),
    ("stmt.return", Words), ("stmt.break", Words), ("stmt.continue", Words),
    ("stmt.function", Words), ("stmt.function.returns", Words), ("stmt.function.result_by_name", Flag),
    ("stmt.pass", Words), ("stmt.emit", Words),
    ("stack.dup", Words), ("stack.drop", Words), ("stack.swap", Words), ("stack.over", Words), ("stack.rot", Words),
    ("stack.eval", Words), ("stack.program.open", Words), ("stack.program.close", Words),
    ("builtin.emit", Words), ("builtin.print", Words), ("builtin.write", Words), ("builtin.print.placeholder", Words),
    ("builtin.len", Words), ("builtin.char_at", Words), ("builtin.ord", Words), ("builtin.chr", Words),
    ("builtin.typeof", Words), ("builtin.error", Words), ("builtin.extern", Words), ("builtin.range", Words),
    ("builtin.real", Words), ("builtin.num", Words), ("builtin.den", Words), ("builtin.push", Words),
    ("builtin.get", Words), ("builtin.put", Words), ("builtin.precision", Words),
    ("builtin.to_string", Words), ("builtin.to_int", Words), ("builtin.to_real", Words),
    ("system.args", Words), ("system.memoization", Words), ("system.real_default_precision", Words), ("system.entry", Words),
    ("system.kind.integer", Words), ("system.kind.rational", Words), ("system.kind.real", Words),
    ("system.kind.string", Words), ("system.kind.boolean", Words), ("system.kind.array", Words), ("system.kind.null", Words),
];

/// Labels with no meaning in this kernel; they must be empty.
const MUST_BE_EMPTY: &[&str] = &[
    "syntax.map.open", "syntax.map.separator", "syntax.map.pair", "syntax.map.close",
    "stmt.foreach", "stmt.foreach.as", "stmt.foreach.pair", "stmt.emit",
];

/// The builtin each label names.
pub const NATIVES: &[(&str, Native)] = &[
    ("builtin.emit", Native::Emit), ("builtin.print", Native::Print), ("builtin.write", Native::Write),
    ("builtin.len", Native::Len), ("builtin.char_at", Native::CharAt), ("builtin.ord", Native::Ord),
    ("builtin.chr", Native::Chr), ("builtin.typeof", Native::Kind), ("builtin.error", Native::Error),
    ("builtin.extern", Native::Extern), ("builtin.range", Native::Range), ("builtin.real", Native::Real),
    ("builtin.precision", Native::Precision), ("builtin.to_string", Native::ToString),
    ("builtin.to_int", Native::ToInt), ("builtin.to_real", Native::ToReal), ("builtin.num", Native::Num),
    ("builtin.den", Native::Den), ("builtin.push", Native::Push), ("builtin.get", Native::Get), ("builtin.put", Native::Put),
];

/// The binary operator each label names (`op.div` decided by `op.div.result`).
pub const BINARY: &[(&str, Op)] = &[
    ("op.add", Op::Add), ("op.sub", Op::Sub), ("op.mul", Op::Mul), ("op.div", Op::Div), ("op.quot", Op::Quot),
    ("op.rem", Op::Rem), ("op.pow", Op::Pow), ("op.eq", Op::Eq), ("op.ne", Op::Ne), ("op.lt", Op::Lt),
    ("op.le", Op::Le), ("op.gt", Op::Gt), ("op.ge", Op::Ge), ("op.and", Op::And), ("op.or", Op::Or),
    ("op.concat", Op::Concat),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Style {
    Indent,
    Brace,
    Keyword,
    Postfix,
}

#[derive(Debug)]
enum Item {
    Words(Vec<String>),
    Flag(bool),
    Count(Option<usize>),
    Text(Option<String>),
    Tiers(Vec<Vec<String>>),
}

/// An operator as the language spells it: the operation, its tier and its associativity.
#[derive(Clone, Copy, Debug)]
pub struct Operator {
    pub op: Op,
    pub tier: u32,
    pub right: bool,
}

pub struct Spec {
    pub name: String,
    pub style: Style,
    items: HashMap<&'static str, Item>,
    /// `$library`: the label a library function provides, by function name.
    pub library: HashMap<String, String>,
    pub infix: HashMap<String, Operator>,
    pub prefix: HashMap<String, Operator>,
    /// The tiers of syntax that sits among the operators: the range and the pipe.
    pub syntax_tier: HashMap<String, u32>,
    pub natives: HashMap<String, Native>,
    pub reserved: HashSet<String>,
    /// Symbol lexemes, longest first.
    pub symbols: Vec<String>,
}

fn object(text: &str) -> Result<Map<String, Json>, String> {
    match serde_json::from_str(text).map_err(|e| format!("definition is not valid JSON: {e}"))? {
        Json::Object(map) => Ok(map),
        _ => Err("definition must be a JSON object".to_string()),
    }
}

/// Name and extensions, read without the rest.
pub fn describe(text: &str) -> Result<(String, Vec<String>), String> {
    let map = object(text)?;
    let name = match map.get("language") {
        Some(Json::String(s)) if !s.is_empty() => s.clone(),
        _ => return Err("label 'language' must be a non-empty string".to_string()),
    };
    let exts = match map.get("extensions") {
        Some(Json::Array(items)) => items.iter().filter_map(|i| i.as_str().map(str::to_string)).collect(),
        _ => return Err("label 'extensions' must be a list of strings".to_string()),
    };
    Ok((name, exts))
}

fn read_item(key: &str, shape: Shape, json: &Json) -> Result<Item, String> {
    let strings = |items: &Vec<Json>| -> Result<Vec<String>, String> {
        items
            .iter()
            .map(|i| match i {
                Json::String(s) if !s.is_empty() => Ok(s.clone()),
                _ => Err(format!("label '{key}' must be a list of non-empty strings")),
            })
            .collect()
    };
    Ok(match (shape, json) {
        (Words, Json::Array(items)) => Item::Words(strings(items)?),
        (Flag, Json::Bool(b)) => Item::Flag(*b),
        (Count, Json::Null) => Item::Count(None),
        (Count, Json::Number(n)) if n.as_u64().is_some() => Item::Count(n.as_u64().map(|n| n as usize)),
        (Text, Json::String(s)) if !s.is_empty() => Item::Text(Some(s.clone())),
        (TextOrNull, Json::Null) => Item::Text(None),
        (TextOrNull, Json::String(s)) if !s.is_empty() => Item::Text(Some(s.clone())),
        (Tiers, Json::Array(tiers)) => Item::Tiers(
            tiers
                .iter()
                .map(|t| match t {
                    Json::Array(items) => strings(items),
                    _ => Err(format!("label '{key}' must be a list of lists of strings")),
                })
                .collect::<Result<_, _>>()?,
        ),
        _ => {
            let want = match shape {
                Words => "a list of non-empty strings",
                Flag => "true or false",
                Count => "a non-negative integer or null",
                Text => "a non-empty string",
                TextOrNull => "a non-empty string or null",
                Tiers => "a list of lists of strings",
            };
            return Err(format!("label '{key}' must be {want}"));
        }
    })
}

impl Spec {
    pub fn read(text: &str) -> Result<Spec, String> {
        let map = object(text)?;
        let mut items = HashMap::new();
        for (key, shape) in SCHEMA {
            let json = map.get(*key).ok_or_else(|| format!("missing label '{key}'"))?;
            items.insert(*key, read_item(key, *shape, json)?);
        }
        let mut unknown: Vec<&str> =
            map.keys().map(String::as_str).filter(|k| !k.starts_with('$') && SCHEMA.iter().all(|(l, _)| l != k)).collect();
        unknown.sort();
        if !unknown.is_empty() {
            return Err(format!("unknown label(s): {}", unknown.join(", ")));
        }
        let mut library = HashMap::new();
        if let Some(Json::Object(pairs)) = map.get("$library") {
            for (label, function) in pairs {
                if let Json::String(f) = function {
                    library.insert(f.clone(), label.clone());
                }
            }
        }
        let mut spec = Spec {
            name: String::new(),
            style: Style::Indent,
            items,
            library,
            infix: HashMap::new(),
            prefix: HashMap::new(),
            syntax_tier: HashMap::new(),
            natives: HashMap::new(),
            reserved: HashSet::new(),
            symbols: Vec::new(),
        };
        spec.name = spec.text("language").unwrap_or_default().to_string();
        spec.check()?;
        spec.derive()?;
        Ok(spec)
    }

    // ---- access

    pub fn words(&self, label: &str) -> &[String] {
        match self.items.get(label) {
            Some(Item::Words(w)) => w,
            _ => panic!("label '{label}' is not a word list"),
        }
    }

    pub fn first(&self, label: &str) -> Option<&str> {
        self.words(label).first().map(String::as_str)
    }

    pub fn has(&self, label: &str) -> bool {
        !self.words(label).is_empty()
    }

    pub fn spells(&self, label: &str, lexeme: &str) -> bool {
        self.words(label).iter().any(|w| w == lexeme)
    }

    pub fn flag(&self, label: &str) -> bool {
        matches!(self.items.get(label), Some(Item::Flag(true)))
    }

    pub fn count(&self, label: &str) -> Option<usize> {
        match self.items.get(label) {
            Some(Item::Count(c)) => *c,
            _ => None,
        }
    }

    pub fn text(&self, label: &str) -> Option<&str> {
        match self.items.get(label) {
            Some(Item::Text(t)) => t.as_deref(),
            _ => None,
        }
    }

    pub fn tiers(&self) -> &[Vec<String>] {
        match self.items.get("op.precedence") {
            Some(Item::Tiers(t)) => t,
            _ => &[],
        }
    }

    pub fn glyph(&self, label: &str) -> Option<char> {
        let w = self.first(label)?;
        let mut it = w.chars();
        match (it.next(), it.next()) {
            (Some(c), None) => Some(c),
            _ => None,
        }
    }

    pub fn glyphs(&self, label: &str) -> Vec<char> {
        self.words(label).iter().filter_map(|w| {
            let mut it = w.chars();
            match (it.next(), it.next()) {
                (Some(c), None) => Some(c),
                _ => None,
            }
        }).collect()
    }

    pub fn error_prefix(&self) -> String {
        let mut chars = self.name.chars();
        match chars.next() {
            Some(c) => format!("{}{}Error", c.to_uppercase(), chars.as_str()),
            None => "Error".to_string(),
        }
    }

    pub fn word_start(&self, c: char) -> bool {
        c == '_' || if self.flag("identifier.unicode") { c.is_alphabetic() } else { c.is_ascii_alphabetic() }
    }

    pub fn word_char(&self, c: char) -> bool {
        c == '_' || if self.flag("identifier.unicode") { c.is_alphanumeric() } else { c.is_ascii_alphanumeric() }
    }

    pub fn word_shaped(&self, s: &str) -> bool {
        let mut chars = s.chars();
        let mut first = chars.next();
        if first.is_some() && first == self.glyph("identifier.variable_prefix") {
            first = chars.next();
        }
        first.map_or(false, |c| self.word_start(c)) && chars.all(|c| self.word_char(c))
    }

    // ---- checks

    fn check(&mut self) -> Result<(), String> {
        if self.count("format_version") != Some(1) {
            return Err("format_version must be 1".to_string());
        }
        for label in MUST_BE_EMPTY {
            if self.has(label) {
                return Err(format!("label '{label}' is not implemented by the microcode2 kernel; leave it empty"));
            }
        }
        for label in ["lexical.string_quotes", "lexical.raw_quotes", "lexical.string_escapes", "lexical.name_quote",
                      "lexical.number.decimal_point", "lexical.number.base_marker", "lexical.number.exponent_marker",
                      "identifier.variable_prefix"] {
            if let Some(w) = self.words(label).iter().find(|w| w.chars().count() != 1) {
                return Err(format!("label '{label}' takes single characters, got '{w}'"));
            }
        }
        if self.words("lexical.comment_block.open").len() != self.words("lexical.comment_block.close").len() {
            return Err("lexical.comment_block.open and .close must pair up position by position".to_string());
        }
        if let Some(q) = self.words("lexical.raw_quotes").iter().find(|q| !self.spells("lexical.string_quotes", q)) {
            return Err(format!("lexical.raw_quotes lists '{q}', which is not in lexical.string_quotes"));
        }
        if let Some(p) = self.first("lexical.number.hex_prefix") {
            if p.chars().count() != 2 || !p.starts_with(|c: char| c.is_ascii_digit()) {
                return Err(format!("lexical.number.hex_prefix must be a digit followed by one letter, got '{p}'"));
            }
        }
        self.style = match self.text("block.style") {
            Some("indentation") => Style::Indent,
            Some("braces") => Style::Brace,
            Some("keyword") => Style::Keyword,
            Some("postfix") => Style::Postfix,
            other => return Err(format!("block.style must be 'indentation', 'braces', 'keyword' or 'postfix', got '{}'", other.unwrap_or(""))),
        };
        let (opens, closes) = (self.words("block.open").len(), self.words("block.close").len());
        let shaped = match self.style {
            Style::Indent => opens == 0 && closes == 0 && self.count("block.indent_size").map_or(false, |n| n > 0),
            Style::Brace => opens > 0 && opens == closes,
            Style::Keyword | Style::Postfix => opens == 0 && closes > 0,
        };
        if !shaped {
            return Err("block.open, block.close and block.indent_size do not fit block.style".to_string());
        }
        let postfix = self.style == Style::Postfix;
        if postfix && !self.tiers().is_empty() {
            return Err("a postfix language takes no op.precedence".to_string());
        }
        if !postfix && ["stack.dup", "stack.drop", "stack.swap", "stack.over", "stack.rot", "stack.eval",
                        "stack.program.open", "stack.program.close"].iter().any(|l| self.has(l)) {
            return Err("the stack.* labels need block.style 'postfix'".to_string());
        }
        if self.words("stack.program.open").len() != self.words("stack.program.close").len() {
            return Err("stack.program.open and .close must pair up position by position".to_string());
        }
        for (open, close) in [("syntax.group.open", "syntax.group.close"), ("syntax.call.open", "syntax.call.close"),
                              ("syntax.array.open", "syntax.array.close"), ("op.index.open", "op.index.close")] {
            if self.has(open) != self.has(close) {
                return Err(format!("labels '{open}' and '{close}' must be given together"));
            }
        }
        if self.has("syntax.call.label") && !self.has("syntax.call.open") {
            return Err("syntax.call.label needs syntax.call.open".to_string());
        }
        if self.flag("op.index.strings") && !self.has("op.index.open") {
            return Err("op.index.strings needs op.index.open".to_string());
        }
        if !matches!(self.text("op.div.result"), None | Some("rational") | Some("real")) {
            return Err("op.div.result must be 'rational', 'real' or null".to_string());
        }
        if !self.has("stmt.let") && (self.has("stmt.let.mutable") || self.has("stmt.let.annotation") || self.flag("stmt.let.type_first")) {
            return Err("stmt.let.mutable, stmt.let.annotation and stmt.let.type_first need stmt.let".to_string());
        }
        if self.flag("stmt.let.type_first") {
            for label in ["stmt.let.mutable", "stmt.let.annotation", "stmt.function", "stmt.function.returns"] {
                if self.has(label) {
                    return Err(format!("stmt.let.type_first leaves no place for {label}; leave it empty"));
                }
            }
        }
        if !self.has("stmt.if") && (self.has("stmt.elif") || self.has("stmt.else")) {
            return Err("stmt.elif and stmt.else need stmt.if".to_string());
        }
        if self.has("stmt.for") != self.has("stmt.for.in") && !(postfix && !self.has("stmt.for.in")) {
            return Err("stmt.for and stmt.for.in must be given together".to_string());
        }
        if self.flag("stmt.function.result_by_name") && !self.has("stmt.function") {
            return Err("stmt.function.result_by_name needs stmt.function".to_string());
        }
        let keyword_labels = [
            "stmt.let", "stmt.let.mutable", "stmt.if", "stmt.elif", "stmt.else", "stmt.while", "stmt.until", "stmt.for",
            "stmt.for.in", "stmt.return", "stmt.break", "stmt.continue", "stmt.function", "stmt.pass",
            "literal.true", "literal.false", "literal.null",
        ];
        for label in keyword_labels {
            if let Some(w) = self.words(label).iter().find(|w| !self.word_shaped(w)) {
                return Err(format!("keyword '{w}' must be shaped like an identifier"));
            }
        }
        let system = ["system.args", "system.memoization", "system.real_default_precision", "system.entry",
                      "system.kind.integer", "system.kind.rational", "system.kind.real", "system.kind.string",
                      "system.kind.boolean", "system.kind.array", "system.kind.null"];
        for label in system {
            if let Some(w) = self.words(label).first().filter(|w| !self.word_shaped(w)) {
                return Err(format!("system name '{w}' must be shaped like an identifier"));
            }
        }
        Ok(())
    }

    // ---- derived tables

    fn derive(&mut self) -> Result<(), String> {
        let postfix = self.style == Style::Postfix;
        let tiers = self.tiers().to_vec();
        let tier_of = |lex: &str, last: bool| -> Option<u32> {
            if postfix {
                return Some(1);
            }
            let found = if last {
                tiers.iter().rposition(|t| t.iter().any(|x| x == lex))
            } else {
                tiers.iter().position(|t| t.iter().any(|x| x == lex))
            };
            found.map(|i| i as u32 + 1)
        };
        let real_div = self.text("op.div.result") == Some("real");
        for (label, op) in BINARY {
            let op = if *label == "op.div" && real_div { Op::DivReal } else { *op };
            for lex in self.words(label).to_vec() {
                let tier = tier_of(&lex, false).ok_or_else(|| format!("'{lex}' ({label}) does not appear in op.precedence"))?;
                let right = self.spells("op.right_associative", &lex);
                if self.infix.insert(lex.clone(), Operator { op, tier, right }).is_some() {
                    return Err(format!("'{lex}' is listed under two binary operator labels"));
                }
            }
        }
        for (label, op) in [("op.not", Op::Not), ("op.negate", Op::Neg)] {
            for lex in self.words(label).to_vec() {
                let tier = tier_of(&lex, true).ok_or_else(|| format!("'{lex}' ({label}) does not appear in op.precedence"))?;
                if self.prefix.insert(lex.clone(), Operator { op, tier, right: false }).is_some() {
                    return Err(format!("'{lex}' is listed under two unary operator labels"));
                }
            }
        }
        for label in ["op.range", "op.pipe"] {
            for lex in self.words(label).to_vec() {
                let tier = tier_of(&lex, false).ok_or_else(|| format!("'{lex}' ({label}) does not appear in op.precedence"))?;
                self.syntax_tier.insert(lex, tier);
            }
        }
        for lex in tiers.iter().flatten() {
            if !self.infix.contains_key(lex) && !self.prefix.contains_key(lex) && !self.syntax_tier.contains_key(lex) {
                return Err(format!("op.precedence lists '{lex}', which is under no operator label"));
            }
        }
        for lex in self.words("op.right_associative") {
            if !self.infix.contains_key(lex) {
                return Err(format!("op.right_associative lists '{lex}', which is not a binary operator"));
            }
        }
        let quotes = self.glyphs("lexical.string_quotes");
        for (label, native) in NATIVES {
            for lex in self.words(label).to_vec() {
                let ok = lex.starts_with(|c: char| c == '_' || c.is_alphabetic())
                    && !lex.chars().any(|c| c.is_whitespace() || quotes.contains(&c));
                if !ok {
                    return Err(format!("builtin name '{lex}' must begin like an identifier and hold no spaces or quotes"));
                }
                if self.natives.insert(lex.clone(), *native).is_some() {
                    return Err(format!("'{lex}' is listed under two builtin labels"));
                }
            }
        }
        // Symbols the scanner segments on, and words it reserves.
        let mut lexemes: Vec<String> = Vec::new();
        lexemes.extend(self.infix.keys().cloned());
        lexemes.extend(self.prefix.keys().cloned());
        lexemes.extend(self.syntax_tier.keys().cloned());
        let listed = [
            "syntax.group.open", "syntax.group.close", "syntax.call.open", "syntax.call.separator", "syntax.call.close",
            "syntax.call.label", "syntax.array.open", "syntax.array.separator", "syntax.array.close",
            "op.index.open", "op.index.close", "block.intro", "stmt.assign", "stmt.terminator",
            "stmt.let.annotation", "stmt.function.returns", "stack.dup", "stack.drop", "stack.swap", "stack.over",
            "stack.rot", "stack.eval", "stack.program.open", "stack.program.close",
            "stmt.let", "stmt.let.mutable", "stmt.if", "stmt.elif", "stmt.else", "stmt.while", "stmt.until",
            "stmt.for", "stmt.for.in", "stmt.return", "stmt.break", "stmt.continue", "stmt.function", "stmt.pass",
            "literal.true", "literal.false", "literal.null",
        ];
        for label in listed {
            lexemes.extend(self.words(label).iter().cloned());
        }
        if self.style != Style::Indent {
            lexemes.extend(self.words("block.open").iter().cloned());
            lexemes.extend(self.words("block.close").iter().cloned());
        }
        let fold = self.flag("lexical.keywords_case_insensitive");
        for lex in lexemes {
            if self.word_shaped(&lex) {
                self.reserved.insert(if fold { lex.to_lowercase() } else { lex });
            } else if !self.symbols.contains(&lex) {
                self.symbols.push(lex);
            }
        }
        self.symbols.sort_by(|a, b| b.len().cmp(&a.len()).then(a.cmp(b)));
        Ok(())
    }
}
