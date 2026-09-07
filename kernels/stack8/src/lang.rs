// A language definition read from JSON into the tables the assembler and
// the machine consult. Every tag must be present and of the right shape;
// keys beginning with `$` are notes and are skipped.

use std::collections::{HashMap, HashSet};

use serde_json::Value as Json;

use crate::value::Sort;
use crate::code::{Builtin, Action};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Blocks {
    Indented,
    Braced,
    Worded,
}

#[derive(Clone, Debug)]
pub struct Brackets {
    pub open: String,
    pub close: String,
    pub between: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Operator {
    pub action: Action,
    pub level: u32,
    pub right_assoc: bool,
}

pub struct Lang {
    pub ident: String,
    pub extensions: Vec<String>,
    pub banner: String,

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
    pub unicode_names: bool,
    pub sigil: Option<char>,
    pub keywords_folded: bool,
    pub names_folded: bool,
    pub quote_for_names: Option<char>,
    pub symbols: Vec<String>,
    pub keywords: HashSet<String>,

    pub blocks: Blocks,
    /// Reverse Polish: words acting on one stack, no expressions.
    pub rpn: bool,
    pub indent_width: usize,
    pub block_opens: Vec<String>,
    pub block_closes: Vec<String>,
    pub block_intros: Vec<String>,
    pub stmt_ends: Vec<String>,
    pub grouping: Option<Brackets>,
    pub calling: Option<Brackets>,
    pub argument_labels: Vec<String>,
    pub array_brackets: Option<Brackets>,
    pub map_brackets: Option<Brackets>,
    /// `k => v` inside a literal, and between the two names of a foreach.
    pub pair_mark: Option<String>,
    pub index_brackets: Option<Brackets>,
    pub text_indexable: bool,

    pub true_words: Vec<String>,
    pub false_words: Vec<String>,
    pub null_words: Vec<String>,
    /// Nothing is shown as no text at all, rather than as the word a
    /// program writes for it: how PHP shows it.
    pub null_silent: bool,
    pub dyadic: HashMap<String, Operator>,
    pub monadic: HashMap<String, Operator>,
    pub pipe_words: Vec<String>,
    pub range_marks: Vec<String>,
    pub precedence: HashMap<String, u32>,

    pub assign_words: Vec<String>,
    pub let_words: Vec<String>,
    pub mutable_words: Vec<String>,
    pub type_marks: Vec<String>,
    pub types_first: bool,
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
    pub return_marks: Vec<String>,
    pub named_result: bool,
    pub pass_words: Vec<String>,

    pub dup_words: Vec<String>,
    pub drop_words: Vec<String>,
    pub swap_words: Vec<String>,
    pub over_words: Vec<String>,
    pub rot_words: Vec<String>,
    pub eval_words: Vec<String>,
    pub quote_open: Vec<String>,
    pub quote_close: Vec<String>,

    pub builtins: HashMap<String, Builtin>,
    pub holes: Vec<String>,
    pub args_binding: Option<String>,
    pub memo_binding: Option<String>,
    pub precision_binding: Option<String>,
    pub entry_binding: Option<String>,
    pub sort_bindings: Vec<(String, Sort)>,

    /// The ext.* labels: extensions of the core, read by the full
    /// kernels and ignored by the reference ones; absent means none.
    pub epilogue: Vec<String>,
    pub bare_calls: bool,
    pub increments: Vec<String>,
    pub decrements: Vec<String>,
    pub interpolating: Vec<char>,
    pub concat: Option<String>,
    pub foreach_words: Vec<String>,
    pub foreach_as_words: Vec<String>,
    pub c_for_words: Vec<String>,
    /// `x op= e` for every binary operator, when the switch is on.
    pub compound: HashMap<String, Action>,
    pub static_words: Vec<String>,
    pub global_words: Vec<String>,
    pub const_words: Vec<String>,
    pub switch_words: Vec<String>,
    pub case_words: Vec<String>,
    pub default_words: Vec<String>,
    pub case_marks: Vec<String>,
    /// The two signs of `test ? a : b`.
    pub ternary: Option<(String, String)>,
    /// A lone statement may stand where a block is expected.
    pub lone_stmt: bool,
    /// A top-level function is bound before anything else runs.
    pub hoisted: bool,
    /// Letters that open a decimal exponent in a number (1e9).
    pub exponent_letters: Vec<char>,
    /// A sign that leaves its operand as it is.
    pub plus_words: Vec<String>,
    /// `break n` and `continue n` leave n loops.
    pub break_levels: bool,
    /// `a[] = v` appends.
    pub append_index: bool,
    /// `for v in a` walks what a holds when a is not a range.
    pub for_collections: bool,

    /// Classes and their objects.
    pub class_words: Vec<String>,
    pub extends_words: Vec<String>,
    pub new_words: Vec<String>,
    /// The name a method knows its own object by.
    pub this_word: Option<String>,
    /// The method run when an object is made.
    pub constructor: Option<String>,
    /// Words that may stand before a member and say nothing this kernel reads.
    pub modifier_words: Vec<String>,
    /// The modifier marking a member the class keeps for itself.
    pub shared_words: Vec<String>,
    /// `object->member`, and `class::member`.
    pub member_mark: Option<String>,
    pub scope_mark: Option<String>,
    pub instanceof_words: Vec<String>,
    pub parent_words: Vec<String>,
    pub self_words: Vec<String>,
    /// Signs a name may be led by, which say nothing: PHP's `\Error`.
    pub name_leads: Vec<char>,
    pub try_words: Vec<String>,
    pub catch_words: Vec<String>,
    pub finally_words: Vec<String>,
    pub throw_words: Vec<String>,
    /// Between the classes one catch takes.
    pub catch_between: Option<String>,
    /// The sign that makes one name stand for another's cell.
    pub reference_mark: Option<String>,
    /// Reading a place an array does not hold gives nothing, rather
    /// than stopping the program.
    pub absent_index: bool,
    /// What the language calls the parts of a web request: the name for
    /// each group the host gathers, and one for all of them together.
    pub request_bindings: Vec<(String, String)>,
}

/// Every tag with its shape: w a word list, s a string, b a switch,
/// n a size or null, o a string or null, t precedence tiers, x a word
/// list this kernel gives no meaning and passes over, as it passes over
/// an ext.* label it does not read.
const LABELS: &str = "
n format_version | s language | w extensions | w lexical.comment_line
w lexical.comment_block.open | w lexical.comment_block.close | w lexical.string_quotes | w lexical.raw_quotes
w lexical.string_escapes | w lexical.prologue | w lexical.name_quote | w lexical.number.decimal_point
w lexical.number.base_marker | w lexical.number.exponent_marker | w lexical.number.hex_prefix | b lexical.keywords_case_insensitive
b identifier.unicode | w identifier.variable_prefix | b identifier.case_insensitive | s block.style
w block.open | w block.close | w block.intro | n block.indent_size
w stmt.terminator | s syntax.notation | w syntax.group.open | w syntax.group.close
w syntax.call.open | w syntax.call.separator | w syntax.call.close | w syntax.call.label
w syntax.array.open | w syntax.array.separator | w syntax.array.close | w syntax.map.open
w syntax.map.separator | w syntax.map.pair | w syntax.map.close | w literal.true
w literal.false | w literal.null | b literal.null.silent | t op.precedence | w op.right_associative
w op.add | w op.sub | w op.mul | w op.div
o op.div.result | w op.quot | w op.rem | w op.pow
w op.eq | w op.ne | w op.lt | w op.le
w op.gt | w op.ge | w op.and | w op.or
w op.not | w op.negate | w op.concat | w op.range
w op.index.open | w op.index.close | b op.index.strings | w op.pipe
w stmt.assign | w stmt.let | w stmt.let.mutable | w stmt.let.annotation
b stmt.let.type_first | w stmt.if | w stmt.elif | w stmt.else
w stmt.while | w stmt.until | w stmt.for | w stmt.for.in
w stmt.foreach | w stmt.foreach.as | w stmt.foreach.pair | w stmt.return
w stmt.break | w stmt.continue | w stmt.function | w stmt.function.returns
b stmt.function.result_by_name | w stmt.pass | x stmt.emit | w stack.dup
w stack.drop | w stack.swap | w stack.over | w stack.rot
w stack.eval | w stack.program.open | w stack.program.close | w builtin.emit
w builtin.print | w builtin.write | w builtin.print.placeholder | w builtin.len
w builtin.char_at | w builtin.ord | w builtin.chr | w builtin.typeof
w builtin.error | w builtin.extern | w builtin.range | w builtin.real
w builtin.num | w builtin.den | w builtin.push | w builtin.get
w builtin.put | w builtin.precision | w builtin.to_string | w builtin.to_int
w builtin.to_real | w system.args | w system.memoization | w system.real_default_precision
w system.entry | w system.kind.integer | w system.kind.rational | w system.kind.real
w system.kind.string | w system.kind.boolean | w system.kind.array | w system.kind.null
";

/// The extension labels a definition may add beyond the core; a
/// missing one reads as empty (or off).
const EXT_LABELS: &str = "
w ext.lexical.epilogue | w ext.builtin.echo | b ext.syntax.call.bare | w ext.op.increment
w ext.op.decrement | w ext.lexical.interpolating_quotes | w ext.stmt.for.c | b ext.op.assign.compound
w ext.stmt.static | w ext.stmt.global | w ext.stmt.const | w ext.builtin.define
w ext.builtin.var_dump | w ext.stmt.switch | w ext.stmt.case | w ext.stmt.default
w ext.stmt.case.mark | w ext.op.ternary | b ext.block.lone_statement | b ext.stmt.function.hoisted
w ext.lexical.number.exponent | w ext.op.plus | b ext.stmt.break.levels
w ext.builtin.array | b ext.op.index.append | b ext.stmt.for.collection | w ext.builtin.print_r
w ext.stmt.function.returns | w ext.stmt.class | w ext.stmt.class.extends | w ext.stmt.class.new
w ext.stmt.class.this | w ext.stmt.class.constructor | w ext.stmt.class.modifier | w ext.stmt.class.shared
w ext.op.member | w ext.op.scope | w ext.op.instanceof | w ext.stmt.class.parent
w ext.stmt.class.self | w ext.lexical.name_lead | w ext.stmt.try | w ext.stmt.catch
w ext.stmt.finally | w ext.stmt.throw | w ext.stmt.catch.separator | w ext.op.reference
w ext.system.request.query | w ext.system.request.form | w ext.system.request.cookies | w ext.system.request.server
w ext.system.request.env | w ext.system.request.files | w ext.system.request.all | b ext.op.index.absent
";

fn shapes_of(table: &'static str) -> Vec<(char, &'static str)> {
    table
        .split(['\n', '|'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|entry| (entry.chars().next().unwrap(), entry[2..].trim()))
        .collect()
}

fn number_shapes() -> Vec<(char, &'static str)> {
    shapes_of(LABELS)
}

static ABSENT_LIST: Json = Json::Array(Vec::new());
static ABSENT_SWITCH: Json = Json::Bool(false);

struct Reader<'a>(&'a serde_json::Map<String, Json>);

impl<'a> Reader<'a> {
    fn field(&self, key: &str) -> Result<&'a Json, String> {
        if let Some(value) = self.0.get(key) {
            return Ok(value);
        }
        match shapes_of(EXT_LABELS).iter().find(|(_, tag)| *tag == key) {
            Some(('b', _)) => Ok(&ABSENT_SWITCH),
            Some(_) => Ok(&ABSENT_LIST),
            None => Err(format!("missing label '{key}'")),
        }
    }

    fn strings(&self, key: &str) -> Result<Vec<String>, String> {
        let Json::Array(items) = self.field(key)? else {
            return Err(format!("label '{key}' must be a list of non-empty strings"));
        };
        items
            .iter()
            .map(|item| match item {
                Json::String(s) if !s.is_empty() => Ok(s.clone()),
                _ => Err(format!("label '{key}' must be a list of non-empty strings")),
            })
            .collect()
    }

    fn head(&self, key: &str) -> Result<Option<String>, String> {
        Ok(self.strings(key)?.into_iter().next())
    }

    fn letters(&self, key: &str) -> Result<Vec<char>, String> {
        self.strings(key)?
            .into_iter()
            .map(|w| {
                let mut it = w.chars();
                match (it.next(), it.next()) {
                    (Some(c), None) => Ok(c),
                    _ => Err(format!("label '{key}' takes single characters, got '{w}'")),
                }
            })
            .collect()
    }

    fn letter(&self, key: &str) -> Result<Option<char>, String> {
        Ok(self.letters(key)?.into_iter().next())
    }

    fn flag(&self, key: &str) -> Result<bool, String> {
        match self.field(key)? {
            Json::Bool(b) => Ok(*b),
            _ => Err(format!("label '{key}' must be true or false")),
        }
    }

    fn string(&self, key: &str) -> Result<String, String> {
        match self.field(key)? {
            Json::String(s) if !s.is_empty() => Ok(s.clone()),
            _ => Err(format!("label '{key}' must be a non-empty string")),
        }
    }

    fn string_or_null(&self, key: &str) -> Result<Option<String>, String> {
        match self.field(key)? {
            Json::Null => Ok(None),
            Json::String(s) if !s.is_empty() => Ok(Some(s.clone())),
            _ => Err(format!("label '{key}' must be a non-empty string or null")),
        }
    }

    fn count(&self, key: &str) -> Result<Option<usize>, String> {
        match self.field(key)? {
            Json::Null => Ok(None),
            Json::Number(n) if n.as_u64().is_some() => Ok(n.as_u64().map(|n| n as usize)),
            _ => Err(format!("label '{key}' must be a non-negative integer or null")),
        }
    }

    fn levels(&self, key: &str) -> Result<Vec<Vec<String>>, String> {
        let wrong = || format!("label '{key}' must be a list of lists of strings");
        let Json::Array(tiers) = self.field(key)? else { return Err(wrong()) };
        tiers
            .iter()
            .map(|tier| {
                let Json::Array(items) = tier else { return Err(wrong()) };
                items
                    .iter()
                    .map(|item| match item {
                        Json::String(s) if !s.is_empty() => Ok(s.clone()),
                        _ => Err(wrong()),
                    })
                    .collect()
            })
            .collect()
    }

    fn brackets(&self, open: &str, close: &str, sep: Option<&str>) -> Result<Option<Brackets>, String> {
        let sep = match sep {
            Some(key) => self.head(key)?,
            None => None,
        };
        match (self.head(open)?, self.head(close)?) {
            (Some(open), Some(close)) => Ok(Some(Brackets { open, close, between: sep })),
            (None, None) if sep.is_none() => Ok(None),
            _ => Err(format!("labels '{open}' and '{close}' must be given together")),
        }
    }
}

fn top_object(text: &str) -> Result<serde_json::Map<String, Json>, String> {
    match serde_json::from_str(text).map_err(|e| format!("definition is not valid JSON: {e}"))? {
        Json::Object(map) => Ok(map),
        _ => Err("definition must be a JSON object".to_string()),
    }
}

/// The language name and extensions a definition declares.
pub fn identify(text: &str) -> Result<(String, Vec<String>), String> {
    let map = top_object(text)?;
    let r = Reader(&map);
    Ok((r.string("language")?, r.strings("extensions")?))
}

fn name_like(s: &str, unicode: bool, prefix: Option<char>) -> bool {
    let mut chars = s.chars().peekable();
    if prefix.is_some() && chars.peek().copied() == prefix {
        chars.next();
    }
    let head = |c: char| c == '_' || if unicode { c.is_alphabetic() } else { c.is_ascii_alphabetic() };
    let tail = |c: char| c == '_' || if unicode { c.is_alphanumeric() } else { c.is_ascii_alphanumeric() };
    match chars.next() {
        Some(c) if head(c) => chars.all(tail),
        _ => false,
    }
}

impl Lang {
    pub fn parse(text: &str) -> Result<Lang, String> {
        let map = top_object(text)?;
        let shapes = number_shapes();
        for (_, tag) in &shapes {
            if !map.contains_key(*tag) {
                return Err(format!("missing label '{tag}'"));
            }
        }
        let extensions = shapes_of(EXT_LABELS);
        let mut strange: Vec<&str> = map
            .keys()
            .filter(|k| !k.starts_with('$') && !shapes.iter().chain(&extensions).any(|(_, l)| l == k))
            .map(String::as_str)
            .collect();
        strange.sort();
        if !strange.is_empty() {
            return Err(format!("unknown label(s): {}", strange.join(", ")));
        }
        let r = Reader(&map);
        if r.count("format_version")? != Some(1) {
            return Err("format_version must be 1".to_string());
        }
        for (shape, tag) in &shapes {
            // A label this kernel gives no meaning to is read past, not refused.
            let _ = (shape, tag);
        }

        let name = r.string("language")?;
        let unicode = r.flag("identifier.unicode")?;
        let var_prefix = r.letter("identifier.variable_prefix")?;

        let comment_opens = r.strings("lexical.comment_block.open")?;
        let comment_closes = r.strings("lexical.comment_block.close")?;
        if comment_opens.len() != comment_closes.len() {
            return Err("lexical.comment_block.open and .close must pair up position by position".to_string());
        }
        let quotes = r.letters("lexical.string_quotes")?;
        let raw_quotes = r.letters("lexical.raw_quotes")?;
        if let Some(q) = raw_quotes.iter().find(|q| !quotes.contains(q)) {
            return Err(format!("lexical.raw_quotes lists '{q}', which is not in lexical.string_quotes"));
        }
        let escapes = r.letters("lexical.string_escapes")?;
        if let Some(e) = escapes.iter().find(|e| !matches!(e, 'n' | 't' | 'r' | '0' | '\\') && !quotes.contains(e)) {
            return Err(format!("lexical.string_escapes: unknown escape letter '{e}'"));
        }
        let hex_prefix = r.head("lexical.number.hex_prefix")?;
        if let Some(p) = &hex_prefix {
            if p.chars().count() != 2 || !p.starts_with(|c: char| c.is_ascii_digit()) {
                return Err(format!("lexical.number.hex_prefix must be a digit followed by one letter, got '{p}'"));
            }
        }

        let style = match r.string("block.style")?.as_str() {
            "indentation" => Blocks::Indented,
            "braces" => Blocks::Braced,
            "keyword" => Blocks::Worded,
            other => return Err(format!("block.style must be 'indentation', 'braces' or 'keyword', got '{other}'")),
        };
        let postfix = match r.string("syntax.notation")?.as_str() {
            "infix" => false,
            "postfix" => true,
            other => return Err(format!("syntax.notation must be 'infix' or 'postfix', got '{other}'")),
        };
        let openers = r.strings("block.open")?;
        let closers = r.strings("block.close")?;
        let size = r.count("block.indent_size")?;
        let indent = match style {
            Blocks::Indented => {
                if !openers.is_empty() || !closers.is_empty() {
                    return Err("indentation blocks take no block.open or block.close".to_string());
                }
                match size {
                    Some(0) => return Err("block.indent_size must be at least 1".to_string()),
                    Some(n) => n,
                    None => return Err("indentation blocks need block.indent_size".to_string()),
                }
            }
            Blocks::Braced => {
                if openers.is_empty() || openers.len() != closers.len() {
                    return Err("braces need block.open and block.close, paired position by position".to_string());
                }
                size.unwrap_or(4)
            }
            Blocks::Worded => {
                if !openers.is_empty() || closers.is_empty() {
                    return Err("keyword blocks take no block.open and need block.close".to_string());
                }
                size.unwrap_or(4)
            }
        };
        let call = r.brackets("syntax.call.open", "syntax.call.close", Some("syntax.call.separator"))?;
        let call_labels = r.strings("syntax.call.label")?;
        if !call_labels.is_empty() && call.is_none() {
            return Err("syntax.call.label needs syntax.call.open".to_string());
        }
        let index = r.brackets("op.index.open", "op.index.close", None)?;
        let index_text = r.flag("op.index.strings")?;
        if index_text && index.is_none() {
            return Err("op.index.strings needs op.index.open".to_string());
        }

        let tiers = r.levels("op.precedence")?;
        if postfix && !tiers.is_empty() {
            return Err("a postfix language takes no op.precedence".to_string());
        }
        let rights = r.strings("op.right_associative")?;
        let div = match r.string_or_null("op.div.result")?.as_deref() {
            None | Some("rational") => Action::Div,
            Some("real") => Action::DivReal,
            Some(other) => return Err(format!("op.div.result must be 'rational', 'real' or null, got '{other}'")),
        };
        let tier_of = |lex: &str, from_top: bool| -> Option<u32> {
            if postfix {
                return Some(0);
            }
            let hit = |tier: &Vec<String>| tier.iter().any(|x| x == lex);
            let at = if from_top { tiers.iter().rposition(hit) } else { tiers.iter().position(hit) };
            at.map(|i| i as u32 + 1)
        };
        let mut binary = HashMap::new();
        for (tag, op) in [
            ("op.add", Action::Add), ("op.sub", Action::Sub), ("op.mul", Action::Mul), ("op.div", div),
            ("op.quot", Action::IntDiv), ("op.rem", Action::Mod), ("op.pow", Action::Power), ("op.eq", Action::Eq),
            ("op.ne", Action::Ne), ("op.lt", Action::Lt), ("op.le", Action::Le), ("op.gt", Action::Gt), ("op.ge", Action::Ge),
            ("op.and", Action::And), ("op.or", Action::Or), ("op.concat", Action::Join),
        ] {
            for lex in r.strings(tag)? {
                let tier = tier_of(&lex, false).ok_or_else(|| format!("'{lex}' ({tag}) does not appear in op.precedence"))?;
                let entry = Operator { action: op.clone(), level: tier, right_assoc: rights.contains(&lex) };
                if binary.insert(lex.clone(), entry).is_some() {
                    return Err(format!("'{lex}' is listed under two binary operator labels"));
                }
            }
        }
        let mut unary = HashMap::new();
        for (tag, op) in [("op.not", Action::Not), ("op.negate", Action::Negate)] {
            for lex in r.strings(tag)? {
                let tier = tier_of(&lex, true).ok_or_else(|| format!("'{lex}' ({tag}) does not appear in op.precedence"))?;
                if unary.insert(lex.clone(), Operator { action: op.clone(), level: tier, right_assoc: false }).is_some() {
                    return Err(format!("'{lex}' is listed under two unary operator labels"));
                }
            }
        }
        let ranges = r.strings("op.range")?;
        let pipes = r.strings("op.pipe")?;
        let mut syntax_tiers = HashMap::new();
        for (tag, list) in [("op.range", &ranges), ("op.pipe", &pipes)] {
            for lex in list {
                let tier = tier_of(lex, false).ok_or_else(|| format!("'{lex}' ({tag}) does not appear in op.precedence"))?;
                syntax_tiers.insert(lex.clone(), tier);
            }
        }
        for lex in tiers.iter().flatten() {
            if !binary.contains_key(lex) && !unary.contains_key(lex) && !syntax_tiers.contains_key(lex) {
                return Err(format!("op.precedence lists '{lex}', which is under no operator tag"));
            }
        }
        if let Some(lex) = rights.iter().find(|lex| !binary.contains_key(*lex)) {
            return Err(format!("op.right_associative lists '{lex}', which is not a binary operator"));
        }

        let lets = r.strings("stmt.let")?;
        let mutables = r.strings("stmt.let.mutable")?;
        let annotation = r.strings("stmt.let.annotation")?;
        let type_first = r.flag("stmt.let.type_first")?;
        if lets.is_empty() && (!mutables.is_empty() || !annotation.is_empty() || type_first) {
            return Err("stmt.let.mutable, stmt.let.annotation and stmt.let.type_first need stmt.let".to_string());
        }
        let ifs = r.strings("stmt.if")?;
        let elifs = r.strings("stmt.elif")?;
        let elses = r.strings("stmt.else")?;
        if ifs.is_empty() && (!elifs.is_empty() || !elses.is_empty()) {
            return Err("stmt.elif and stmt.else need stmt.if".to_string());
        }
        let fors = r.strings("stmt.for")?;
        let ins = r.strings("stmt.for.in")?;
        if fors.is_empty() != ins.is_empty() && !(postfix && ins.is_empty()) {
            return Err("stmt.for and stmt.for.in must be given together".to_string());
        }
        let functions = r.strings("stmt.function")?;
        // The extension mark stands beside the core one: a language whose
        // return types the porter cannot spell says it here instead.
        let mut returns_marks = r.strings("stmt.function.returns")?;
        returns_marks.extend(r.strings("ext.stmt.function.returns")?);
        let result_by_name = r.flag("stmt.function.result_by_name")?;
        if result_by_name && functions.is_empty() {
            return Err("stmt.function.result_by_name needs stmt.function".to_string());
        }
        if type_first {
            for (tag, list) in [
                ("stmt.let.mutable", &mutables), ("stmt.let.annotation", &annotation),
                ("stmt.function", &functions), ("stmt.function.returns", &returns_marks),
            ] {
                if !list.is_empty() {
                    return Err(format!("stmt.let.type_first leaves no place for {tag}; leave it empty"));
                }
            }
        }

        let stack_lists = [
            "stack.dup", "stack.drop", "stack.swap", "stack.over", "stack.rot", "stack.eval",
            "stack.program.open", "stack.program.close",
        ]
        .iter()
        .map(|tag| r.strings(tag))
        .collect::<Result<Vec<_>, _>>()?;
        if !postfix && stack_lists.iter().any(|l| !l.is_empty()) {
            return Err("the stack.* labels need syntax.notation 'postfix'".to_string());
        }
        if stack_lists[6].len() != stack_lists[7].len() {
            return Err("stack.program.open and .close must pair up position by position".to_string());
        }
        let mut stack_lists = stack_lists.into_iter();
        let mut next_list = || stack_lists.next().expect("eight stack lists");

        let mut natives = HashMap::new();
        for (tag, native) in [
            ("builtin.emit", Builtin::Echo), ("builtin.print", Builtin::Say), ("builtin.write", Builtin::Out),
            ("builtin.len", Builtin::Length), ("builtin.char_at", Builtin::CharAtIndex), ("builtin.ord", Builtin::CodeOf),
            ("builtin.chr", Builtin::CharOf), ("builtin.typeof", Builtin::SortOf), ("builtin.error", Builtin::Raise),
            ("builtin.extern", Builtin::External), ("builtin.range", Builtin::Span), ("builtin.real", Builtin::MakeReal),
            ("builtin.precision", Builtin::Places), ("builtin.to_string", Builtin::ToText),
            ("builtin.to_int", Builtin::ToInt), ("builtin.to_real", Builtin::AsReal), ("builtin.num", Builtin::Numer),
            ("builtin.den", Builtin::Denom), ("builtin.push", Builtin::Append), ("builtin.get", Builtin::Fetch),
            ("builtin.put", Builtin::Replace), ("ext.builtin.echo", Builtin::Tell), ("ext.builtin.define", Builtin::Define),
            ("ext.builtin.var_dump", Builtin::Dump), ("ext.builtin.array", Builtin::Pack),
            ("ext.builtin.print_r", Builtin::Layout),
        ] {
            for lex in r.strings(tag)? {
                let begins = lex.chars().next().map_or(false, |c| c == '_' || c.is_alphabetic());
                if !begins || lex.chars().any(|c| c.is_whitespace() || quotes.contains(&c)) {
                    return Err(format!("builtin name '{lex}' must begin like an identifier and hold no spaces or quotes"));
                }
                if natives.insert(lex.clone(), native).is_some() {
                    return Err(format!("'{lex}' is listed under two builtin labels"));
                }
            }
        }

        let mut kind_names = Vec::new();
        for (tag, kind) in [
            ("system.kind.integer", Sort::Integer), ("system.kind.rational", Sort::Rational),
            ("system.kind.real", Sort::Real), ("system.kind.string", Sort::Text),
            ("system.kind.boolean", Sort::Boolean), ("system.kind.array", Sort::Array),
            ("system.kind.null", Sort::Null),
        ] {
            if let Some(n) = r.head(tag)? {
                kind_names.push((n, kind));
            }
        }
        let args_name = r.head("system.args")?;
        let memo_name = r.head("system.memoization")?;
        let precision_name = r.head("system.real_default_precision")?;
        let entry_name = r.head("system.entry")?;
        let system_names = [&args_name, &memo_name, &precision_name, &entry_name].into_iter().flatten();
        for n in system_names.chain(kind_names.iter().map(|(n, _)| n)) {
            if !name_like(n, unicode, var_prefix) {
                return Err(format!("system name '{n}' must be shaped like an identifier"));
            }
        }

        let mut prefix: String = name.chars().take(1).flat_map(char::to_uppercase).collect();
        prefix.push_str(name.get(1..).unwrap_or(""));
        prefix.push_str("Error");

        let mut lang = Lang {
            ident: name,
            extensions: r.strings("extensions")?,
            banner: prefix,
            line_comments: r.strings("lexical.comment_line")?,
            block_comments: comment_opens.into_iter().zip(comment_closes).collect(),
            quotes,
            raw_quotes,
            escape_letters: escapes,
            prologue: r.head("lexical.prologue")?,
            point: r.letter("lexical.number.decimal_point")?,
            base_mark: r.letter("lexical.number.base_marker")?,
            exponent_mark: r.letter("lexical.number.exponent_marker")?,
            hex_prefix,
            unicode_names: unicode,
            sigil: var_prefix,
            keywords_folded: r.flag("lexical.keywords_case_insensitive")?,
            names_folded: r.flag("identifier.case_insensitive")?,
            quote_for_names: r.letter("lexical.name_quote")?,
            symbols: Vec::new(),
            keywords: HashSet::new(),
            blocks: style,
            rpn: postfix,
            indent_width: indent,
            block_opens: openers,
            block_closes: closers,
            block_intros: r.strings("block.intro")?,
            stmt_ends: r.strings("stmt.terminator")?,
            grouping: r.brackets("syntax.group.open", "syntax.group.close", None)?,
            calling: call,
            argument_labels: call_labels,
            array_brackets: r.brackets("syntax.array.open", "syntax.array.close", Some("syntax.array.separator"))?,
            map_brackets: r.brackets("syntax.map.open", "syntax.map.close", Some("syntax.map.separator"))?,
            pair_mark: r.head("syntax.map.pair")?,
            index_brackets: index,
            text_indexable: index_text,
            true_words: r.strings("literal.true")?,
            false_words: r.strings("literal.false")?,
            null_words: r.strings("literal.null")?,
            null_silent: r.flag("literal.null.silent")?,
            dyadic: binary,
            monadic: unary,
            pipe_words: pipes,
            range_marks: ranges,
            precedence: syntax_tiers,
            assign_words: r.strings("stmt.assign")?,
            let_words: lets,
            mutable_words: mutables,
            type_marks: annotation,
            types_first: type_first,
            if_words: ifs,
            elif_words: elifs,
            else_words: elses,
            while_words: r.strings("stmt.while")?,
            until_words: r.strings("stmt.until")?,
            for_words: fors,
            in_words: ins,
            return_words: r.strings("stmt.return")?,
            break_words: r.strings("stmt.break")?,
            continue_words: r.strings("stmt.continue")?,
            function_words: functions,
            return_marks: returns_marks,
            named_result: result_by_name,
            pass_words: r.strings("stmt.pass")?,
            dup_words: next_list(),
            drop_words: next_list(),
            swap_words: next_list(),
            over_words: next_list(),
            rot_words: next_list(),
            eval_words: next_list(),
            quote_open: next_list(),
            quote_close: next_list(),
            builtins: natives,
            holes: r.strings("builtin.print.placeholder")?,
            args_binding: args_name,
            memo_binding: memo_name,
            precision_binding: precision_name,
            entry_binding: entry_name,
            sort_bindings: kind_names,
            epilogue: r.strings("ext.lexical.epilogue")?,
            bare_calls: r.flag("ext.syntax.call.bare")?,
            increments: r.strings("ext.op.increment")?,
            decrements: r.strings("ext.op.decrement")?,
            interpolating: r.letters("ext.lexical.interpolating_quotes")?,
            concat: r.head("op.concat")?,
            foreach_words: r.strings("stmt.foreach")?,
            foreach_as_words: r.strings("stmt.foreach.as")?,
            c_for_words: r.strings("ext.stmt.for.c")?,
            compound: HashMap::new(),
            static_words: r.strings("ext.stmt.static")?,
            global_words: r.strings("ext.stmt.global")?,
            const_words: r.strings("ext.stmt.const")?,
            switch_words: r.strings("ext.stmt.switch")?,
            case_words: r.strings("ext.stmt.case")?,
            default_words: r.strings("ext.stmt.default")?,
            case_marks: r.strings("ext.stmt.case.mark")?,
            ternary: match r.strings("ext.op.ternary")?.as_slice() {
                [] => None,
                [q, m] => Some((q.clone(), m.clone())),
                _ => return Err("ext.op.ternary takes exactly two signs, the question and the mark".to_string()),
            },
            lone_stmt: r.flag("ext.block.lone_statement")?,
            hoisted: r.flag("ext.stmt.function.hoisted")?,
            exponent_letters: r.letters("ext.lexical.number.exponent")?,
            plus_words: r.strings("ext.op.plus")?,
            break_levels: r.flag("ext.stmt.break.levels")?,
            append_index: r.flag("ext.op.index.append")?,
            for_collections: r.flag("ext.stmt.for.collection")?,
            class_words: r.strings("ext.stmt.class")?,
            extends_words: r.strings("ext.stmt.class.extends")?,
            new_words: r.strings("ext.stmt.class.new")?,
            this_word: r.head("ext.stmt.class.this")?,
            constructor: r.head("ext.stmt.class.constructor")?,
            modifier_words: r.strings("ext.stmt.class.modifier")?,
            shared_words: r.strings("ext.stmt.class.shared")?,
            member_mark: r.head("ext.op.member")?,
            scope_mark: r.head("ext.op.scope")?,
            instanceof_words: r.strings("ext.op.instanceof")?,
            parent_words: r.strings("ext.stmt.class.parent")?,
            self_words: r.strings("ext.stmt.class.self")?,
            name_leads: r.letters("ext.lexical.name_lead")?,
            try_words: r.strings("ext.stmt.try")?,
            catch_words: r.strings("ext.stmt.catch")?,
            finally_words: r.strings("ext.stmt.finally")?,
            throw_words: r.strings("ext.stmt.throw")?,
            catch_between: r.head("ext.stmt.catch.separator")?,
            reference_mark: r.head("ext.op.reference")?,
            absent_index: r.flag("ext.op.index.absent")?,
            request_bindings: {
                let groups = [
                    ("GET", "ext.system.request.query"), ("POST", "ext.system.request.form"),
                    ("COOKIE", "ext.system.request.cookies"), ("SERVER", "ext.system.request.server"),
                    ("ENV", "ext.system.request.env"), ("FILES", "ext.system.request.files"),
                    ("ALL", "ext.system.request.all"),
                ];
                let mut named = Vec::new();
                for (group, tag) in groups {
                    if let Some(name) = r.head(tag)? {
                        named.push((group.to_string(), name));
                    }
                }
                named
            },
        };
        if !lang.try_words.is_empty() && lang.catch_words.is_empty() {
            return Err("ext.stmt.try needs ext.stmt.catch".to_string());
        }
        if !lang.class_words.is_empty() && (lang.member_mark.is_none() || lang.new_words.is_empty()) {
            return Err("ext.stmt.class needs ext.op.member and ext.stmt.class.new".to_string());
        }
        if !lang.foreach_words.is_empty() && lang.foreach_as_words.is_empty() {
            return Err("stmt.foreach needs stmt.foreach.as".to_string());
        }
        if !r.strings("stmt.foreach.pair")?.is_empty() && lang.pair_mark.is_none() {
            return Err("stmt.foreach.pair needs syntax.map.pair".to_string());
        }
        if r.flag("ext.op.assign.compound")? {
            // Every binary operator followed by the assignment sign, unless
            // that spelling is already an operator (`<=`) or the operator
            // itself ends in the sign (`==`, whose `===` is another operator).
            let mut compound = HashMap::new();
            for (lex, op) in &lang.dyadic {
                for assign in &lang.assign_words {
                    let joined = format!("{lex}{assign}");
                    if !lex.ends_with(assign.as_str()) && !lang.dyadic.contains_key(&joined) && !lang.monadic.contains_key(&joined) {
                        compound.insert(joined, op.action.clone());
                    }
                }
            }
            lang.compound = compound;
        }
        if !lang.switch_words.is_empty() && (lang.case_words.is_empty() || lang.case_marks.is_empty()) {
            return Err("ext.stmt.switch needs ext.stmt.case and ext.stmt.case.mark".to_string());
        }
        if let Some(q) = lang.interpolating.iter().find(|q| !lang.quotes.contains(q)) {
            return Err(format!("ext.lexical.interpolating_quotes '{q}' is not among lexical.string_quotes"));
        }
        if !lang.interpolating.is_empty() && (lang.grouping.is_none() || lang.concat.is_none()) {
            return Err("ext.lexical.interpolating_quotes needs syntax.group and op.concat".to_string());
        }
        if lang.bare_calls && lang.calling.is_none() {
            return Err("ext.syntax.call.bare needs syntax.call".to_string());
        }
        lang.order_lexemes()?;
        Ok(lang)
    }

    /// Every lexeme is a symbol the scanner cuts on or a reserved word.
    fn order_lexemes(&mut self) -> Result<(), String> {
        let (unicode, prefix) = (self.unicode_names, self.sigil);
        let mut symbols: Vec<String> = Vec::new();
        let mut reserved: HashSet<String> = HashSet::new();
        let mut place = |lex: &str| {
            if name_like(lex, unicode, prefix) {
                reserved.insert(lex.to_string());
            } else if !symbols.iter().any(|s| s == lex) {
                symbols.push(lex.to_string());
            }
        };
        for lex in self.dyadic.keys().chain(self.monadic.keys()).chain(self.precedence.keys()).chain(self.compound.keys()) {
            place(lex);
        }
        if let Some((question, mark)) = &self.ternary {
            place(question);
            place(mark);
        }
        for lex in &self.plus_words {
            place(lex);
        }
        if let Some(mark) = &self.pair_mark {
            place(mark);
        }
        for mark in [&self.member_mark, &self.scope_mark, &self.catch_between, &self.reference_mark].into_iter().flatten() {
            place(mark);
        }
        for pair in [&self.grouping, &self.calling, &self.array_brackets, &self.map_brackets, &self.index_brackets].into_iter().flatten() {
            place(&pair.open);
            place(&pair.close);
            if let Some(sep) = &pair.between {
                place(sep);
            }
        }
        let mut lists: Vec<&Vec<String>> = vec![
            &self.block_intros, &self.assign_words, &self.stmt_ends, &self.argument_labels, &self.type_marks, &self.return_marks,
            &self.dup_words, &self.drop_words, &self.swap_words, &self.over_words, &self.rot_words, &self.eval_words, &self.quote_open,
            &self.quote_close, &self.increments, &self.decrements, &self.case_marks,
        ];
        if self.blocks != Blocks::Indented {
            lists.push(&self.block_opens);
            lists.push(&self.block_closes);
        }
        for lex in lists.into_iter().flatten() {
            place(lex);
        }
        let keywords = [
            &self.let_words, &self.mutable_words, &self.if_words, &self.elif_words, &self.else_words, &self.while_words, &self.until_words, &self.for_words,
            &self.in_words, &self.return_words, &self.break_words, &self.continue_words, &self.function_words, &self.pass_words, &self.true_words,
            &self.false_words, &self.null_words, &self.c_for_words, &self.static_words, &self.global_words, &self.const_words,
            &self.switch_words, &self.case_words, &self.default_words, &self.foreach_words, &self.foreach_as_words,
            &self.class_words, &self.extends_words, &self.new_words, &self.modifier_words, &self.shared_words,
            &self.instanceof_words, &self.parent_words, &self.self_words, &self.try_words, &self.catch_words,
            &self.finally_words, &self.throw_words,
        ];
        for word in keywords.into_iter().flatten() {
            if !name_like(word, unicode, prefix) {
                return Err(format!("keyword '{word}' must be shaped like an identifier"));
            }
            reserved.insert(word.clone());
        }
        if self.keywords_folded {
            reserved = reserved.into_iter().map(|w| w.to_lowercase()).collect();
        }
        symbols.sort_by_key(|s| std::cmp::Reverse(s.len()));
        self.symbols = symbols;
        self.keywords = reserved;
        Ok(())
    }

    pub fn spells(list: &[String], word: &str) -> bool {
        list.iter().any(|w| w == word)
    }

    pub fn ends_stmt(&self, lex: &str) -> bool {
        Lang::spells(&self.stmt_ends, lex)
    }

    pub fn begins_name(&self, c: char) -> bool {
        c == '_' || if self.unicode_names { c.is_alphabetic() } else { c.is_ascii_alphabetic() }
    }

    pub fn extends_name(&self, c: char) -> bool {
        c == '_' || if self.unicode_names { c.is_alphanumeric() } else { c.is_ascii_alphanumeric() }
    }
}
