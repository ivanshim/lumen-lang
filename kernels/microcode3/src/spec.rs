// The definition as data. Every label is read into a typed cell, checked
// for shape, and a few tables are derived for the stages.

use std::collections::{HashMap, HashSet};

use serde_json::Value as Json;

use crate::tree::Op;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Layout {
    Indent,
    Braces,
    Keyword,
    Postfix,
}

#[derive(Debug)]
enum Cell {
    List(Vec<String>),
    Bool(bool),
    Number(Option<usize>),
    Word(Option<String>),
    Table(Vec<Vec<String>>),
}

#[derive(Clone, Copy, Debug)]
pub struct Operator {
    pub op: Op,
    pub tier: u32,
    pub right: bool,
}

pub struct Spec {
    pub name: String,
    pub layout: Layout,
    cells: HashMap<&'static str, Cell>,
    pub binary: HashMap<String, Operator>,
    pub unary: HashMap<String, Operator>,
    pub syntax_tier: HashMap<String, u32>,
    pub builtins: HashMap<String, Op>,
    pub reserved: HashSet<String>,
    pub symbols: Vec<String>,
}

// Label shapes: L list of words, B boolean, N count or null, W word, O word or null, T tiers.
const LABELS: &str = "\
format_version:N language:W extensions:L \
lexical.comment_line:L lexical.comment_block.open:L lexical.comment_block.close:L \
lexical.string_quotes:L lexical.raw_quotes:L lexical.string_escapes:L lexical.prologue:L lexical.name_quote:L \
lexical.number.decimal_point:L lexical.number.base_marker:L lexical.number.exponent_marker:L \
lexical.number.hex_prefix:L lexical.keywords_case_insensitive:B \
identifier.unicode:B identifier.variable_prefix:L identifier.case_insensitive:B \
block.style:W block.open:L block.close:L block.intro:L block.indent_size:N \
stmt.terminator:L \
syntax.group.open:L syntax.group.close:L \
syntax.call.open:L syntax.call.separator:L syntax.call.close:L syntax.call.label:L \
syntax.array.open:L syntax.array.separator:L syntax.array.close:L \
syntax.map.open:L syntax.map.separator:L syntax.map.pair:L syntax.map.close:L \
literal.true:L literal.false:L literal.null:L \
op.precedence:T op.right_associative:L \
op.add:L op.sub:L op.mul:L op.div:L op.div.result:O op.quot:L op.rem:L op.pow:L \
op.eq:L op.ne:L op.lt:L op.le:L op.gt:L op.ge:L \
op.and:L op.or:L op.not:L op.negate:L op.concat:L op.range:L \
op.index.open:L op.index.close:L op.index.strings:B op.pipe:L \
stmt.assign:L stmt.let:L stmt.let.mutable:L stmt.let.annotation:L stmt.let.type_first:B \
stmt.if:L stmt.elif:L stmt.else:L stmt.while:L stmt.until:L stmt.for:L stmt.for.in:L \
stmt.foreach:L stmt.foreach.as:L stmt.foreach.pair:L \
stmt.return:L stmt.break:L stmt.continue:L stmt.function:L stmt.function.returns:L \
stmt.function.result_by_name:B stmt.pass:L stmt.emit:L \
stack.dup:L stack.drop:L stack.swap:L stack.over:L stack.rot:L stack.eval:L \
stack.program.open:L stack.program.close:L \
builtin.emit:L builtin.print:L builtin.write:L builtin.print.placeholder:L \
builtin.len:L builtin.char_at:L builtin.ord:L builtin.chr:L builtin.typeof:L builtin.error:L \
builtin.extern:L builtin.range:L builtin.real:L builtin.num:L builtin.den:L builtin.push:L \
builtin.get:L builtin.put:L builtin.precision:L builtin.to_string:L builtin.to_int:L builtin.to_real:L \
system.args:L system.memoization:L system.real_default_precision:L system.entry:L \
system.kind.integer:L system.kind.rational:L system.kind.real:L system.kind.string:L \
system.kind.boolean:L system.kind.array:L system.kind.null:L";

const EMPTY_ONLY: [&str; 8] = [
    "syntax.map.open", "syntax.map.separator", "syntax.map.pair", "syntax.map.close",
    "stmt.foreach", "stmt.foreach.as", "stmt.foreach.pair", "stmt.emit",
];

/// Builtin labels and the operation each names.
pub const BUILTIN_LABELS: [(&str, Op); 21] = [
    ("builtin.emit", Op::Emit), ("builtin.print", Op::Print), ("builtin.write", Op::Write), ("builtin.len", Op::Len),
    ("builtin.char_at", Op::CharAt), ("builtin.ord", Op::Ord), ("builtin.chr", Op::Chr), ("builtin.typeof", Op::Kind),
    ("builtin.error", Op::Error), ("builtin.extern", Op::Extern), ("builtin.range", Op::Range), ("builtin.real", Op::Real),
    ("builtin.precision", Op::Precision), ("builtin.to_string", Op::ToString), ("builtin.to_int", Op::ToInt),
    ("builtin.to_real", Op::ToReal), ("builtin.num", Op::Num), ("builtin.den", Op::Den), ("builtin.push", Op::Push),
    ("builtin.get", Op::Get), ("builtin.put", Op::Put),
];

const BINARY_LABELS: [(&str, Op); 16] = [
    ("op.add", Op::Add), ("op.sub", Op::Sub), ("op.mul", Op::Mul), ("op.div", Op::Div), ("op.quot", Op::Quot),
    ("op.rem", Op::Rem), ("op.pow", Op::Pow), ("op.eq", Op::Eq), ("op.ne", Op::Ne), ("op.lt", Op::Lt), ("op.le", Op::Le),
    ("op.gt", Op::Gt), ("op.ge", Op::Ge), ("op.and", Op::And), ("op.or", Op::Or), ("op.concat", Op::Concat),
];

fn as_object(text: &str) -> Result<serde_json::Map<String, Json>, String> {
    match serde_json::from_str(text).map_err(|e| format!("definition is not valid JSON: {e}"))? {
        Json::Object(m) => Ok(m),
        _ => Err("definition must be a JSON object".to_string()),
    }
}

pub fn describe(text: &str) -> Result<(String, Vec<String>), String> {
    let m = as_object(text)?;
    let name = m.get("language").and_then(Json::as_str).filter(|s| !s.is_empty()).ok_or("label 'language' must be a non-empty string")?;
    let exts = m.get("extensions").and_then(Json::as_array).ok_or("label 'extensions' must be a list of strings")?;
    Ok((name.to_string(), exts.iter().filter_map(|e| e.as_str().map(str::to_string)).collect()))
}

fn word_list(key: &str, json: &Json) -> Result<Vec<String>, String> {
    let Json::Array(items) = json else { return Err(format!("label '{key}' must be a list of non-empty strings")) };
    items
        .iter()
        .map(|i| match i.as_str() {
            Some(s) if !s.is_empty() => Ok(s.to_string()),
            _ => Err(format!("label '{key}' must be a list of non-empty strings")),
        })
        .collect()
}

impl Spec {
    pub fn read(text: &str) -> Result<Spec, String> {
        let map = as_object(text)?;
        let mut cells = HashMap::new();
        let known: Vec<(&'static str, char)> = LABELS.split_whitespace().map(|e| {
            let (k, s) = e.rsplit_once(':').unwrap();
            (k, s.chars().next().unwrap())
        }).collect();
        for (key, shape) in &known {
            let json = map.get(*key).ok_or_else(|| format!("missing label '{key}'"))?;
            let cell = match (shape, json) {
                ('L', j) => Cell::List(word_list(key, j)?),
                ('B', Json::Bool(b)) => Cell::Bool(*b),
                ('N', Json::Null) => Cell::Number(None),
                ('N', Json::Number(n)) if n.as_u64().is_some() => Cell::Number(n.as_u64().map(|n| n as usize)),
                ('W', Json::String(s)) if !s.is_empty() => Cell::Word(Some(s.clone())),
                ('O', Json::Null) => Cell::Word(None),
                ('O', Json::String(s)) if !s.is_empty() => Cell::Word(Some(s.clone())),
                ('T', Json::Array(tiers)) => Cell::Table(tiers.iter().map(|t| word_list(key, t)).collect::<Result<_, _>>()?),
                _ => return Err(format!("label '{key}' has the wrong shape")),
            };
            cells.insert(*key, cell);
        }
        let mut strange: Vec<&str> = map.keys().map(String::as_str).filter(|k| !k.starts_with('$') && !known.iter().any(|(l, _)| l == k)).collect();
        strange.sort();
        if !strange.is_empty() {
            return Err(format!("unknown label(s): {}", strange.join(", ")));
        }
        let mut spec = Spec {
            name: String::new(),
            layout: Layout::Indent,
            cells,
            binary: HashMap::new(),
            unary: HashMap::new(),
            syntax_tier: HashMap::new(),
            builtins: HashMap::new(),
            reserved: HashSet::new(),
            symbols: Vec::new(),
        };
        spec.name = spec.word("language").unwrap_or("").to_string();
        spec.validate()?;
        spec.tables()?;
        Ok(spec)
    }

    pub fn list(&self, key: &str) -> &[String] {
        match self.cells.get(key) {
            Some(Cell::List(l)) => l,
            _ => panic!("'{key}' is not a list label"),
        }
    }

    pub fn one(&self, key: &str) -> Option<&str> {
        self.list(key).first().map(String::as_str)
    }

    pub fn any(&self, key: &str) -> bool {
        !self.list(key).is_empty()
    }

    pub fn is(&self, key: &str, lexeme: &str) -> bool {
        self.list(key).iter().any(|w| w == lexeme)
    }

    pub fn on(&self, key: &str) -> bool {
        matches!(self.cells.get(key), Some(Cell::Bool(true)))
    }

    pub fn number(&self, key: &str) -> Option<usize> {
        match self.cells.get(key) {
            Some(Cell::Number(n)) => *n,
            _ => None,
        }
    }

    pub fn word(&self, key: &str) -> Option<&str> {
        match self.cells.get(key) {
            Some(Cell::Word(w)) => w.as_deref(),
            _ => None,
        }
    }

    fn table(&self) -> &[Vec<String>] {
        match self.cells.get("op.precedence") {
            Some(Cell::Table(t)) => t,
            _ => &[],
        }
    }

    pub fn ch(&self, key: &str) -> Option<char> {
        let mut it = self.one(key)?.chars();
        match (it.next(), it.next()) {
            (Some(c), None) => Some(c),
            _ => None,
        }
    }

    pub fn chars(&self, key: &str) -> Vec<char> {
        self.list(key).iter().filter_map(|w| if w.chars().count() == 1 { w.chars().next() } else { None }).collect()
    }

    pub fn error_prefix(&self) -> String {
        let mut it = self.name.chars();
        it.next().map_or("Error".to_string(), |c| format!("{}{}Error", c.to_uppercase(), it.as_str()))
    }

    pub fn starts_word(&self, c: char) -> bool {
        c == '_' || if self.on("identifier.unicode") { c.is_alphabetic() } else { c.is_ascii_alphabetic() }
    }

    pub fn in_word(&self, c: char) -> bool {
        c == '_' || if self.on("identifier.unicode") { c.is_alphanumeric() } else { c.is_ascii_alphanumeric() }
    }

    pub fn looks_like_word(&self, s: &str) -> bool {
        let mut it = s.chars();
        let mut first = it.next();
        if first.is_some() && first == self.ch("identifier.variable_prefix") {
            first = it.next();
        }
        first.map_or(false, |c| self.starts_word(c)) && it.all(|c| self.in_word(c))
    }

    fn validate(&mut self) -> Result<(), String> {
        if self.number("format_version") != Some(1) {
            return Err("format_version must be 1".to_string());
        }
        for key in EMPTY_ONLY {
            if self.any(key) {
                return Err(format!("label '{key}' is not implemented by the microcode3 kernel; leave it empty"));
            }
        }
        let singles = ["lexical.string_quotes", "lexical.raw_quotes", "lexical.string_escapes", "lexical.name_quote",
            "lexical.number.decimal_point", "lexical.number.base_marker", "lexical.number.exponent_marker", "identifier.variable_prefix"];
        for key in singles {
            if let Some(w) = self.list(key).iter().find(|w| w.chars().count() != 1) {
                return Err(format!("label '{key}' takes single characters, got '{w}'"));
            }
        }
        if self.list("lexical.comment_block.open").len() != self.list("lexical.comment_block.close").len() {
            return Err("lexical.comment_block.open and .close must pair up position by position".to_string());
        }
        if let Some(p) = self.one("lexical.number.hex_prefix") {
            if p.chars().count() != 2 || !p.starts_with(|c: char| c.is_ascii_digit()) {
                return Err(format!("lexical.number.hex_prefix must be a digit followed by one letter, got '{p}'"));
            }
        }
        self.layout = match self.word("block.style") {
            Some("indentation") => Layout::Indent,
            Some("braces") => Layout::Braces,
            Some("keyword") => Layout::Keyword,
            Some("postfix") => Layout::Postfix,
            other => return Err(format!("block.style must be 'indentation', 'braces', 'keyword' or 'postfix', got '{}'", other.unwrap_or(""))),
        };
        let (o, c) = (self.list("block.open").len(), self.list("block.close").len());
        let fits = match self.layout {
            Layout::Indent => o == 0 && c == 0 && self.number("block.indent_size").map_or(false, |n| n > 0),
            Layout::Braces => o > 0 && o == c,
            _ => o == 0 && c > 0,
        };
        if !fits {
            return Err("block.open, block.close and block.indent_size do not fit block.style".to_string());
        }
        let postfix = self.layout == Layout::Postfix;
        if postfix && !self.table().is_empty() {
            return Err("a postfix language takes no op.precedence".to_string());
        }
        let stack_labels = ["stack.dup", "stack.drop", "stack.swap", "stack.over", "stack.rot", "stack.eval", "stack.program.open", "stack.program.close"];
        if !postfix && stack_labels.iter().any(|l| self.any(l)) {
            return Err("the stack.* labels need block.style 'postfix'".to_string());
        }
        for (a, b) in [("syntax.group.open", "syntax.group.close"), ("syntax.call.open", "syntax.call.close"),
                       ("syntax.array.open", "syntax.array.close"), ("op.index.open", "op.index.close"),
                       ("stack.program.open", "stack.program.close")] {
            if self.list(a).len() != self.list(b).len() && (self.list(a).is_empty() || self.list(b).is_empty()) {
                return Err(format!("labels '{a}' and '{b}' must be given together"));
            }
        }
        if self.any("syntax.call.label") && !self.any("syntax.call.open") {
            return Err("syntax.call.label needs syntax.call.open".to_string());
        }
        if self.on("op.index.strings") && !self.any("op.index.open") {
            return Err("op.index.strings needs op.index.open".to_string());
        }
        if !matches!(self.word("op.div.result"), None | Some("rational") | Some("real")) {
            return Err("op.div.result must be 'rational', 'real' or null".to_string());
        }
        if !self.any("stmt.let") && (self.any("stmt.let.mutable") || self.any("stmt.let.annotation") || self.on("stmt.let.type_first")) {
            return Err("stmt.let.mutable, stmt.let.annotation and stmt.let.type_first need stmt.let".to_string());
        }
        if self.on("stmt.let.type_first") {
            for l in ["stmt.let.mutable", "stmt.let.annotation", "stmt.function", "stmt.function.returns"] {
                if self.any(l) {
                    return Err(format!("stmt.let.type_first leaves no place for {l}; leave it empty"));
                }
            }
        }
        if !self.any("stmt.if") && (self.any("stmt.elif") || self.any("stmt.else")) {
            return Err("stmt.elif and stmt.else need stmt.if".to_string());
        }
        if self.any("stmt.for") != self.any("stmt.for.in") && !(postfix && !self.any("stmt.for.in")) {
            return Err("stmt.for and stmt.for.in must be given together".to_string());
        }
        if self.on("stmt.function.result_by_name") && !self.any("stmt.function") {
            return Err("stmt.function.result_by_name needs stmt.function".to_string());
        }
        let keywords = ["stmt.let", "stmt.let.mutable", "stmt.if", "stmt.elif", "stmt.else", "stmt.while", "stmt.until", "stmt.for",
            "stmt.for.in", "stmt.return", "stmt.break", "stmt.continue", "stmt.function", "stmt.pass", "literal.true", "literal.false", "literal.null"];
        for key in keywords {
            if let Some(w) = self.list(key).iter().find(|w| !self.looks_like_word(w)) {
                return Err(format!("keyword '{w}' must be shaped like an identifier"));
            }
        }
        for key in ["system.args", "system.memoization", "system.real_default_precision", "system.entry", "system.kind.integer",
                    "system.kind.rational", "system.kind.real", "system.kind.string", "system.kind.boolean", "system.kind.array", "system.kind.null"] {
            if let Some(w) = self.one(key).filter(|w| !self.looks_like_word(w)) {
                return Err(format!("system name '{w}' must be shaped like an identifier"));
            }
        }
        Ok(())
    }

    fn tables(&mut self) -> Result<(), String> {
        let postfix = self.layout == Layout::Postfix;
        let table = self.table().to_vec();
        let place = |lex: &str, last: bool| -> Option<u32> {
            if postfix {
                return Some(1);
            }
            let at = if last { table.iter().rposition(|t| t.contains(&lex.to_string())) } else { table.iter().position(|t| t.contains(&lex.to_string())) };
            at.map(|i| i as u32 + 1)
        };
        let real_division = self.word("op.div.result") == Some("real");
        for (label, op) in BINARY_LABELS {
            let op = if label == "op.div" && real_division { Op::DivReal } else { op };
            for lex in self.list(label).to_vec() {
                let tier = place(&lex, false).ok_or_else(|| format!("'{lex}' ({label}) does not appear in op.precedence"))?;
                let right = self.is("op.right_associative", &lex);
                if self.binary.insert(lex.clone(), Operator { op, tier, right }).is_some() {
                    return Err(format!("'{lex}' is listed under two binary operator labels"));
                }
            }
        }
        for (label, op) in [("op.not", Op::Not), ("op.negate", Op::Neg)] {
            for lex in self.list(label).to_vec() {
                let tier = place(&lex, true).ok_or_else(|| format!("'{lex}' ({label}) does not appear in op.precedence"))?;
                if self.unary.insert(lex.clone(), Operator { op, tier, right: false }).is_some() {
                    return Err(format!("'{lex}' is listed under two unary operator labels"));
                }
            }
        }
        for label in ["op.range", "op.pipe"] {
            for lex in self.list(label).to_vec() {
                let tier = place(&lex, false).ok_or_else(|| format!("'{lex}' ({label}) does not appear in op.precedence"))?;
                self.syntax_tier.insert(lex, tier);
            }
        }
        for lex in table.iter().flatten() {
            if !self.binary.contains_key(lex) && !self.unary.contains_key(lex) && !self.syntax_tier.contains_key(lex) {
                return Err(format!("op.precedence lists '{lex}', which is under no operator label"));
            }
        }
        for lex in self.list("op.right_associative") {
            if !self.binary.contains_key(lex) {
                return Err(format!("op.right_associative lists '{lex}', which is not a binary operator"));
            }
        }
        let quotes = self.chars("lexical.string_quotes");
        for (label, op) in BUILTIN_LABELS {
            for lex in self.list(label).to_vec() {
                let fine = lex.starts_with(|c: char| c == '_' || c.is_alphabetic()) && !lex.chars().any(|c| c.is_whitespace() || quotes.contains(&c));
                if !fine {
                    return Err(format!("builtin name '{lex}' must begin like an identifier and hold no spaces or quotes"));
                }
                if self.builtins.insert(lex, op).is_some() {
                    return Err(format!("a builtin name is listed under two builtin labels"));
                }
            }
        }
        let mut all: Vec<String> = self.binary.keys().chain(self.unary.keys()).chain(self.syntax_tier.keys()).cloned().collect();
        let symbol_labels = ["syntax.group.open", "syntax.group.close", "syntax.call.open", "syntax.call.separator", "syntax.call.close",
            "syntax.call.label", "syntax.array.open", "syntax.array.separator", "syntax.array.close", "op.index.open", "op.index.close",
            "block.intro", "stmt.assign", "stmt.terminator", "stmt.let.annotation", "stmt.function.returns", "stack.dup", "stack.drop",
            "stack.swap", "stack.over", "stack.rot", "stack.eval", "stack.program.open", "stack.program.close", "stmt.let",
            "stmt.let.mutable", "stmt.if", "stmt.elif", "stmt.else", "stmt.while", "stmt.until", "stmt.for", "stmt.for.in",
            "stmt.return", "stmt.break", "stmt.continue", "stmt.function", "stmt.pass", "literal.true", "literal.false", "literal.null"];
        for key in symbol_labels {
            all.extend(self.list(key).iter().cloned());
        }
        if self.layout != Layout::Indent {
            all.extend(self.list("block.open").iter().cloned());
            all.extend(self.list("block.close").iter().cloned());
        }
        let fold = self.on("lexical.keywords_case_insensitive");
        for lex in all {
            if self.looks_like_word(&lex) {
                self.reserved.insert(if fold { lex.to_lowercase() } else { lex });
            } else if !self.symbols.contains(&lex) {
                self.symbols.push(lex);
            }
        }
        self.symbols.sort_by(|a, b| b.len().cmp(&a.len()).then(a.cmp(b)));
        Ok(())
    }
}
