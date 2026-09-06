// Language definition: the data that turns the kernel into a language.
//
// A definition is one flat JSON object (langs/<language>.json) mapping a
// fixed set of labels, the things the kernel can do, to the strings the
// language spells them with. This module reads that object strictly: every
// label present, every value of the right type, no unknown keys, and every
// keyword shaped like a word. It builds the tables the four stages consume
// and contains no keyword, operator or function name of its own.

use std::collections::{HashMap, HashSet};

use serde_json::{Map, Value as Json};

pub struct LanguageSchema {
    pub name: String,
    pub extensions: Vec<String>,
    /// Prefix used when reporting errors, e.g. `LumenError`.
    pub error_prefix: String,
    pub lexical: Lexical,
    pub structure: Structure,
    pub literals: Literals,
    pub operators: Operators,
    pub statements: Statements,
    /// Surface function name → kernel built-in.
    pub functions: HashMap<String, Builtin>,
    pub system: System,
}

/// Whether `word` is one of the spellings in `list`.
pub fn spelled(list: &[String], word: &str) -> bool {
    list.iter().any(|w| w == word)
}

// ---------------- Stage 1: lexical tables ----------------

pub struct Lexical {
    /// Line-comment markers; comments are removed before lexing.
    pub comment_lines: Vec<String>,
    /// Block-comment delimiter pairs.
    pub comment_blocks: Vec<(String, String)>,
    /// String delimiters.
    pub quotes: Vec<char>,
    /// Escape tables per delimiter: escape letter → replacement text.
    /// A backslash followed by an unlisted letter is kept verbatim.
    pub escapes: HashMap<char, HashMap<char, String>>,
    /// Text that may open the file and is ignored, e.g. `<?php`.
    pub prologue: Option<String>,
    /// Symbol lexemes the ingest stage segments on; longest match wins.
    pub operators: Vec<String>,
    pub number: NumberSyntax,
    /// Whether identifiers may use letters and digits beyond ASCII.
    pub identifier_unicode: bool,
    /// Character that begins a variable name, e.g. `$`.
    pub variable_prefix: Option<char>,
    /// Keywords match regardless of letter case.
    pub keywords_case_insensitive: bool,
    /// Identifiers are folded to lower case.
    pub identifiers_case_insensitive: bool,
    /// Every keyword, literal word and word-form operator.
    pub reserved_words: HashSet<String>,
}

#[derive(Default)]
pub struct NumberSyntax {
    pub decimal_point: Option<char>,
    /// `<base>@<digits>` literals, e.g. `16@FF`.
    pub base_marker: Option<char>,
    /// Exponent marker inside base-N literals, e.g. `10@1.5^3`.
    pub exponent_marker: Option<char>,
    /// Prefix of hexadecimal literals, e.g. `0x`.
    pub hex_prefix: Option<String>,
}

// ---------------- Stage 2: structure tables ----------------

pub struct Structure {
    pub blocks: BlockStyle,
    pub indent_size: usize,
    /// Block delimiters, as parallel lists: the opener at one position pairs
    /// with the closer at the same position. Synthesised from indentation
    /// changes for `indentation`; present in the source otherwise. A
    /// `keyword` block has no opener, only closers.
    pub block_open: Vec<String>,
    pub block_close: Vec<String>,
    /// Tokens that end a block header (Python's `:`, Lua's `then`); dropped
    /// when present.
    pub block_intro: Vec<String>,
    /// Statement terminators besides line ends, e.g. `;`.
    pub terminators: Vec<String>,
    pub group: Option<Pair>,
    pub call: Option<Pair>,
    pub array: Option<Pair>,
    pub index: Option<Pair>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BlockStyle {
    Indentation,
    Braces,
    /// No opener; the body runs to a closing word, and an `if` chain shares one.
    Keyword,
}

#[derive(Clone)]
pub struct Pair {
    pub open: String,
    pub close: String,
    pub separator: Option<String>,
}

// ---------------- Stage 3: reduce tables ----------------

#[derive(Default)]
pub struct Literals {
    pub true_words: Vec<String>,
    pub false_words: Vec<String>,
    pub null_words: Vec<String>,
}

pub struct Operators {
    pub binary: HashMap<String, BinaryOp>,
    pub unary: HashMap<String, UnaryOp>,
}

#[derive(Clone)]
pub struct BinaryOp {
    pub precedence: f32,
    pub associativity: Assoc,
    pub op: Op,
}

#[derive(Clone)]
pub struct UnaryOp {
    pub precedence: f32,
    pub op: Op,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Assoc {
    Left,
    Right,
}

/// Kernel operations. Definitions map surface operators onto these; the
/// last group is used internally when statements are desugared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Quot,
    Rem,
    Pow,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Not,
    Negate,
    Concat,
    Range,
    Index,
    Pipe,
    ArrayLiteral,
    RangeStart,
    RangeEnd,
}

/// Statement forms and the words that introduce them. An empty list means
/// the language has no such statement.
#[derive(Default)]
pub struct Statements {
    pub assignment: Vec<String>,
    pub binding: Option<Binding>,
    pub branch: Option<Branch>,
    pub loop_while: Vec<String>,
    pub loop_until: Vec<String>,
    pub loop_for: Option<ForLoop>,
    pub return_: Vec<String>,
    pub break_: Vec<String>,
    pub continue_: Vec<String>,
    pub function: Vec<String>,
    /// Token introducing an (ignored) return type after the parameters, e.g. `->`.
    pub function_returns: Vec<String>,
    /// A no-op statement keyword, e.g. Python's `pass`.
    pub pass: Vec<String>,
}

pub struct Binding {
    pub keyword: Vec<String>,
    pub mutable_modifier: Vec<String>,
    /// Token introducing an (ignored) type annotation, e.g. `:`.
    pub type_annotation: Vec<String>,
}

pub struct Branch {
    pub keyword: Vec<String>,
    pub else_keyword: Vec<String>,
    pub elif_keyword: Vec<String>,
}

pub struct ForLoop {
    pub keyword: Vec<String>,
    pub in_keyword: Vec<String>,
}

// ---------------- Stage 4: execute tables ----------------

/// Built-in functions the kernel can provide. Which surface names reach
/// them, if any, is the definition's choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Builtin {
    Emit,
    PrintLine,
    Write,
    Real,
    IntToString,
    RealToString,
    RationalToString,
    BoolToString,
    ArrayToString,
    NullToString,
    KindToString,
    Len,
    CharAt,
    Ord,
    Chr,
    Error,
    Kind,
    Num,
    Den,
    Int,
    Frac,
    Extern,
    Push,
    Range,
}

#[derive(Default)]
pub struct System {
    /// Read-only binding holding the program arguments as one string.
    pub args: Option<String>,
    /// Binding that switches function-result caching on and off.
    pub memoization: Option<String>,
    /// Bindings for kind meta-values.
    pub kinds: HashMap<String, KindName>,
    /// Binding holding the default precision of reals.
    pub real_default_precision: Option<String>,
    /// Function invoked after the program body, if it was defined.
    pub entry: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum KindName {
    Integer,
    Rational,
    Real,
    String,
    Boolean,
    Array,
    Null,
}

impl LanguageSchema {
    pub fn from_json(text: &str) -> Result<Self, String> {
        let root: Json = serde_json::from_str(text).map_err(|e| format!("definition is not valid JSON: {e}"))?;
        let map = match root {
            Json::Object(map) => map,
            _ => return Err("definition must be a JSON object".to_string()),
        };
        let mut labels = Labels { map: &map, seen: HashSet::new() };
        let schema = build(&mut labels)?;
        labels.finish()?;
        Ok(schema)
    }

    /// The language name and file extensions of a definition, without
    /// validating the rest of it.
    pub fn describe(text: &str) -> Result<(String, Vec<String>), String> {
        let root: Json = serde_json::from_str(text).map_err(|e| format!("definition is not valid JSON: {e}"))?;
        let map = match root {
            Json::Object(map) => map,
            _ => return Err("definition must be a JSON object".to_string()),
        };
        let mut labels = Labels { map: &map, seen: HashSet::new() };
        Ok((labels.text("language")?, labels.lexemes("extensions")?))
    }

    /// Operator lexemes sorted longest first, for maximal munch.
    pub fn operators_longest_first(&self) -> Vec<&str> {
        let mut ops: Vec<&str> = self.lexical.operators.iter().map(String::as_str).collect();
        ops.sort_by(|a, b| b.len().cmp(&a.len()));
        ops
    }

    pub fn is_terminator(&self, lexeme: &str) -> bool {
        self.structure.terminators.iter().any(|t| t == lexeme)
    }
}

// ---------------- The reader ----------------

/// Typed access to the labels of one definition, remembering which were
/// read so that unknown keys can be reported at the end.
struct Labels<'a> {
    map: &'a Map<String, Json>,
    seen: HashSet<String>,
}

impl<'a> Labels<'a> {
    fn raw(&mut self, key: &str) -> Result<&'a Json, String> {
        self.seen.insert(key.to_string());
        self.map.get(key).ok_or_else(|| format!("missing label '{key}'"))
    }

    /// A lexeme label: a list of non-empty strings.
    fn lexemes(&mut self, key: &str) -> Result<Vec<String>, String> {
        let wrong = || format!("label '{key}' must be a list of non-empty strings");
        match self.raw(key)? {
            Json::Array(items) => items
                .iter()
                .map(|item| match item {
                    Json::String(s) if !s.is_empty() => Ok(s.clone()),
                    _ => Err(wrong()),
                })
                .collect(),
            _ => Err(wrong()),
        }
    }

    fn first(&mut self, key: &str) -> Result<Option<String>, String> {
        Ok(self.lexemes(key)?.into_iter().next())
    }

    /// A lexeme label whose entries are single characters.
    fn chars(&mut self, key: &str) -> Result<Vec<char>, String> {
        self.lexemes(key)?
            .iter()
            .map(|s| {
                let mut it = s.chars();
                match (it.next(), it.next()) {
                    (Some(c), None) => Ok(c),
                    _ => Err(format!("label '{key}' takes single characters, got '{s}'")),
                }
            })
            .collect()
    }

    fn first_char(&mut self, key: &str) -> Result<Option<char>, String> {
        Ok(self.chars(key)?.into_iter().next())
    }

    fn flag(&mut self, key: &str) -> Result<bool, String> {
        match self.raw(key)? {
            Json::Bool(b) => Ok(*b),
            _ => Err(format!("label '{key}' must be true or false")),
        }
    }

    /// A count setting: a non-negative integer or null.
    fn count(&mut self, key: &str) -> Result<Option<usize>, String> {
        match self.raw(key)? {
            Json::Null => Ok(None),
            Json::Number(n) => n
                .as_u64()
                .map(|n| Some(n as usize))
                .ok_or_else(|| format!("label '{key}' must be a non-negative integer or null")),
            _ => Err(format!("label '{key}' must be a non-negative integer or null")),
        }
    }

    fn text(&mut self, key: &str) -> Result<String, String> {
        match self.raw(key)? {
            Json::String(s) if !s.is_empty() => Ok(s.clone()),
            _ => Err(format!("label '{key}' must be a non-empty string")),
        }
    }

    /// Precedence tiers: a list of lists of lexemes.
    fn tiers(&mut self, key: &str) -> Result<Vec<Vec<String>>, String> {
        let wrong = || format!("label '{key}' must be a list of lists of strings");
        match self.raw(key)? {
            Json::Array(tiers) => tiers
                .iter()
                .map(|tier| match tier {
                    Json::Array(items) => items
                        .iter()
                        .map(|item| match item {
                            Json::String(s) if !s.is_empty() => Ok(s.clone()),
                            _ => Err(wrong()),
                        })
                        .collect(),
                    _ => Err(wrong()),
                })
                .collect(),
            _ => Err(wrong()),
        }
    }

    /// A label the kernel recognises but does not implement: it must be empty.
    fn unsupported(&mut self, key: &str) -> Result<(), String> {
        if self.lexemes(key)?.is_empty() {
            Ok(())
        } else {
            Err(format!("label '{key}' is not implemented by the microcode kernel; leave it empty"))
        }
    }

    fn finish(self) -> Result<(), String> {
        let mut unknown: Vec<&String> =
            self.map.keys().filter(|k| !k.starts_with("$comment") && !self.seen.contains(*k)).collect();
        unknown.sort();
        if unknown.is_empty() {
            Ok(())
        } else {
            let names: Vec<&str> = unknown.iter().map(|s| s.as_str()).collect();
            Err(format!("unknown label(s): {}", names.join(", ")))
        }
    }
}

fn pair(l: &mut Labels, open: &str, close: &str, separator: Option<&str>) -> Result<Option<Pair>, String> {
    let o = l.first(open)?;
    let c = l.first(close)?;
    let s = match separator {
        Some(key) => l.first(key)?,
        None => None,
    };
    match (o, c) {
        (Some(open), Some(close)) => Ok(Some(Pair { open, close, separator: s })),
        (None, None) if s.is_none() => Ok(None),
        _ => Err(format!("labels '{open}' and '{close}' must be given together")),
    }
}

fn build(l: &mut Labels) -> Result<LanguageSchema, String> {
    if l.count("format_version")? != Some(1) {
        return Err("format_version must be 1".to_string());
    }
    let name = l.text("language")?;
    let extensions = l.lexemes("extensions")?;

    // Identifier policy comes first: the word tests below depend on it.
    let identifier_unicode = l.flag("identifier.unicode")?;
    let variable_prefix = l.first_char("identifier.variable_prefix")?;
    let identifiers_case_insensitive = l.flag("identifier.case_insensitive")?;
    let keywords_case_insensitive = l.flag("lexical.keywords_case_insensitive")?;
    let word_start = |c: char| c == '_' || if identifier_unicode { c.is_alphabetic() } else { c.is_ascii_alphabetic() };
    let word_char = |c: char| c == '_' || if identifier_unicode { c.is_alphanumeric() } else { c.is_ascii_alphanumeric() };
    let is_word = |s: &str| {
        let mut chars = s.chars();
        let first = chars.next();
        let starts = match first {
            Some(c) if word_start(c) => true,
            Some(c) if Some(c) == variable_prefix => chars.clone().next().map_or(false, word_start),
            _ => false,
        };
        starts && chars.all(word_char)
    };

    // ---- lexical ----
    let comment_lines = l.lexemes("lexical.comment_line")?;
    let comment_opens = l.lexemes("lexical.comment_block.open")?;
    let comment_closes = l.lexemes("lexical.comment_block.close")?;
    if comment_opens.len() != comment_closes.len() {
        return Err("lexical.comment_block.open and .close must pair up position by position".to_string());
    }
    let comment_blocks: Vec<(String, String)> = comment_opens.into_iter().zip(comment_closes).collect();
    let quotes = l.chars("lexical.string_quotes")?;
    let raw_quotes = l.chars("lexical.raw_quotes")?;
    for q in &raw_quotes {
        if !quotes.contains(q) {
            return Err(format!("lexical.raw_quotes lists '{q}', which is not in lexical.string_quotes"));
        }
    }
    let escape_letters = l.chars("lexical.string_escapes")?;
    let mut escapes = HashMap::new();
    for &quote in &quotes {
        let mut table = HashMap::new();
        table.insert('\\', "\\".to_string());
        table.insert(quote, quote.to_string());
        if !raw_quotes.contains(&quote) {
            for &letter in &escape_letters {
                let replacement = match letter {
                    'n' => "\n".to_string(),
                    't' => "\t".to_string(),
                    'r' => "\r".to_string(),
                    '0' => "\0".to_string(),
                    '\\' => "\\".to_string(),
                    c if quotes.contains(&c) => c.to_string(),
                    other => return Err(format!("lexical.string_escapes: unknown escape letter '{other}'")),
                };
                table.insert(letter, replacement);
            }
        }
        escapes.insert(quote, table);
    }
    let prologue = l.first("lexical.prologue")?;
    let number = NumberSyntax {
        decimal_point: l.first_char("lexical.number.decimal_point")?,
        base_marker: l.first_char("lexical.number.base_marker")?,
        exponent_marker: l.first_char("lexical.number.exponent_marker")?,
        hex_prefix: l.first("lexical.number.hex_prefix")?,
    };
    if let Some(prefix) = &number.hex_prefix {
        if prefix.chars().count() != 2 || !prefix.starts_with(|c: char| c.is_ascii_digit()) {
            return Err(format!("lexical.number.hex_prefix must be a digit followed by one letter, got '{prefix}'"));
        }
    }

    // ---- structure ----
    let style = match l.text("block.style")?.as_str() {
        "indentation" => BlockStyle::Indentation,
        "braces" => BlockStyle::Braces,
        "keyword" => BlockStyle::Keyword,
        other => return Err(format!("block.style must be 'indentation', 'braces' or 'keyword', got '{other}'")),
    };
    let block_open = l.lexemes("block.open")?;
    let block_close = l.lexemes("block.close")?;
    let block_intro = l.lexemes("block.intro")?;
    let indent_size = l.count("block.indent_size")?;
    let (block_open, block_close, indent_size) = match style {
        BlockStyle::Braces => {
            if block_open.is_empty() || block_open.len() != block_close.len() {
                return Err("braces need block.open and block.close, paired position by position".to_string());
            }
            (block_open, block_close, indent_size.unwrap_or(4))
        }
        BlockStyle::Keyword => {
            if !block_open.is_empty() || block_close.is_empty() {
                return Err("keyword blocks take no block.open and need block.close".to_string());
            }
            (block_open, block_close, indent_size.unwrap_or(4))
        }
        BlockStyle::Indentation => {
            if !block_open.is_empty() || !block_close.is_empty() {
                return Err("indentation blocks take no block.open or block.close".to_string());
            }
            let size = indent_size.ok_or("indentation blocks need block.indent_size")?;
            if size == 0 {
                return Err("block.indent_size must be at least 1".to_string());
            }
            (vec!["<block>".to_string()], vec!["</block>".to_string()], size)
        }
    };
    let terminators = l.lexemes("stmt.terminator")?;
    let group = pair(l, "syntax.group.open", "syntax.group.close", None)?;
    let call = pair(l, "syntax.call.open", "syntax.call.close", Some("syntax.call.separator"))?;
    let array = pair(l, "syntax.array.open", "syntax.array.close", Some("syntax.array.separator"))?;
    for key in ["syntax.map.open", "syntax.map.separator", "syntax.map.pair", "syntax.map.close"] {
        l.unsupported(key)?;
    }
    let index = pair(l, "op.index.open", "op.index.close", None)?;

    // ---- literals ----
    let literals = Literals {
        true_words: l.lexemes("literal.true")?,
        false_words: l.lexemes("literal.false")?,
        null_words: l.lexemes("literal.null")?,
    };

    // ---- operators ----
    let tiers = l.tiers("op.precedence")?;
    let right_assoc = l.lexemes("op.right_associative")?;
    let binary_labels = [
        ("op.add", Op::Add),
        ("op.sub", Op::Sub),
        ("op.mul", Op::Mul),
        ("op.div", Op::Div),
        ("op.quot", Op::Quot),
        ("op.rem", Op::Rem),
        ("op.pow", Op::Pow),
        ("op.eq", Op::Eq),
        ("op.ne", Op::Ne),
        ("op.lt", Op::Lt),
        ("op.le", Op::Le),
        ("op.gt", Op::Gt),
        ("op.ge", Op::Ge),
        ("op.and", Op::And),
        ("op.or", Op::Or),
        ("op.concat", Op::Concat),
        ("op.range", Op::Range),
        ("op.pipe", Op::Pipe),
    ];
    let unary_labels = [("op.not", Op::Not), ("op.negate", Op::Negate)];
    let tier_first = |lex: &str| tiers.iter().position(|t| t.iter().any(|x| x == lex));
    let tier_last = |lex: &str| tiers.iter().rposition(|t| t.iter().any(|x| x == lex));
    let mut binary: HashMap<String, BinaryOp> = HashMap::new();
    let mut unary: HashMap<String, UnaryOp> = HashMap::new();
    for (label, op) in binary_labels {
        for lex in l.lexemes(label)? {
            let tier = tier_first(&lex).ok_or_else(|| format!("'{lex}' ({label}) does not appear in op.precedence"))?;
            let associativity = if right_assoc.iter().any(|r| *r == lex) { Assoc::Right } else { Assoc::Left };
            if binary.insert(lex.clone(), BinaryOp { precedence: (tier + 1) as f32, associativity, op }).is_some() {
                return Err(format!("'{lex}' is listed under two binary operator labels"));
            }
        }
    }
    for (label, op) in unary_labels {
        for lex in l.lexemes(label)? {
            let tier = tier_last(&lex).ok_or_else(|| format!("'{lex}' ({label}) does not appear in op.precedence"))?;
            if unary.insert(lex.clone(), UnaryOp { precedence: (tier + 1) as f32, op }).is_some() {
                return Err(format!("'{lex}' is listed under two unary operator labels"));
            }
        }
    }
    for lex in tiers.iter().flatten() {
        if !binary.contains_key(lex) && !unary.contains_key(lex) {
            return Err(format!("op.precedence lists '{lex}', which is under no operator label"));
        }
    }
    for lex in &right_assoc {
        if !binary.contains_key(lex) {
            return Err(format!("op.right_associative lists '{lex}', which is not a binary operator"));
        }
    }

    // ---- statements ----
    let assignment = l.lexemes("stmt.assign")?;
    let let_keyword = l.lexemes("stmt.let")?;
    let let_mutable = l.lexemes("stmt.let.mutable")?;
    let let_annotation = l.lexemes("stmt.let.annotation")?;
    let binding = if let_keyword.is_empty() {
        if !let_mutable.is_empty() || !let_annotation.is_empty() {
            return Err("stmt.let.mutable and stmt.let.annotation need stmt.let".to_string());
        }
        None
    } else {
        Some(Binding { keyword: let_keyword, mutable_modifier: let_mutable, type_annotation: let_annotation })
    };
    let if_keyword = l.lexemes("stmt.if")?;
    let elif_keyword = l.lexemes("stmt.elif")?;
    let else_keyword = l.lexemes("stmt.else")?;
    let branch = if if_keyword.is_empty() {
        if !elif_keyword.is_empty() || !else_keyword.is_empty() {
            return Err("stmt.elif and stmt.else need stmt.if".to_string());
        }
        None
    } else {
        Some(Branch { keyword: if_keyword, else_keyword, elif_keyword })
    };
    let loop_while = l.lexemes("stmt.while")?;
    let loop_until = l.lexemes("stmt.until")?;
    let for_keyword = l.lexemes("stmt.for")?;
    let in_keyword = l.lexemes("stmt.for.in")?;
    let loop_for = match (for_keyword.is_empty(), in_keyword.is_empty()) {
        (true, true) => None,
        (false, false) => Some(ForLoop { keyword: for_keyword, in_keyword }),
        _ => return Err("stmt.for and stmt.for.in must be given together".to_string()),
    };
    for key in ["stmt.foreach", "stmt.foreach.as", "stmt.foreach.pair"] {
        l.unsupported(key)?;
    }
    let statements = Statements {
        assignment,
        binding,
        branch,
        loop_while,
        loop_until,
        loop_for,
        return_: l.lexemes("stmt.return")?,
        break_: l.lexemes("stmt.break")?,
        continue_: l.lexemes("stmt.continue")?,
        function: l.lexemes("stmt.function")?,
        function_returns: l.lexemes("stmt.function.returns")?,
        pass: l.lexemes("stmt.pass")?,
    };
    l.unsupported("stmt.emit")?;

    // ---- functions ----
    let builtin_labels = [
        ("builtin.emit", Builtin::Emit),
        ("builtin.print", Builtin::PrintLine),
        ("builtin.write", Builtin::Write),
        ("builtin.len", Builtin::Len),
        ("builtin.char_at", Builtin::CharAt),
        ("builtin.ord", Builtin::Ord),
        ("builtin.chr", Builtin::Chr),
        ("builtin.typeof", Builtin::Kind),
        ("builtin.error", Builtin::Error),
        ("builtin.extern", Builtin::Extern),
        ("builtin.range", Builtin::Range),
        ("builtin.real", Builtin::Real),
        ("builtin.int_to_string", Builtin::IntToString),
        ("builtin.real_to_string", Builtin::RealToString),
        ("builtin.rational_to_string", Builtin::RationalToString),
        ("builtin.bool_to_string", Builtin::BoolToString),
        ("builtin.array_to_string", Builtin::ArrayToString),
        ("builtin.null_to_string", Builtin::NullToString),
        ("builtin.kind_to_string", Builtin::KindToString),
        ("builtin.num", Builtin::Num),
        ("builtin.den", Builtin::Den),
        ("builtin.int", Builtin::Int),
        ("builtin.frac", Builtin::Frac),
        ("builtin.push", Builtin::Push),
    ];
    let mut functions = HashMap::new();
    for (label, builtin) in builtin_labels {
        for lex in l.lexemes(label)? {
            if functions.insert(lex.clone(), builtin).is_some() {
                return Err(format!("'{lex}' is listed under two builtin labels"));
            }
        }
    }

    // ---- system ----
    let kind_labels = [
        ("system.kind.integer", KindName::Integer),
        ("system.kind.rational", KindName::Rational),
        ("system.kind.real", KindName::Real),
        ("system.kind.string", KindName::String),
        ("system.kind.boolean", KindName::Boolean),
        ("system.kind.array", KindName::Array),
        ("system.kind.null", KindName::Null),
    ];
    let mut kinds = HashMap::new();
    for (label, kind) in kind_labels {
        if let Some(name) = l.first(label)? {
            kinds.insert(name, kind);
        }
    }
    let system = System {
        args: l.first("system.args")?,
        memoization: l.first("system.memoization")?,
        kinds,
        real_default_precision: l.first("system.real_default_precision")?,
        entry: l.first("system.entry")?,
    };

    // ---- derived tables and shape checks ----
    let mut symbols: Vec<String> = Vec::new();
    let mut reserved: HashSet<String> = HashSet::new();
    let mut classify = |lex: &str| {
        if is_word(lex) {
            reserved.insert(lex.to_string());
        } else if !symbols.iter().any(|s| s == lex) {
            symbols.push(lex.to_string());
        }
    };
    for lex in binary.keys().chain(unary.keys()) {
        classify(lex);
    }
    for p in [&group, &call, &array, &index].into_iter().flatten() {
        classify(&p.open);
        classify(&p.close);
        if let Some(sep) = &p.separator {
            classify(sep);
        }
    }
    if style != BlockStyle::Indentation {
        for lex in block_open.iter().chain(block_close.iter()) {
            classify(lex);
        }
    }
    for intro in &block_intro {
        classify(intro);
    }
    for lex in statements.assignment.iter().chain(terminators.iter()) {
        classify(lex);
    }
    if let Some(b) = &statements.binding {
        for lex in &b.type_annotation {
            classify(lex);
        }
    }
    for lex in &statements.function_returns {
        classify(lex);
    }
    let mut keywords: Vec<&String> = Vec::new();
    if let Some(b) = &statements.binding {
        keywords.extend(b.keyword.iter().chain(b.mutable_modifier.iter()));
    }
    if let Some(b) = &statements.branch {
        keywords.extend(b.keyword.iter().chain(b.else_keyword.iter()).chain(b.elif_keyword.iter()));
    }
    if let Some(f) = &statements.loop_for {
        keywords.extend(f.keyword.iter().chain(f.in_keyword.iter()));
    }
    keywords.extend(
        statements
            .loop_while
            .iter()
            .chain(statements.loop_until.iter())
            .chain(statements.return_.iter())
            .chain(statements.break_.iter())
            .chain(statements.continue_.iter())
            .chain(statements.function.iter())
            .chain(statements.pass.iter())
            .chain(literals.true_words.iter())
            .chain(literals.false_words.iter())
            .chain(literals.null_words.iter()),
    );
    for word in keywords {
        if !is_word(word) {
            return Err(format!("keyword '{word}' must be shaped like an identifier"));
        }
        reserved.insert(word.clone());
    }
    for name in functions.keys() {
        let base = name.trim_end_matches(|c: char| !word_char(c));
        let suffix = &name[base.len()..];
        let suffix_ok = suffix.is_empty() || (suffix.chars().count() == 1 && symbols.iter().any(|s| s == suffix));
        if !is_word(base) || !suffix_ok {
            return Err(format!(
                "builtin name '{name}' must be an identifier, optionally ending in one operator character"
            ));
        }
    }
    for name in [&system.args, &system.memoization, &system.real_default_precision, &system.entry]
        .into_iter()
        .flatten()
        .chain(system.kinds.keys())
    {
        if !is_word(name) {
            return Err(format!("system name '{name}' must be shaped like an identifier"));
        }
    }
    if keywords_case_insensitive {
        reserved = reserved.into_iter().map(|w| w.to_lowercase()).collect();
    }

    let mut prefix_chars = name.chars();
    let error_prefix = match prefix_chars.next() {
        Some(first) => format!("{}{}Error", first.to_uppercase(), prefix_chars.as_str()),
        None => "Error".to_string(),
    };

    Ok(LanguageSchema {
        name,
        extensions,
        error_prefix,
        lexical: Lexical {
            comment_lines,
            comment_blocks,
            quotes,
            escapes,
            prologue,
            operators: symbols,
            number,
            identifier_unicode,
            variable_prefix,
            keywords_case_insensitive,
            identifiers_case_insensitive,
            reserved_words: reserved,
        },
        structure: Structure {
            blocks: style,
            indent_size,
            block_open,
            block_close,
            block_intro,
            terminators,
            group,
            call,
            array,
            index,
        },
        literals,
        operators: Operators { binary, unary },
        statements,
        functions,
        system,
    })
}
