// The language definition, read from a langs/*.json file.
//
// The stream kernel hosts a language in code: handlers give constructs their
// meaning. Which language, and how every construct is spelled, comes from the
// definition. One definition is installed per run; handlers reach it through
// `def()`. Labels are looked up by name, and asking for a label the
// definition lacks is a programming error that stops the interpreter at once.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde_json::Value as Json;

/// The definitions embedded at build time.
pub const EMBEDDED: &[&str] = &[
    include_str!("../../../../langs/lumen.json"),
    include_str!("../../../../langs/python.json"),
    include_str!("../../../../langs/rust.json"),
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BlockStyle {
    Indentation,
    Braces,
    /// No opener; the body runs to a closing word, and an `if` chain shares one.
    Keyword,
}

/// Every label a definition must carry. The reader rejects a file that
/// lacks one or carries a key outside this set, so a mistake in a
/// user-written definition is reported at load, not mid-parse.
const LABELS: &[&str] = &[
    "format_version", "language", "extensions",
    "lexical.comment_line", "lexical.comment_block.open", "lexical.comment_block.close",
    "lexical.string_quotes", "lexical.raw_quotes", "lexical.string_escapes", "lexical.prologue",
    "lexical.number.decimal_point", "lexical.number.base_marker", "lexical.number.exponent_marker",
    "lexical.number.hex_prefix", "lexical.keywords_case_insensitive",
    "identifier.unicode", "identifier.variable_prefix", "identifier.case_insensitive",
    "block.style", "block.open", "block.close", "block.intro", "block.indent_size", "stmt.terminator",
    "syntax.group.open", "syntax.group.close",
    "syntax.call.open", "syntax.call.separator", "syntax.call.close", "syntax.call.label",
    "syntax.array.open", "syntax.array.separator", "syntax.array.close",
    "syntax.map.open", "syntax.map.separator", "syntax.map.pair", "syntax.map.close",
    "literal.true", "literal.false", "literal.null",
    "op.precedence", "op.right_associative",
    "op.add", "op.sub", "op.mul", "op.div", "op.quot", "op.rem", "op.pow",
    "op.eq", "op.ne", "op.lt", "op.le", "op.gt", "op.ge",
    "op.and", "op.or", "op.not", "op.negate",
    "op.concat", "op.range", "op.index.open", "op.index.close", "op.pipe",
    "stmt.assign", "stmt.let", "stmt.let.mutable", "stmt.let.annotation", "stmt.let.type_first",
    "stmt.if", "stmt.elif", "stmt.else", "stmt.while", "stmt.until", "stmt.for", "stmt.for.in",
    "stmt.foreach", "stmt.foreach.as", "stmt.foreach.pair",
    "stmt.return", "stmt.break", "stmt.continue", "stmt.function", "stmt.function.returns",
    "stmt.pass", "stmt.emit",
    "builtin.emit", "builtin.print", "builtin.write", "builtin.print.placeholder", "builtin.len", "builtin.char_at", "builtin.ord",
    "builtin.chr", "builtin.typeof", "builtin.error", "builtin.extern", "builtin.range",
    "builtin.real", "builtin.int_to_string", "builtin.real_to_string", "builtin.rational_to_string",
    "builtin.bool_to_string", "builtin.array_to_string", "builtin.null_to_string", "builtin.kind_to_string",
    "builtin.num", "builtin.den", "builtin.int", "builtin.frac", "builtin.push",
    "system.args", "system.memoization", "system.real_default_precision", "system.entry",
    "system.kind.integer", "system.kind.rational", "system.kind.real", "system.kind.string",
    "system.kind.boolean", "system.kind.array", "system.kind.null",
];

/// Labels the stream kernel recognises but does not implement; a
/// definition must leave them empty.
const UNSUPPORTED: &[&str] = &[
    "syntax.map.open", "syntax.map.separator", "syntax.map.pair", "syntax.map.close",
    "stmt.foreach", "stmt.foreach.as", "stmt.foreach.pair", "stmt.emit",
];

pub struct Definition {
    pub name: String,
    pub extensions: Vec<String>,
    lexemes: HashMap<String, Vec<String>>,
    tiers: Vec<Vec<String>>,
    right_associative: Vec<String>,
    pub identifier_unicode: bool,
    pub identifiers_case_insensitive: bool,
    pub keywords_case_insensitive: bool,
    /// The binding words are type names placed first (C's `int x = 1;`),
    /// and a name followed by the call bracket defines a function.
    pub type_first: bool,
    pub block_style: BlockStyle,
    pub indent_size: usize,
    /// Every reserved word, computed once: the parser asks on every token.
    reserved: Vec<String>,
}

static CURRENT: OnceLock<Definition> = OnceLock::new();

/// Make `definition` the one this process hosts. A process hosts one
/// language; installing a second, different one is an error.
pub fn install(definition: Definition) -> Result<(), String> {
    let name = definition.name.clone();
    match CURRENT.set(definition) {
        Ok(()) => Ok(()),
        Err(_) if CURRENT.get().map_or(false, |d| d.name == name) => Ok(()),
        Err(_) => Err(format!("the stream kernel already hosts '{}' in this process", CURRENT.get().unwrap().name)),
    }
}

/// The installed definition.
pub fn def() -> &'static Definition {
    CURRENT.get().expect("a language definition is installed before any program is run")
}

/// The embedded definition text for a language name.
pub fn embedded(name: &str) -> Result<&'static str, String> {
    for text in EMBEDDED {
        if describe(text)?.0 == name {
            return Ok(text);
        }
    }
    Err(format!("Error: Unknown language '{}'", name))
}

/// The name and file extensions of a definition, read without installing it.
pub fn describe(text: &str) -> Result<(String, Vec<String>), String> {
    let root: Json = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let map = root.as_object().ok_or("the definition must be a JSON object")?;
    let name = map.get("language").and_then(Json::as_str).ok_or("missing label 'language'")?.to_string();
    let extensions = map.get("extensions").map(strings).transpose()?.unwrap_or_default();
    Ok((name, extensions))
}

fn strings(value: &Json) -> Result<Vec<String>, String> {
    match value {
        Json::Array(items) => items
            .iter()
            .map(|item| item.as_str().map(str::to_string).ok_or_else(|| "expected a list of strings".to_string()))
            .collect(),
        _ => Err("expected a list".to_string()),
    }
}

impl Definition {
    pub fn parse(text: &str) -> Result<Self, String> {
        let root: Json = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let map = root.as_object().ok_or("the definition must be a JSON object")?;
        let mut definition = Definition {
            name: String::new(),
            extensions: Vec::new(),
            lexemes: HashMap::new(),
            tiers: Vec::new(),
            right_associative: Vec::new(),
            identifier_unicode: false,
            identifiers_case_insensitive: false,
            keywords_case_insensitive: false,
            type_first: false,
            block_style: BlockStyle::Indentation,
            indent_size: 4,
            reserved: Vec::new(),
        };
        let mut style_seen = false;
        for key in LABELS {
            if !map.contains_key(*key) {
                return Err(format!("missing label '{key}'"));
            }
        }
        for key in map.keys() {
            if !key.starts_with("$comment") && !LABELS.contains(&key.as_str()) {
                return Err(format!("unknown label '{key}'"));
            }
        }
        for (key, value) in map {
            match (key.as_str(), value) {
                (k, _) if k.starts_with("$comment") => {}
                ("language", Json::String(name)) => definition.name = name.clone(),
                ("extensions", value) => definition.extensions = strings(value)?,
                ("op.precedence", Json::Array(tiers)) => {
                    definition.tiers = tiers.iter().map(strings).collect::<Result<_, _>>()?;
                }
                ("op.right_associative", value) => definition.right_associative = strings(value)?,
                ("identifier.unicode", Json::Bool(flag)) => definition.identifier_unicode = *flag,
                ("identifier.case_insensitive", Json::Bool(flag)) => definition.identifiers_case_insensitive = *flag,
                ("lexical.keywords_case_insensitive", Json::Bool(flag)) => definition.keywords_case_insensitive = *flag,
                ("stmt.let.type_first", Json::Bool(flag)) => definition.type_first = *flag,
                ("block.style", Json::String(style)) => {
                    style_seen = true;
                    definition.block_style = match style.as_str() {
                        "indentation" => BlockStyle::Indentation,
                        "braces" => BlockStyle::Braces,
                        "keyword" => BlockStyle::Keyword,
                        other => return Err(format!("block.style must be 'indentation', 'braces' or 'keyword', got '{other}'")),
                    };
                }
                ("block.indent_size", Json::Number(n)) => {
                    definition.indent_size = n.as_u64().ok_or("block.indent_size must be a count")? as usize;
                }
                (_, Json::Array(_)) => {
                    definition.lexemes.insert(key.clone(), strings(value)?);
                }
                _ => {} // Settings this kernel has no use for.
            }
        }
        if definition.name.is_empty() {
            return Err("missing label 'language'".to_string());
        }
        if !style_seen {
            return Err("missing label 'block.style'".to_string());
        }
        for key in UNSUPPORTED {
            if !definition.list(key).is_empty() {
                return Err(format!("label '{key}' is not implemented by the stream kernel; leave it empty"));
            }
        }
        let opens = definition.list("block.open").len();
        let closes = definition.list("block.close").len();
        let paired = match definition.block_style {
            BlockStyle::Indentation => opens == 0 && closes == 0,
            BlockStyle::Braces => opens > 0 && opens == closes,
            BlockStyle::Keyword => opens == 0 && closes > 0,
        };
        if !paired {
            return Err("block.open and block.close do not fit block.style".to_string());
        }
        if definition.list("lexical.comment_block.open").len() != definition.list("lexical.comment_block.close").len() {
            return Err("lexical.comment_block.open and .close must pair up position by position".to_string());
        }
        if definition.type_first {
            if definition.list("stmt.let").is_empty() {
                return Err("stmt.let.type_first needs stmt.let to list the type words".to_string());
            }
            for label in ["stmt.let.mutable", "stmt.let.annotation", "stmt.function", "stmt.function.returns"] {
                if !definition.list(label).is_empty() {
                    return Err(format!("stmt.let.type_first leaves no place for {label}; leave it empty"));
                }
            }
        }
        if !definition.list("syntax.call.label").is_empty() && definition.list("syntax.call.open").is_empty() {
            return Err("syntax.call.label needs syntax.call.open".to_string());
        }
        definition.reserved = definition.collect_reserved_words();
        Ok(definition)
    }

    /// Every spelling of a label, canonical first.
    pub fn list(&self, label: &str) -> &[String] {
        match self.lexemes.get(label) {
            Some(list) => list,
            None => panic!("the stream kernel asked for a label the definition lacks: {label}"),
        }
    }

    /// Whether `lexeme` is a spelling of `label`.
    pub fn is(&self, label: &str, lexeme: &str) -> bool {
        self.list(label).iter().any(|s| s == lexeme)
    }

    /// The canonical spelling of a label, which the language must have.
    pub fn first(&self, label: &str) -> &str {
        match self.list(label).first() {
            Some(s) => s,
            None => panic!("the stream kernel needs a spelling for {label}, and {} has none", self.name),
        }
    }

    /// The canonical spelling of a label, or `fallback` when the language has none.
    pub fn word_or<'a>(&'a self, label: &str, fallback: &'a str) -> &'a str {
        self.list(label).first().map_or(fallback, String::as_str)
    }

    /// Single-character spellings of a label, e.g. string quotes.
    pub fn chars(&self, label: &str) -> Vec<char> {
        self.list(label).iter().filter_map(|s| single(s)).collect()
    }

    pub fn first_char(&self, label: &str) -> Option<char> {
        self.list(label).first().and_then(|s| single(s))
    }

    /// Whether `name` spells any builtin.
    pub fn is_builtin(&self, name: &str) -> bool {
        self.lexemes.iter().any(|(label, list)| names_builtins(label) && list.iter().any(|s| s == name))
    }

    /// Builtin names not shaped like one word (`println!`, `console.log`);
    /// the lexer recognises them whole, as it does reserved words.
    pub fn compound_builtins(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .lexemes
            .iter()
            .filter(|(label, _)| names_builtins(label))
            .flat_map(|(_, list)| list.iter().filter(|s| !self.word_shaped(s)).cloned())
            .collect();
        names.sort();
        names.dedup();
        names
    }

    /// Precedence of a binary operator: the first tier it appears in.
    pub fn binary_precedence(&self, lexeme: &str) -> u32 {
        match self.tiers.iter().position(|tier| tier.iter().any(|s| s == lexeme)) {
            Some(index) => index as u32 + 1,
            None => panic!("operator '{lexeme}' has no tier in op.precedence"),
        }
    }

    /// Precedence of a unary operator: the last tier it appears in.
    pub fn unary_precedence(&self, lexeme: &str) -> u32 {
        match self.tiers.iter().rposition(|tier| tier.iter().any(|s| s == lexeme)) {
            Some(index) => index as u32 + 1,
            None => panic!("operator '{lexeme}' has no tier in op.precedence"),
        }
    }

    /// One above every operator tier, for postfix forms such as indexing.
    pub fn postfix_precedence(&self) -> u32 {
        self.tiers.len() as u32 + 1
    }

    pub fn is_right_associative(&self, lexeme: &str) -> bool {
        self.right_associative.iter().any(|s| s == lexeme)
    }

    /// Word-shaped lexemes the lexer must recognise whole: statement
    /// keywords, literal words, word-form operators, block words, the
    /// builtins this kernel parses as statements, and the memoization switch.
    pub fn reserved_words(&self) -> &[String] {
        &self.reserved
    }

    fn collect_reserved_words(&self) -> Vec<String> {
        let mut words = Vec::new();
        for (label, list) in &self.lexemes {
            let reserved = label.starts_with("stmt.")
                || label.starts_with("literal.")
                || label.starts_with("op.")
                || label.starts_with("block.")
                || matches!(label.as_str(), "builtin.emit" | "builtin.push" | "builtin.extern" | "system.memoization");
            if reserved {
                words.extend(list.iter().filter(|s| self.word_shaped(s)).cloned());
            }
        }
        words.sort();
        words.dedup();
        words
    }

    /// Whether `word` is reserved, compared case-insensitively when the
    /// language folds keywords.
    pub fn is_reserved(&self, word: &str) -> bool {
        if self.keywords_case_insensitive {
            let folded = word.to_lowercase();
            self.reserved.iter().any(|w| *w == folded)
        } else {
            self.reserved.iter().any(|w| w == word)
        }
    }

    /// Symbol lexemes the lexer must recognise whole: operators, brackets,
    /// block delimiters, terminators, comment markers and the prologue.
    pub fn symbols(&self) -> Vec<String> {
        let mut symbols = Vec::new();
        for (label, list) in &self.lexemes {
            let symbolic = label.starts_with("op.")
                || label.starts_with("syntax.")
                || label.starts_with("block.")
                || label.starts_with("lexical.comment")
                || matches!(
                    label.as_str(),
                    "stmt.assign" | "stmt.let.annotation" | "stmt.function.returns" | "stmt.terminator" | "lexical.prologue"
                );
            if symbolic {
                symbols.extend(list.iter().filter(|s| !self.word_shaped(s)).cloned());
            }
        }
        symbols.sort();
        symbols.dedup();
        symbols
    }

    fn word_shaped(&self, s: &str) -> bool {
        let unicode = self.identifier_unicode;
        let start = |c: char| c == '_' || if unicode { c.is_alphabetic() } else { c.is_ascii_alphabetic() };
        let cont = |c: char| c == '_' || if unicode { c.is_alphanumeric() } else { c.is_ascii_alphanumeric() };
        let mut chars = s.chars();
        chars.next().map_or(false, start) && chars.all(cont)
    }
}

/// Whether a label lists builtin names (rather than the print placeholders).
fn names_builtins(label: &str) -> bool {
    label.starts_with("builtin.") && label != "builtin.print.placeholder"
}

fn single(s: &str) -> Option<char> {
    let mut chars = s.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => Some(c),
        _ => None,
    }
}
