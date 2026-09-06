// Language schema: the data that turns the kernel into a language.
//
// A schema is deserialised from YAML and contains only tables: lexemes,
// block rules, operator precedence with the kernel operation each maps to,
// statement keywords with the form each introduces, the surface names of
// built-in functions, and the names of system bindings. It contains no code.

use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct LanguageSchema {
    pub name: String,
    /// Prefix used when reporting errors, e.g. `LumenError`.
    #[serde(default = "default_error_prefix")]
    pub error_prefix: String,
    pub lexical: Lexical,
    pub structure: Structure,
    #[serde(default)]
    pub literals: Literals,
    pub operators: Operators,
    #[serde(default)]
    pub statements: Statements,
    /// Surface function name → kernel built-in.
    #[serde(default)]
    pub functions: HashMap<String, Builtin>,
    #[serde(default)]
    pub system: System,
}

fn default_error_prefix() -> String {
    "Error".to_string()
}

// ---------------- Stage 1: lexical tables ----------------

#[derive(Debug, Clone, Deserialize)]
pub struct Lexical {
    /// Line-comment marker; comments are removed before lexing.
    #[serde(default)]
    pub comment: Option<String>,
    /// String delimiters.
    #[serde(default)]
    pub quotes: Vec<char>,
    /// Escape tables per delimiter: escape letter → replacement text.
    /// A backslash followed by an unlisted letter is kept verbatim.
    #[serde(default)]
    pub escapes: HashMap<char, HashMap<char, String>>,
    /// Operator and punctuation lexemes; longest match wins.
    pub operators: Vec<String>,
    /// Whether identifiers may use letters and digits beyond ASCII.
    #[serde(default)]
    pub identifier_unicode: bool,
    #[serde(default)]
    pub number: NumberSyntax,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NumberSyntax {
    #[serde(default)]
    pub decimal_point: Option<char>,
    /// `<base>@<digits>` literals, e.g. `16@FF`.
    #[serde(default)]
    pub base_marker: Option<char>,
    /// Exponent marker inside base-N literals, e.g. `10@1.5^3`.
    #[serde(default)]
    pub exponent_marker: Option<char>,
}

// ---------------- Stage 2: structure tables ----------------

#[derive(Debug, Clone, Deserialize)]
pub struct Structure {
    pub blocks: BlockStyle,
    #[serde(default = "four")]
    pub indent_size: usize,
    /// Block delimiters: present in the source for `braces`, synthesised
    /// from indentation changes for `indentation`.
    pub block_open: String,
    pub block_close: String,
    /// Token that ends a block header (Python's `:`); dropped when present.
    #[serde(default)]
    pub block_intro: Option<String>,
    /// Statement terminators besides line ends, e.g. `;`.
    #[serde(default)]
    pub terminators: Vec<String>,
    #[serde(default)]
    pub group: Option<Pair>,
    #[serde(default)]
    pub call: Option<Pair>,
    #[serde(default)]
    pub array: Option<Pair>,
}

fn four() -> usize {
    4
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BlockStyle {
    Indentation,
    Braces,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Pair {
    pub open: String,
    pub close: String,
    #[serde(default)]
    pub separator: Option<String>,
}

// ---------------- Stage 3: reduce tables ----------------

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Literals {
    #[serde(rename = "true", default)]
    pub true_words: Vec<String>,
    #[serde(rename = "false", default)]
    pub false_words: Vec<String>,
    #[serde(rename = "null", default)]
    pub null_words: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Operators {
    #[serde(default)]
    pub binary: HashMap<String, BinaryOp>,
    #[serde(default)]
    pub unary: HashMap<String, UnaryOp>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BinaryOp {
    pub precedence: f32,
    #[serde(default)]
    pub associativity: Assoc,
    pub op: Op,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UnaryOp {
    pub precedence: f32,
    pub op: Op,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Assoc {
    #[default]
    Left,
    Right,
}

/// Kernel operations. Schemas map surface operators onto these; the last
/// group is used internally when statements are desugared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
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

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Statements {
    /// Assignment operator, e.g. `=`.
    #[serde(default)]
    pub assignment: Option<String>,
    #[serde(default)]
    pub binding: Option<Binding>,
    #[serde(default)]
    pub branch: Option<Branch>,
    #[serde(default)]
    pub loop_while: Option<Keyword>,
    #[serde(default)]
    pub loop_until: Option<Keyword>,
    #[serde(default)]
    pub loop_for: Option<ForLoop>,
    #[serde(rename = "return", default)]
    pub return_: Option<Keyword>,
    #[serde(rename = "break", default)]
    pub break_: Option<Keyword>,
    #[serde(rename = "continue", default)]
    pub continue_: Option<Keyword>,
    #[serde(default)]
    pub function: Option<Keyword>,
    /// A no-op statement keyword, e.g. Python's `pass`.
    #[serde(default)]
    pub pass: Option<Keyword>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Keyword {
    pub keyword: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Binding {
    pub keyword: String,
    #[serde(default)]
    pub mutable_modifier: Option<String>,
    /// Token introducing an (ignored) type annotation, e.g. `:`.
    #[serde(default)]
    pub type_annotation: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Branch {
    pub keyword: String,
    pub else_keyword: String,
    #[serde(default)]
    pub elif_keyword: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ForLoop {
    pub keyword: String,
    pub in_keyword: String,
}

// ---------------- Stage 4: execute tables ----------------

/// Built-in functions the kernel can provide. Which surface names reach
/// them, if any, is the schema's choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
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
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct System {
    /// Read-only binding holding the program arguments as one string.
    #[serde(default)]
    pub args: Option<String>,
    /// Binding that switches function-result caching on and off.
    #[serde(default)]
    pub memoization: Option<String>,
    /// Bindings for kind meta-values.
    #[serde(default)]
    pub kinds: HashMap<String, KindName>,
    /// Integer constants bound at start-up.
    #[serde(default)]
    pub integer_constants: HashMap<String, i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
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
    pub fn from_yaml(text: &str) -> Result<Self, String> {
        serde_yaml::from_str(text).map_err(|e| format!("schema error: {e}"))
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
