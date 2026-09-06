// Lumen's spelling, read from configs/lumen.json.
//
// The stream kernel hosts Lumen in code: handlers give constructs their
// meaning. Which strings spell those constructs is not the code's to decide.
// Every lexeme a handler matches comes from the shared definition, so both
// kernels read one file and a change to Lumen's spelling is a data edit.
// Labels are looked up by name; asking for a label the definition lacks is
// a programming error and stops the interpreter at once.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde_json::Value as Json;

const LUMEN: &str = include_str!("../../../../../configs/lumen.json");

pub struct Definition {
    lexemes: HashMap<String, Vec<String>>,
    tiers: Vec<Vec<String>>,
    right_associative: Vec<String>,
    pub identifier_unicode: bool,
    pub indent_size: usize,
}

/// The definition, parsed once per process.
pub fn def() -> &'static Definition {
    static DEFINITION: OnceLock<Definition> = OnceLock::new();
    DEFINITION.get_or_init(|| Definition::parse(LUMEN).unwrap_or_else(|e| panic!("configs/lumen.json: {e}")))
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
    fn parse(text: &str) -> Result<Self, String> {
        let root: Json = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let map = root.as_object().ok_or("the definition must be a JSON object")?;
        let mut definition = Definition {
            lexemes: HashMap::new(),
            tiers: Vec::new(),
            right_associative: Vec::new(),
            identifier_unicode: false,
            indent_size: 4,
        };
        for (key, value) in map {
            match (key.as_str(), value) {
                (k, _) if k.starts_with("$comment") => {}
                ("op.precedence", Json::Array(tiers)) => {
                    definition.tiers = tiers.iter().map(strings).collect::<Result<_, _>>()?;
                }
                ("op.right_associative", value) => definition.right_associative = strings(value)?,
                ("identifier.unicode", Json::Bool(flag)) => definition.identifier_unicode = *flag,
                ("block.indent_size", Json::Number(n)) => {
                    definition.indent_size = n.as_u64().ok_or("block.indent_size must be a count")? as usize;
                }
                (_, Json::Array(_)) => {
                    definition.lexemes.insert(key.clone(), strings(value)?);
                }
                _ => {} // Settings the stream kernel has no use for.
            }
        }
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

    /// The canonical spelling of a label.
    pub fn first(&self, label: &str) -> &str {
        match self.list(label).first() {
            Some(s) => s,
            None => panic!("the stream kernel needs a spelling for {label}, and Lumen has none"),
        }
    }

    /// Single-character spellings of a label, e.g. string quotes.
    pub fn chars(&self, label: &str) -> Vec<char> {
        self.list(label).iter().filter_map(|s| single(s)).collect()
    }

    pub fn first_char(&self, label: &str) -> Option<char> {
        self.list(label).first().and_then(|s| single(s))
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
    /// keywords, literal words, word-form operators, the builtins this
    /// kernel parses as statements, and the memoization switch.
    pub fn reserved_words(&self) -> Vec<String> {
        let mut words = Vec::new();
        for (label, list) in &self.lexemes {
            let reserved = label.starts_with("stmt.")
                || label.starts_with("literal.")
                || label.starts_with("op.")
                || matches!(label.as_str(), "builtin.emit" | "builtin.push" | "builtin.extern" | "system.memoization");
            if reserved {
                words.extend(list.iter().filter(|s| self.word_shaped(s)).cloned());
            }
        }
        words.sort();
        words.dedup();
        words
    }

    /// Symbol lexemes the lexer must recognise whole: operators, brackets,
    /// the assignment sign and the annotation mark.
    pub fn symbols(&self) -> Vec<String> {
        let mut symbols = Vec::new();
        for (label, list) in &self.lexemes {
            let symbolic = label.starts_with("op.")
                || label.starts_with("syntax.")
                || matches!(label.as_str(), "stmt.assign" | "stmt.let.annotation");
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
        let mut chars = s.chars();
        chars.next().map_or(false, |c| crate::languages::word_start(unicode, c))
            && chars.all(|c| crate::languages::word_char(unicode, c))
    }
}

fn single(s: &str) -> Option<char> {
    let mut chars = s.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => Some(c),
        _ => None,
    }
}
