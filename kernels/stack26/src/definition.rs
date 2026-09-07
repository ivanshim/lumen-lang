// A language definition, read from JSON into the tables the compiler
// consults. The reader is strict: every label present, no label unknown,
// every value of the right shape. Keys beginning with `$` are notes for
// readers and are skipped.

use std::collections::{HashMap, HashSet};

use serde_json::Value as Json;

use crate::code::{Builtin, Op};
use crate::value::Kind;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Layout {
    Indented,
    Bracketed,
    Closed,
}

#[derive(Clone, Debug)]
pub struct Brackets {
    pub open: String,
    pub close: String,
    pub between: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Operator {
    pub op: Op,
    pub tier: u32,
    pub rightward: bool,
}

pub struct Language {
    pub name: String,
    pub extensions: Vec<String>,
    pub error_prefix: String,

    // ---- text
    pub line_comments: Vec<String>,
    pub block_comments: Vec<(String, String)>,
    pub quotes: Vec<char>,
    pub raw_quotes: Vec<char>,
    pub escape_letters: Vec<char>,
    pub prologue: Option<String>,
    pub point: Option<char>,
    pub base_mark: Option<char>,
    pub exponent_mark: Option<char>,
    pub hex_prefix: Option<String>,
    pub unicode_words: bool,
    pub variable_prefix: Option<char>,
    pub fold_keywords: bool,
    pub fold_names: bool,
    pub name_quote: Option<char>,
    /// Symbol lexemes, longest first.
    pub symbols: Vec<String>,
    pub reserved: HashSet<String>,

    // ---- layout
    pub layout: Layout,
    /// Reverse Polish: words over one stack, no expressions or statements.
    pub postfix: bool,
    pub indent: usize,
    pub openers: Vec<String>,
    pub closers: Vec<String>,
    pub intros: Vec<String>,
    pub terminators: Vec<String>,
    pub group: Option<Brackets>,
    pub call: Option<Brackets>,
    pub call_labels: Vec<String>,
    pub array: Option<Brackets>,
    pub index: Option<Brackets>,
    pub index_text: bool,

    // ---- words
    pub yes_words: Vec<String>,
    pub no_words: Vec<String>,
    pub null_words: Vec<String>,
    pub binary: HashMap<String, Operator>,
    pub unary: HashMap<String, Operator>,
    pub pipes: Vec<String>,
    pub ranges: Vec<String>,
    /// The tiers of the range and pipe lexemes, which are syntax rather than operations.
    pub special_tiers: HashMap<String, u32>,

    // ---- statements
    pub assign: Vec<String>,
    pub let_words: Vec<String>,
    pub mutable_words: Vec<String>,
    pub annotation: Vec<String>,
    pub type_first: bool,
    pub if_words: Vec<String>,
    pub elif_words: Vec<String>,
    pub else_words: Vec<String>,
    pub while_words: Vec<String>,
    pub until_words: Vec<String>,
    pub for_words: Vec<String>,
    pub in_words: Vec<String>,
    pub return_words: Vec<String>,
    pub break_words: Vec<String>,
    pub continue_words: Vec<String>,
    pub function_words: Vec<String>,
    pub returns_words: Vec<String>,
    pub result_by_name: bool,
    pub pass_words: Vec<String>,

    // ---- stack
    pub dup: Vec<String>,
    pub drop: Vec<String>,
    pub swap: Vec<String>,
    pub over: Vec<String>,
    pub rot: Vec<String>,
    pub eval: Vec<String>,
    pub program_open: Vec<String>,
    pub program_close: Vec<String>,

    // ---- builtins and system names
    pub builtins: HashMap<String, Builtin>,
    pub placeholders: Vec<String>,
    pub args_name: Option<String>,
    pub memo_name: Option<String>,
    pub precision_name: Option<String>,
    pub entry_name: Option<String>,
    pub kind_names: Vec<(String, Kind)>,
}

/// Labels every definition must carry, in the file's order.
const LABELS: &[&str] = &[
    "format_version", "language", "extensions",
    "lexical.comment_line", "lexical.comment_block.open", "lexical.comment_block.close",
    "lexical.string_quotes", "lexical.raw_quotes", "lexical.string_escapes", "lexical.prologue", "lexical.name_quote",
    "lexical.number.decimal_point", "lexical.number.base_marker", "lexical.number.exponent_marker",
    "lexical.number.hex_prefix", "lexical.keywords_case_insensitive",
    "identifier.unicode", "identifier.variable_prefix", "identifier.case_insensitive",
    "block.style", "block.open", "block.close", "block.intro", "block.indent_size",
    "stmt.terminator", "syntax.notation",
    "syntax.group.open", "syntax.group.close",
    "syntax.call.open", "syntax.call.separator", "syntax.call.close", "syntax.call.label",
    "syntax.array.open", "syntax.array.separator", "syntax.array.close",
    "syntax.map.open", "syntax.map.separator", "syntax.map.pair", "syntax.map.close",
    "literal.true", "literal.false", "literal.null",
    "op.precedence", "op.right_associative",
    "op.add", "op.sub", "op.mul", "op.div", "op.div.result", "op.quot", "op.rem", "op.pow",
    "op.eq", "op.ne", "op.lt", "op.le", "op.gt", "op.ge",
    "op.and", "op.or", "op.not", "op.negate", "op.concat", "op.range",
    "op.index.open", "op.index.close", "op.index.strings", "op.pipe",
    "stmt.assign", "stmt.let", "stmt.let.mutable", "stmt.let.annotation", "stmt.let.type_first",
    "stmt.if", "stmt.elif", "stmt.else", "stmt.while", "stmt.until", "stmt.for", "stmt.for.in",
    "stmt.foreach", "stmt.foreach.as", "stmt.foreach.pair",
    "stmt.return", "stmt.break", "stmt.continue", "stmt.function", "stmt.function.returns",
    "stmt.function.result_by_name", "stmt.pass", "stmt.emit",
    "stack.dup", "stack.drop", "stack.swap", "stack.over", "stack.rot", "stack.eval",
    "stack.program.open", "stack.program.close",
    "builtin.emit", "builtin.print", "builtin.write", "builtin.print.placeholder",
    "builtin.len", "builtin.char_at", "builtin.ord", "builtin.chr", "builtin.typeof", "builtin.error",
    "builtin.extern", "builtin.range", "builtin.real", "builtin.num", "builtin.den", "builtin.push",
    "builtin.get", "builtin.put", "builtin.precision", "builtin.to_string", "builtin.to_int", "builtin.to_real",
    "system.args", "system.memoization", "system.real_default_precision", "system.entry",
    "system.kind.integer", "system.kind.rational", "system.kind.real", "system.kind.string",
    "system.kind.boolean", "system.kind.array", "system.kind.null",
];

/// Labels this kernel knows of but gives no meaning; they must be empty.
const UNUSED: &[&str] = &[
    "syntax.map.open", "syntax.map.separator", "syntax.map.pair", "syntax.map.close",
    "stmt.foreach", "stmt.foreach.as", "stmt.foreach.pair", "stmt.emit",
];

struct Table<'a>(&'a serde_json::Map<String, Json>);

impl<'a> Table<'a> {
    fn json(&self, key: &str) -> Result<&'a Json, String> {
        self.0.get(key).ok_or_else(|| format!("missing label '{key}'"))
    }

    fn words(&self, key: &str) -> Result<Vec<String>, String> {
        let bad = || format!("label '{key}' must be a list of non-empty strings");
        let Json::Array(items) = self.json(key)? else { return Err(bad()) };
        let mut out = Vec::with_capacity(items.len());
        for item in items {
            match item {
                Json::String(s) if !s.is_empty() => out.push(s.clone()),
                _ => return Err(bad()),
            }
        }
        Ok(out)
    }

    fn one(&self, key: &str) -> Result<Option<String>, String> {
        Ok(self.words(key)?.into_iter().next())
    }

    fn glyphs(&self, key: &str) -> Result<Vec<char>, String> {
        let mut out = Vec::new();
        for word in self.words(key)? {
            let mut it = word.chars();
            match (it.next(), it.next()) {
                (Some(c), None) => out.push(c),
                _ => return Err(format!("label '{key}' takes single characters, got '{word}'")),
            }
        }
        Ok(out)
    }

    fn glyph(&self, key: &str) -> Result<Option<char>, String> {
        Ok(self.glyphs(key)?.into_iter().next())
    }

    fn switch(&self, key: &str) -> Result<bool, String> {
        match self.json(key)? {
            Json::Bool(b) => Ok(*b),
            _ => Err(format!("label '{key}' must be true or false")),
        }
    }

    fn string(&self, key: &str) -> Result<String, String> {
        match self.json(key)? {
            Json::String(s) if !s.is_empty() => Ok(s.clone()),
            _ => Err(format!("label '{key}' must be a non-empty string")),
        }
    }

    fn string_or_null(&self, key: &str) -> Result<Option<String>, String> {
        match self.json(key)? {
            Json::Null => Ok(None),
            Json::String(s) if !s.is_empty() => Ok(Some(s.clone())),
            _ => Err(format!("label '{key}' must be a non-empty string or null")),
        }
    }

    fn size(&self, key: &str) -> Result<Option<usize>, String> {
        match self.json(key)? {
            Json::Null => Ok(None),
            Json::Number(n) if n.as_u64().is_some() => Ok(n.as_u64().map(|n| n as usize)),
            _ => Err(format!("label '{key}' must be a non-negative integer or null")),
        }
    }

    fn tiers(&self, key: &str) -> Result<Vec<Vec<String>>, String> {
        let bad = || format!("label '{key}' must be a list of lists of strings");
        let Json::Array(tiers) = self.json(key)? else { return Err(bad()) };
        let mut out = Vec::new();
        for tier in tiers {
            let Json::Array(items) = tier else { return Err(bad()) };
            let mut words = Vec::new();
            for item in items {
                match item {
                    Json::String(s) if !s.is_empty() => words.push(s.clone()),
                    _ => return Err(bad()),
                }
            }
            out.push(words);
        }
        Ok(out)
    }

    fn brackets(&self, open: &str, close: &str, between: Option<&str>) -> Result<Option<Brackets>, String> {
        let o = self.one(open)?;
        let c = self.one(close)?;
        let b = match between {
            Some(key) => self.one(key)?,
            None => None,
        };
        match (o, c) {
            (Some(open), Some(close)) => Ok(Some(Brackets { open, close, between: b })),
            (None, None) if b.is_none() => Ok(None),
            _ => Err(format!("labels '{open}' and '{close}' must be given together")),
        }
    }
}

fn object(text: &str) -> Result<serde_json::Map<String, Json>, String> {
    let root: Json = serde_json::from_str(text).map_err(|e| format!("definition is not valid JSON: {e}"))?;
    match root {
        Json::Object(map) => Ok(map),
        _ => Err("definition must be a JSON object".to_string()),
    }
}

/// The name and extensions a definition declares, without reading the rest.
pub fn describe(text: &str) -> Result<(String, Vec<String>), String> {
    let map = object(text)?;
    let t = Table(&map);
    Ok((t.string("language")?, t.words("extensions")?))
}

fn word_shaped(s: &str, unicode: bool, prefix: Option<char>) -> bool {
    let starts = |c: char| c == '_' || if unicode { c.is_alphabetic() } else { c.is_ascii_alphabetic() };
    let goes_on = |c: char| c == '_' || if unicode { c.is_alphanumeric() } else { c.is_ascii_alphanumeric() };
    let mut chars = s.chars();
    let mut first = match chars.next() {
        Some(c) => c,
        None => return false,
    };
    if Some(first) == prefix {
        first = match chars.next() {
            Some(c) => c,
            None => return false,
        };
    }
    starts(first) && chars.all(goes_on)
}

impl Language {
    pub fn read(text: &str) -> Result<Language, String> {
        let map = object(text)?;
        for key in LABELS {
            if !map.contains_key(*key) {
                return Err(format!("missing label '{key}'"));
            }
        }
        let mut unknown: Vec<&String> =
            map.keys().filter(|k| !k.starts_with('$') && !LABELS.contains(&k.as_str())).collect();
        unknown.sort();
        if !unknown.is_empty() {
            let names: Vec<&str> = unknown.iter().map(|s| s.as_str()).collect();
            return Err(format!("unknown label(s): {}", names.join(", ")));
        }
        let t = Table(&map);
        if t.size("format_version")? != Some(1) {
            return Err("format_version must be 1".to_string());
        }
        for key in UNUSED {
            if !t.words(key)?.is_empty() {
                return Err(format!("label '{key}' is not implemented by the stack kernel; leave it empty"));
            }
        }

        let name = t.string("language")?;
        let unicode_words = t.switch("identifier.unicode")?;
        let variable_prefix = t.glyph("identifier.variable_prefix")?;
        let shaped = |s: &str| word_shaped(s, unicode_words, variable_prefix);

        // ---- text
        let opens = t.words("lexical.comment_block.open")?;
        let closes = t.words("lexical.comment_block.close")?;
        if opens.len() != closes.len() {
            return Err("lexical.comment_block.open and .close must pair up position by position".to_string());
        }
        let quotes = t.glyphs("lexical.string_quotes")?;
        let raw_quotes = t.glyphs("lexical.raw_quotes")?;
        if let Some(q) = raw_quotes.iter().find(|q| !quotes.contains(q)) {
            return Err(format!("lexical.raw_quotes lists '{q}', which is not in lexical.string_quotes"));
        }
        let escape_letters = t.glyphs("lexical.string_escapes")?;
        for letter in &escape_letters {
            if !matches!(letter, 'n' | 't' | 'r' | '0' | '\\') && !quotes.contains(letter) {
                return Err(format!("lexical.string_escapes: unknown escape letter '{letter}'"));
            }
        }
        let hex_prefix = t.one("lexical.number.hex_prefix")?;
        if let Some(p) = &hex_prefix {
            if p.chars().count() != 2 || !p.starts_with(|c: char| c.is_ascii_digit()) {
                return Err(format!("lexical.number.hex_prefix must be a digit followed by one letter, got '{p}'"));
            }
        }

        // ---- layout
        let layout = match t.string("block.style")?.as_str() {
            "indentation" => Layout::Indented,
            "braces" => Layout::Bracketed,
            "keyword" => Layout::Closed,
            other => return Err(format!("block.style must be 'indentation', 'braces' or 'keyword', got '{other}'")),
        };
        let postfix = match t.string("syntax.notation")?.as_str() {
            "infix" => false,
            "postfix" => true,
            other => return Err(format!("syntax.notation must be 'infix' or 'postfix', got '{other}'")),
        };
        let openers = t.words("block.open")?;
        let closers = t.words("block.close")?;
        let intros = t.words("block.intro")?;
        let indent_size = t.size("block.indent_size")?;
        let indent = match layout {
            Layout::Indented => {
                if !openers.is_empty() || !closers.is_empty() {
                    return Err("indentation blocks take no block.open or block.close".to_string());
                }
                match indent_size {
                    Some(0) => return Err("block.indent_size must be at least 1".to_string()),
                    Some(n) => n,
                    None => return Err("indentation blocks need block.indent_size".to_string()),
                }
            }
            Layout::Bracketed => {
                if openers.is_empty() || openers.len() != closers.len() {
                    return Err("braces need block.open and block.close, paired position by position".to_string());
                }
                indent_size.unwrap_or(4)
            }
            Layout::Closed => {
                if !openers.is_empty() || closers.is_empty() {
                    return Err("keyword blocks take no block.open and need block.close".to_string());
                }
                indent_size.unwrap_or(4)
            }
        };
        let call = t.brackets("syntax.call.open", "syntax.call.close", Some("syntax.call.separator"))?;
        let call_labels = t.words("syntax.call.label")?;
        if !call_labels.is_empty() && call.is_none() {
            return Err("syntax.call.label needs syntax.call.open".to_string());
        }
        let index = t.brackets("op.index.open", "op.index.close", None)?;
        let index_text = t.switch("op.index.strings")?;
        if index_text && index.is_none() {
            return Err("op.index.strings needs op.index.open".to_string());
        }

        // ---- operators
        let tiers = t.tiers("op.precedence")?;
        if postfix && !tiers.is_empty() {
            return Err("a postfix language takes no op.precedence".to_string());
        }
        let rightward = t.words("op.right_associative")?;
        let div = match t.string_or_null("op.div.result")?.as_deref() {
            None | Some("rational") => Op::Div,
            Some("real") => Op::RealDiv,
            Some(other) => return Err(format!("op.div.result must be 'rational', 'real' or null, got '{other}'")),
        };
        let first_tier = |lex: &str| {
            if postfix { Some(0) } else { tiers.iter().position(|tier| tier.iter().any(|x| x == lex)) }
        };
        let last_tier = |lex: &str| {
            if postfix { Some(0) } else { tiers.iter().rposition(|tier| tier.iter().any(|x| x == lex)) }
        };
        let mut binary = HashMap::new();
        let binary_labels = [
            ("op.add", Op::Add), ("op.sub", Op::Sub), ("op.mul", Op::Mul), ("op.div", div),
            ("op.quot", Op::Quot), ("op.rem", Op::Rem), ("op.pow", Op::Pow),
            ("op.eq", Op::Eq), ("op.ne", Op::Ne), ("op.lt", Op::Lt), ("op.le", Op::Le),
            ("op.gt", Op::Gt), ("op.ge", Op::Ge), ("op.and", Op::And), ("op.or", Op::Or),
            ("op.concat", Op::Concat),
        ];
        for (label, op) in binary_labels {
            for lex in t.words(label)? {
                let tier = first_tier(&lex).ok_or_else(|| format!("'{lex}' ({label}) does not appear in op.precedence"))?;
                let operator = Operator { op, tier: tier as u32 + 1, rightward: rightward.contains(&lex) };
                if binary.insert(lex.clone(), operator).is_some() {
                    return Err(format!("'{lex}' is listed under two binary operator labels"));
                }
            }
        }
        let mut unary = HashMap::new();
        for (label, op) in [("op.not", Op::Not), ("op.negate", Op::Neg)] {
            for lex in t.words(label)? {
                let tier = last_tier(&lex).ok_or_else(|| format!("'{lex}' ({label}) does not appear in op.precedence"))?;
                let operator = Operator { op, tier: tier as u32 + 1, rightward: false };
                if unary.insert(lex.clone(), operator).is_some() {
                    return Err(format!("'{lex}' is listed under two unary operator labels"));
                }
            }
        }
        // The range and the pipe are syntax, not operations, but they sit in the tiers.
        let ranges = t.words("op.range")?;
        let pipes = t.words("op.pipe")?;
        let mut special = HashMap::new();
        for (label, list) in [("op.range", &ranges), ("op.pipe", &pipes)] {
            for lex in list {
                let tier = first_tier(lex).ok_or_else(|| format!("'{lex}' ({label}) does not appear in op.precedence"))?;
                special.insert(lex.clone(), tier as u32 + 1);
            }
        }
        for lex in tiers.iter().flatten() {
            if !binary.contains_key(lex) && !unary.contains_key(lex) && !special.contains_key(lex) {
                return Err(format!("op.precedence lists '{lex}', which is under no operator label"));
            }
        }
        for lex in &rightward {
            if !binary.contains_key(lex) {
                return Err(format!("op.right_associative lists '{lex}', which is not a binary operator"));
            }
        }

        // ---- statements
        let let_words = t.words("stmt.let")?;
        let mutable_words = t.words("stmt.let.mutable")?;
        let annotation = t.words("stmt.let.annotation")?;
        let type_first = t.switch("stmt.let.type_first")?;
        if let_words.is_empty() && (!mutable_words.is_empty() || !annotation.is_empty() || type_first) {
            return Err("stmt.let.mutable, stmt.let.annotation and stmt.let.type_first need stmt.let".to_string());
        }
        let if_words = t.words("stmt.if")?;
        let elif_words = t.words("stmt.elif")?;
        let else_words = t.words("stmt.else")?;
        if if_words.is_empty() && (!elif_words.is_empty() || !else_words.is_empty()) {
            return Err("stmt.elif and stmt.else need stmt.if".to_string());
        }
        let for_words = t.words("stmt.for")?;
        let in_words = t.words("stmt.for.in")?;
        if for_words.is_empty() != in_words.is_empty() && !(postfix && in_words.is_empty()) {
            return Err("stmt.for and stmt.for.in must be given together".to_string());
        }
        let function_words = t.words("stmt.function")?;
        let returns_words = t.words("stmt.function.returns")?;
        let result_by_name = t.switch("stmt.function.result_by_name")?;
        if result_by_name && function_words.is_empty() {
            return Err("stmt.function.result_by_name needs stmt.function".to_string());
        }
        if type_first {
            for (label, list) in [
                ("stmt.let.mutable", &mutable_words),
                ("stmt.let.annotation", &annotation),
                ("stmt.function", &function_words),
                ("stmt.function.returns", &returns_words),
            ] {
                if !list.is_empty() {
                    return Err(format!("stmt.let.type_first leaves no place for {label}; leave it empty"));
                }
            }
        }

        // ---- stack
        let stack_labels = [
            "stack.dup", "stack.drop", "stack.swap", "stack.over", "stack.rot", "stack.eval",
            "stack.program.open", "stack.program.close",
        ];
        let mut stack_words: Vec<Vec<String>> = Vec::new();
        for label in stack_labels {
            stack_words.push(t.words(label)?);
        }
        if !postfix && stack_words.iter().any(|list| !list.is_empty()) {
            return Err("the stack.* labels need syntax.notation 'postfix'".to_string());
        }
        if stack_words[6].len() != stack_words[7].len() {
            return Err("stack.program.open and .close must pair up position by position".to_string());
        }

        // ---- builtins
        let builtin_labels = [
            ("builtin.emit", Builtin::Emit), ("builtin.print", Builtin::Print), ("builtin.write", Builtin::Write),
            ("builtin.len", Builtin::Len), ("builtin.char_at", Builtin::CharAt), ("builtin.ord", Builtin::Ord),
            ("builtin.chr", Builtin::Chr), ("builtin.typeof", Builtin::Kind), ("builtin.error", Builtin::Error),
            ("builtin.extern", Builtin::Extern), ("builtin.range", Builtin::Range), ("builtin.real", Builtin::Real),
            ("builtin.precision", Builtin::Precision), ("builtin.to_string", Builtin::ToText),
            ("builtin.to_int", Builtin::ToInt), ("builtin.to_real", Builtin::ToReal), ("builtin.num", Builtin::Num),
            ("builtin.den", Builtin::Den), ("builtin.push", Builtin::Push), ("builtin.get", Builtin::Get),
            ("builtin.put", Builtin::Put),
        ];
        let mut builtins = HashMap::new();
        for (label, builtin) in builtin_labels {
            for lex in t.words(label)? {
                let mut chars = lex.chars();
                let begins = chars.next().map_or(false, |c| c == '_' || c.is_alphabetic());
                if !begins || lex.chars().any(|c| c.is_whitespace() || quotes.contains(&c)) {
                    return Err(format!("builtin name '{lex}' must begin like an identifier and hold no spaces or quotes"));
                }
                if builtins.insert(lex.clone(), builtin).is_some() {
                    return Err(format!("'{lex}' is listed under two builtin labels"));
                }
            }
        }

        // ---- system
        let mut kind_names = Vec::new();
        for (label, kind) in [
            ("system.kind.integer", Kind::Integer), ("system.kind.rational", Kind::Rational),
            ("system.kind.real", Kind::Real), ("system.kind.string", Kind::Text),
            ("system.kind.boolean", Kind::Boolean), ("system.kind.array", Kind::List),
            ("system.kind.null", Kind::Nothing),
        ] {
            if let Some(name) = t.one(label)? {
                kind_names.push((name, kind));
            }
        }
        let args_name = t.one("system.args")?;
        let memo_name = t.one("system.memoization")?;
        let precision_name = t.one("system.real_default_precision")?;
        let entry_name = t.one("system.entry")?;
        for n in [&args_name, &memo_name, &precision_name, &entry_name].into_iter().flatten() {
            if !shaped(n) {
                return Err(format!("system name '{n}' must be shaped like an identifier"));
            }
        }
        for (n, _) in &kind_names {
            if !shaped(n) {
                return Err(format!("system name '{n}' must be shaped like an identifier"));
            }
        }

        let mut language = Language {
            name: name.clone(),
            extensions: t.words("extensions")?,
            error_prefix: {
                let mut chars = name.chars();
                match chars.next() {
                    Some(c) => format!("{}{}Error", c.to_uppercase(), chars.as_str()),
                    None => "Error".to_string(),
                }
            },
            line_comments: t.words("lexical.comment_line")?,
            block_comments: opens.into_iter().zip(closes).collect(),
            quotes,
            raw_quotes,
            escape_letters,
            prologue: t.one("lexical.prologue")?,
            point: t.glyph("lexical.number.decimal_point")?,
            base_mark: t.glyph("lexical.number.base_marker")?,
            exponent_mark: t.glyph("lexical.number.exponent_marker")?,
            hex_prefix,
            unicode_words,
            variable_prefix,
            fold_keywords: t.switch("lexical.keywords_case_insensitive")?,
            fold_names: t.switch("identifier.case_insensitive")?,
            name_quote: t.glyph("lexical.name_quote")?,
            symbols: Vec::new(),
            reserved: HashSet::new(),
            layout,
            postfix,
            indent,
            openers,
            closers,
            intros,
            terminators: t.words("stmt.terminator")?,
            group: t.brackets("syntax.group.open", "syntax.group.close", None)?,
            call,
            call_labels,
            array: t.brackets("syntax.array.open", "syntax.array.close", Some("syntax.array.separator"))?,
            index,
            index_text,
            yes_words: t.words("literal.true")?,
            no_words: t.words("literal.false")?,
            null_words: t.words("literal.null")?,
            binary,
            unary,
            pipes,
            ranges,
            special_tiers: special.clone(),
            assign: t.words("stmt.assign")?,
            let_words,
            mutable_words,
            annotation,
            type_first,
            if_words,
            elif_words,
            else_words,
            while_words: t.words("stmt.while")?,
            until_words: t.words("stmt.until")?,
            for_words,
            in_words,
            return_words: t.words("stmt.return")?,
            break_words: t.words("stmt.break")?,
            continue_words: t.words("stmt.continue")?,
            function_words,
            returns_words,
            result_by_name,
            pass_words: t.words("stmt.pass")?,
            dup: stack_words[0].clone(),
            drop: stack_words[1].clone(),
            swap: stack_words[2].clone(),
            over: stack_words[3].clone(),
            rot: stack_words[4].clone(),
            eval: stack_words[5].clone(),
            program_open: stack_words[6].clone(),
            program_close: stack_words[7].clone(),
            builtins,
            placeholders: t.words("builtin.print.placeholder")?,
            args_name,
            memo_name,
            precision_name,
            entry_name,
            kind_names,
        };
        language.classify(&special)?;
        Ok(language)
    }

    /// Sort every lexeme into the symbols the scanner segments on and the
    /// words it reserves; a keyword must be shaped like a word.
    fn classify(&mut self, special: &HashMap<String, u32>) -> Result<(), String> {
        let mut symbols: Vec<String> = Vec::new();
        let mut reserved: HashSet<String> = HashSet::new();
        let unicode = self.unicode_words;
        let prefix = self.variable_prefix;
        let mut sort = |lex: &str| {
            if word_shaped(lex, unicode, prefix) {
                reserved.insert(lex.to_string());
            } else if !symbols.iter().any(|s| s == lex) {
                symbols.push(lex.to_string());
            }
        };
        for lex in self.binary.keys().chain(self.unary.keys()).chain(special.keys()) {
            sort(lex);
        }
        for b in [&self.group, &self.call, &self.array, &self.index].into_iter().flatten() {
            sort(&b.open);
            sort(&b.close);
            if let Some(sep) = &b.between {
                sort(sep);
            }
        }
        if self.layout != Layout::Indented {
            for lex in self.openers.iter().chain(self.closers.iter()) {
                sort(lex);
            }
        }
        let symbolic: Vec<&Vec<String>> = vec![
            &self.intros, &self.assign, &self.terminators, &self.call_labels, &self.annotation, &self.returns_words,
            &self.dup, &self.drop, &self.swap, &self.over, &self.rot, &self.eval, &self.program_open, &self.program_close,
        ];
        for list in symbolic {
            for lex in list {
                sort(lex);
            }
        }
        let keyword_lists: Vec<&Vec<String>> = vec![
            &self.let_words, &self.mutable_words, &self.if_words, &self.elif_words, &self.else_words,
            &self.while_words, &self.until_words, &self.for_words, &self.in_words, &self.return_words,
            &self.break_words, &self.continue_words, &self.function_words, &self.pass_words,
            &self.yes_words, &self.no_words, &self.null_words,
        ];
        for list in keyword_lists {
            for word in list {
                if !word_shaped(word, unicode, prefix) {
                    return Err(format!("keyword '{word}' must be shaped like an identifier"));
                }
                reserved.insert(word.clone());
            }
        }
        if self.fold_keywords {
            reserved = reserved.into_iter().map(|w| w.to_lowercase()).collect();
        }
        symbols.sort_by(|a, b| b.len().cmp(&a.len()));
        self.symbols = symbols;
        self.reserved = reserved;
        Ok(())
    }

    pub fn spells(list: &[String], word: &str) -> bool {
        list.iter().any(|w| w == word)
    }

    pub fn terminator(&self, lex: &str) -> bool {
        Self::spells(&self.terminators, lex)
    }

    pub fn word_starts(&self, c: char) -> bool {
        c == '_' || if self.unicode_words { c.is_alphabetic() } else { c.is_ascii_alphabetic() }
    }

    pub fn word_continues(&self, c: char) -> bool {
        c == '_' || if self.unicode_words { c.is_alphanumeric() } else { c.is_ascii_alphanumeric() }
    }
}
