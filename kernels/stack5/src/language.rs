// A language definition read from JSON into the tables the assembler and
// the machine consult. Every label must be present and of the right shape;
// keys beginning with `$` are notes and are skipped.

use std::collections::{HashMap, HashSet};

use serde_json::Value as Json;

use crate::values::Kind;
use crate::words::{Native, Op};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Style {
    Indented,
    Braced,
    Keyword,
}

#[derive(Clone, Debug)]
pub struct Pair {
    pub open: String,
    pub close: String,
    pub sep: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Infix {
    pub op: Op,
    pub tier: u32,
    pub right: bool,
}

pub struct Def {
    pub name: String,
    pub extensions: Vec<String>,
    pub prefix: String,

    pub line_comments: Vec<String>,
    pub block_comments: Vec<(String, String)>,
    pub quotes: Vec<char>,
    pub raw_quotes: Vec<char>,
    pub escapes: Vec<char>,
    pub prologue: Option<String>,
    pub point: Option<char>,
    pub base_mark: Option<char>,
    pub exponent_mark: Option<char>,
    pub hex_prefix: Option<String>,
    pub unicode: bool,
    pub var_prefix: Option<char>,
    pub fold_keywords: bool,
    pub fold_names: bool,
    pub name_quote: Option<char>,
    pub symbols: Vec<String>,
    pub reserved: HashSet<String>,

    pub style: Style,
    /// Reverse Polish: words acting on one stack, no expressions.
    pub postfix: bool,
    pub indent: usize,
    pub openers: Vec<String>,
    pub closers: Vec<String>,
    pub intros: Vec<String>,
    pub terminators: Vec<String>,
    pub group: Option<Pair>,
    pub call: Option<Pair>,
    pub call_labels: Vec<String>,
    pub array: Option<Pair>,
    pub index: Option<Pair>,
    pub index_text: bool,

    pub yes: Vec<String>,
    pub no: Vec<String>,
    pub none: Vec<String>,
    pub binary: HashMap<String, Infix>,
    pub unary: HashMap<String, Infix>,
    pub pipes: Vec<String>,
    pub ranges: Vec<String>,
    pub syntax_tiers: HashMap<String, u32>,

    pub assign: Vec<String>,
    pub lets: Vec<String>,
    pub mutables: Vec<String>,
    pub annotation: Vec<String>,
    pub type_first: bool,
    pub ifs: Vec<String>,
    pub elifs: Vec<String>,
    pub elses: Vec<String>,
    pub whiles: Vec<String>,
    pub untils: Vec<String>,
    pub fors: Vec<String>,
    pub ins: Vec<String>,
    pub returns: Vec<String>,
    pub breaks: Vec<String>,
    pub continues: Vec<String>,
    pub functions: Vec<String>,
    pub returns_marks: Vec<String>,
    pub result_by_name: bool,
    pub passes: Vec<String>,

    pub dup: Vec<String>,
    pub drop: Vec<String>,
    pub swap: Vec<String>,
    pub over: Vec<String>,
    pub rot: Vec<String>,
    pub eval: Vec<String>,
    pub program_open: Vec<String>,
    pub program_close: Vec<String>,

    pub natives: HashMap<String, Native>,
    pub placeholders: Vec<String>,
    pub args_name: Option<String>,
    pub memo_name: Option<String>,
    pub precision_name: Option<String>,
    pub entry_name: Option<String>,
    pub kind_names: Vec<(String, Kind)>,
}

/// Every label with its shape: w a word list, s a string, b a switch,
/// n a size or null, o a string or null, t precedence tiers, x a word
/// list this kernel gives no meaning and requires empty.
const LABELS: &str = "
n format_version | s language | w extensions
w lexical.comment_line | w lexical.comment_block.open | w lexical.comment_block.close
w lexical.string_quotes | w lexical.raw_quotes | w lexical.string_escapes | w lexical.prologue
w lexical.name_quote | w lexical.number.decimal_point | w lexical.number.base_marker
w lexical.number.exponent_marker | w lexical.number.hex_prefix | b lexical.keywords_case_insensitive
b identifier.unicode | w identifier.variable_prefix | b identifier.case_insensitive
s block.style | w block.open | w block.close | w block.intro | n block.indent_size
w stmt.terminator | s syntax.notation
w syntax.group.open | w syntax.group.close
w syntax.call.open | w syntax.call.separator | w syntax.call.close | w syntax.call.label
w syntax.array.open | w syntax.array.separator | w syntax.array.close
x syntax.map.open | x syntax.map.separator | x syntax.map.pair | x syntax.map.close
w literal.true | w literal.false | w literal.null
t op.precedence | w op.right_associative
w op.add | w op.sub | w op.mul | w op.div | o op.div.result | w op.quot | w op.rem | w op.pow
w op.eq | w op.ne | w op.lt | w op.le | w op.gt | w op.ge
w op.and | w op.or | w op.not | w op.negate | w op.concat | w op.range
w op.index.open | w op.index.close | b op.index.strings | w op.pipe
w stmt.assign | w stmt.let | w stmt.let.mutable | w stmt.let.annotation | b stmt.let.type_first
w stmt.if | w stmt.elif | w stmt.else | w stmt.while | w stmt.until | w stmt.for | w stmt.for.in
x stmt.foreach | x stmt.foreach.as | x stmt.foreach.pair
w stmt.return | w stmt.break | w stmt.continue | w stmt.function | w stmt.function.returns
b stmt.function.result_by_name | w stmt.pass | x stmt.emit
w stack.dup | w stack.drop | w stack.swap | w stack.over | w stack.rot | w stack.eval
w stack.program.open | w stack.program.close
w builtin.emit | w builtin.print | w builtin.write | w builtin.print.placeholder
w builtin.len | w builtin.char_at | w builtin.ord | w builtin.chr | w builtin.typeof | w builtin.error
w builtin.extern | w builtin.range | w builtin.real | w builtin.num | w builtin.den | w builtin.push
w builtin.get | w builtin.put | w builtin.precision | w builtin.to_string | w builtin.to_int | w builtin.to_real
w system.args | w system.memoization | w system.real_default_precision | w system.entry
w system.kind.integer | w system.kind.rational | w system.kind.real | w system.kind.string
w system.kind.boolean | w system.kind.array | w system.kind.null
";

fn shapes() -> Vec<(char, &'static str)> {
    LABELS
        .split(['\n', '|'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|entry| (entry.chars().next().unwrap(), entry[2..].trim()))
        .collect()
}

struct Reader<'a>(&'a serde_json::Map<String, Json>);

impl<'a> Reader<'a> {
    fn at(&self, key: &str) -> Result<&'a Json, String> {
        self.0.get(key).ok_or_else(|| format!("missing label '{key}'"))
    }

    fn list(&self, key: &str) -> Result<Vec<String>, String> {
        let Json::Array(items) = self.at(key)? else {
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

    fn first(&self, key: &str) -> Result<Option<String>, String> {
        Ok(self.list(key)?.into_iter().next())
    }

    fn chars(&self, key: &str) -> Result<Vec<char>, String> {
        self.list(key)?
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

    fn char(&self, key: &str) -> Result<Option<char>, String> {
        Ok(self.chars(key)?.into_iter().next())
    }

    fn switch(&self, key: &str) -> Result<bool, String> {
        match self.at(key)? {
            Json::Bool(b) => Ok(*b),
            _ => Err(format!("label '{key}' must be true or false")),
        }
    }

    fn text(&self, key: &str) -> Result<String, String> {
        match self.at(key)? {
            Json::String(s) if !s.is_empty() => Ok(s.clone()),
            _ => Err(format!("label '{key}' must be a non-empty string")),
        }
    }

    fn text_or_null(&self, key: &str) -> Result<Option<String>, String> {
        match self.at(key)? {
            Json::Null => Ok(None),
            Json::String(s) if !s.is_empty() => Ok(Some(s.clone())),
            _ => Err(format!("label '{key}' must be a non-empty string or null")),
        }
    }

    fn size(&self, key: &str) -> Result<Option<usize>, String> {
        match self.at(key)? {
            Json::Null => Ok(None),
            Json::Number(n) if n.as_u64().is_some() => Ok(n.as_u64().map(|n| n as usize)),
            _ => Err(format!("label '{key}' must be a non-negative integer or null")),
        }
    }

    fn tiers(&self, key: &str) -> Result<Vec<Vec<String>>, String> {
        let wrong = || format!("label '{key}' must be a list of lists of strings");
        let Json::Array(tiers) = self.at(key)? else { return Err(wrong()) };
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

    fn pair(&self, open: &str, close: &str, sep: Option<&str>) -> Result<Option<Pair>, String> {
        let sep = match sep {
            Some(key) => self.first(key)?,
            None => None,
        };
        match (self.first(open)?, self.first(close)?) {
            (Some(open), Some(close)) => Ok(Some(Pair { open, close, sep })),
            (None, None) if sep.is_none() => Ok(None),
            _ => Err(format!("labels '{open}' and '{close}' must be given together")),
        }
    }
}

fn root(text: &str) -> Result<serde_json::Map<String, Json>, String> {
    match serde_json::from_str(text).map_err(|e| format!("definition is not valid JSON: {e}"))? {
        Json::Object(map) => Ok(map),
        _ => Err("definition must be a JSON object".to_string()),
    }
}

/// The language name and extensions a definition declares.
pub fn describe(text: &str) -> Result<(String, Vec<String>), String> {
    let map = root(text)?;
    let r = Reader(&map);
    Ok((r.text("language")?, r.list("extensions")?))
}

fn identifier_like(s: &str, unicode: bool, prefix: Option<char>) -> bool {
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

impl Def {
    pub fn read(text: &str) -> Result<Def, String> {
        let map = root(text)?;
        let shapes = shapes();
        for (_, label) in &shapes {
            if !map.contains_key(*label) {
                return Err(format!("missing label '{label}'"));
            }
        }
        let mut strange: Vec<&str> = map
            .keys()
            .filter(|k| !k.starts_with('$') && !shapes.iter().any(|(_, l)| l == k))
            .map(String::as_str)
            .collect();
        strange.sort();
        if !strange.is_empty() {
            return Err(format!("unknown label(s): {}", strange.join(", ")));
        }
        let r = Reader(&map);
        if r.size("format_version")? != Some(1) {
            return Err("format_version must be 1".to_string());
        }
        for (shape, label) in &shapes {
            if *shape == 'x' && !r.list(label)?.is_empty() {
                return Err(format!("label '{label}' is not implemented by the stack5 kernel; leave it empty"));
            }
        }

        let name = r.text("language")?;
        let unicode = r.switch("identifier.unicode")?;
        let var_prefix = r.char("identifier.variable_prefix")?;

        let comment_opens = r.list("lexical.comment_block.open")?;
        let comment_closes = r.list("lexical.comment_block.close")?;
        if comment_opens.len() != comment_closes.len() {
            return Err("lexical.comment_block.open and .close must pair up position by position".to_string());
        }
        let quotes = r.chars("lexical.string_quotes")?;
        let raw_quotes = r.chars("lexical.raw_quotes")?;
        if let Some(q) = raw_quotes.iter().find(|q| !quotes.contains(q)) {
            return Err(format!("lexical.raw_quotes lists '{q}', which is not in lexical.string_quotes"));
        }
        let escapes = r.chars("lexical.string_escapes")?;
        if let Some(e) = escapes.iter().find(|e| !matches!(e, 'n' | 't' | 'r' | '0' | '\\') && !quotes.contains(e)) {
            return Err(format!("lexical.string_escapes: unknown escape letter '{e}'"));
        }
        let hex_prefix = r.first("lexical.number.hex_prefix")?;
        if let Some(p) = &hex_prefix {
            if p.chars().count() != 2 || !p.starts_with(|c: char| c.is_ascii_digit()) {
                return Err(format!("lexical.number.hex_prefix must be a digit followed by one letter, got '{p}'"));
            }
        }

        let style = match r.text("block.style")?.as_str() {
            "indentation" => Style::Indented,
            "braces" => Style::Braced,
            "keyword" => Style::Keyword,
            other => return Err(format!("block.style must be 'indentation', 'braces' or 'keyword', got '{other}'")),
        };
        let postfix = match r.text("syntax.notation")?.as_str() {
            "infix" => false,
            "postfix" => true,
            other => return Err(format!("syntax.notation must be 'infix' or 'postfix', got '{other}'")),
        };
        let openers = r.list("block.open")?;
        let closers = r.list("block.close")?;
        let size = r.size("block.indent_size")?;
        let indent = match style {
            Style::Indented => {
                if !openers.is_empty() || !closers.is_empty() {
                    return Err("indentation blocks take no block.open or block.close".to_string());
                }
                match size {
                    Some(0) => return Err("block.indent_size must be at least 1".to_string()),
                    Some(n) => n,
                    None => return Err("indentation blocks need block.indent_size".to_string()),
                }
            }
            Style::Braced => {
                if openers.is_empty() || openers.len() != closers.len() {
                    return Err("braces need block.open and block.close, paired position by position".to_string());
                }
                size.unwrap_or(4)
            }
            Style::Keyword => {
                if !openers.is_empty() || closers.is_empty() {
                    return Err("keyword blocks take no block.open and need block.close".to_string());
                }
                size.unwrap_or(4)
            }
        };
        let call = r.pair("syntax.call.open", "syntax.call.close", Some("syntax.call.separator"))?;
        let call_labels = r.list("syntax.call.label")?;
        if !call_labels.is_empty() && call.is_none() {
            return Err("syntax.call.label needs syntax.call.open".to_string());
        }
        let index = r.pair("op.index.open", "op.index.close", None)?;
        let index_text = r.switch("op.index.strings")?;
        if index_text && index.is_none() {
            return Err("op.index.strings needs op.index.open".to_string());
        }

        let tiers = r.tiers("op.precedence")?;
        if postfix && !tiers.is_empty() {
            return Err("a postfix language takes no op.precedence".to_string());
        }
        let rights = r.list("op.right_associative")?;
        let div = match r.text_or_null("op.div.result")?.as_deref() {
            None | Some("rational") => Op::Div,
            Some("real") => Op::RealDiv,
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
        for (label, op) in [
            ("op.add", Op::Add), ("op.sub", Op::Sub), ("op.mul", Op::Mul), ("op.div", div),
            ("op.quot", Op::Quot), ("op.rem", Op::Rem), ("op.pow", Op::Pow), ("op.eq", Op::Eq),
            ("op.ne", Op::Ne), ("op.lt", Op::Lt), ("op.le", Op::Le), ("op.gt", Op::Gt), ("op.ge", Op::Ge),
            ("op.and", Op::And), ("op.or", Op::Or), ("op.concat", Op::Concat),
        ] {
            for lex in r.list(label)? {
                let tier = tier_of(&lex, false).ok_or_else(|| format!("'{lex}' ({label}) does not appear in op.precedence"))?;
                let entry = Infix { op: op.clone(), tier, right: rights.contains(&lex) };
                if binary.insert(lex.clone(), entry).is_some() {
                    return Err(format!("'{lex}' is listed under two binary operator labels"));
                }
            }
        }
        let mut unary = HashMap::new();
        for (label, op) in [("op.not", Op::Not), ("op.negate", Op::Neg)] {
            for lex in r.list(label)? {
                let tier = tier_of(&lex, true).ok_or_else(|| format!("'{lex}' ({label}) does not appear in op.precedence"))?;
                if unary.insert(lex.clone(), Infix { op: op.clone(), tier, right: false }).is_some() {
                    return Err(format!("'{lex}' is listed under two unary operator labels"));
                }
            }
        }
        let ranges = r.list("op.range")?;
        let pipes = r.list("op.pipe")?;
        let mut syntax_tiers = HashMap::new();
        for (label, list) in [("op.range", &ranges), ("op.pipe", &pipes)] {
            for lex in list {
                let tier = tier_of(lex, false).ok_or_else(|| format!("'{lex}' ({label}) does not appear in op.precedence"))?;
                syntax_tiers.insert(lex.clone(), tier);
            }
        }
        for lex in tiers.iter().flatten() {
            if !binary.contains_key(lex) && !unary.contains_key(lex) && !syntax_tiers.contains_key(lex) {
                return Err(format!("op.precedence lists '{lex}', which is under no operator label"));
            }
        }
        if let Some(lex) = rights.iter().find(|lex| !binary.contains_key(*lex)) {
            return Err(format!("op.right_associative lists '{lex}', which is not a binary operator"));
        }

        let lets = r.list("stmt.let")?;
        let mutables = r.list("stmt.let.mutable")?;
        let annotation = r.list("stmt.let.annotation")?;
        let type_first = r.switch("stmt.let.type_first")?;
        if lets.is_empty() && (!mutables.is_empty() || !annotation.is_empty() || type_first) {
            return Err("stmt.let.mutable, stmt.let.annotation and stmt.let.type_first need stmt.let".to_string());
        }
        let ifs = r.list("stmt.if")?;
        let elifs = r.list("stmt.elif")?;
        let elses = r.list("stmt.else")?;
        if ifs.is_empty() && (!elifs.is_empty() || !elses.is_empty()) {
            return Err("stmt.elif and stmt.else need stmt.if".to_string());
        }
        let fors = r.list("stmt.for")?;
        let ins = r.list("stmt.for.in")?;
        if fors.is_empty() != ins.is_empty() && !(postfix && ins.is_empty()) {
            return Err("stmt.for and stmt.for.in must be given together".to_string());
        }
        let functions = r.list("stmt.function")?;
        let returns_marks = r.list("stmt.function.returns")?;
        let result_by_name = r.switch("stmt.function.result_by_name")?;
        if result_by_name && functions.is_empty() {
            return Err("stmt.function.result_by_name needs stmt.function".to_string());
        }
        if type_first {
            for (label, list) in [
                ("stmt.let.mutable", &mutables), ("stmt.let.annotation", &annotation),
                ("stmt.function", &functions), ("stmt.function.returns", &returns_marks),
            ] {
                if !list.is_empty() {
                    return Err(format!("stmt.let.type_first leaves no place for {label}; leave it empty"));
                }
            }
        }

        let stack_lists = [
            "stack.dup", "stack.drop", "stack.swap", "stack.over", "stack.rot", "stack.eval",
            "stack.program.open", "stack.program.close",
        ]
        .iter()
        .map(|label| r.list(label))
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
        for (label, native) in [
            ("builtin.emit", Native::Emit), ("builtin.print", Native::Print), ("builtin.write", Native::Write),
            ("builtin.len", Native::Len), ("builtin.char_at", Native::CharAt), ("builtin.ord", Native::Ord),
            ("builtin.chr", Native::Chr), ("builtin.typeof", Native::Kind), ("builtin.error", Native::Fail),
            ("builtin.extern", Native::Extern), ("builtin.range", Native::Range), ("builtin.real", Native::Real),
            ("builtin.precision", Native::Precision), ("builtin.to_string", Native::Text),
            ("builtin.to_int", Native::Int), ("builtin.to_real", Native::ToReal), ("builtin.num", Native::Num),
            ("builtin.den", Native::Den), ("builtin.push", Native::Push), ("builtin.get", Native::Get),
            ("builtin.put", Native::Put),
        ] {
            for lex in r.list(label)? {
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
        for (label, kind) in [
            ("system.kind.integer", Kind::Integer), ("system.kind.rational", Kind::Rational),
            ("system.kind.real", Kind::Real), ("system.kind.string", Kind::Text),
            ("system.kind.boolean", Kind::Boolean), ("system.kind.array", Kind::Array),
            ("system.kind.null", Kind::Null),
        ] {
            if let Some(n) = r.first(label)? {
                kind_names.push((n, kind));
            }
        }
        let args_name = r.first("system.args")?;
        let memo_name = r.first("system.memoization")?;
        let precision_name = r.first("system.real_default_precision")?;
        let entry_name = r.first("system.entry")?;
        let system_names = [&args_name, &memo_name, &precision_name, &entry_name].into_iter().flatten();
        for n in system_names.chain(kind_names.iter().map(|(n, _)| n)) {
            if !identifier_like(n, unicode, var_prefix) {
                return Err(format!("system name '{n}' must be shaped like an identifier"));
            }
        }

        let mut prefix: String = name.chars().take(1).flat_map(char::to_uppercase).collect();
        prefix.push_str(name.get(1..).unwrap_or(""));
        prefix.push_str("Error");

        let mut def = Def {
            name,
            extensions: r.list("extensions")?,
            prefix,
            line_comments: r.list("lexical.comment_line")?,
            block_comments: comment_opens.into_iter().zip(comment_closes).collect(),
            quotes,
            raw_quotes,
            escapes,
            prologue: r.first("lexical.prologue")?,
            point: r.char("lexical.number.decimal_point")?,
            base_mark: r.char("lexical.number.base_marker")?,
            exponent_mark: r.char("lexical.number.exponent_marker")?,
            hex_prefix,
            unicode,
            var_prefix,
            fold_keywords: r.switch("lexical.keywords_case_insensitive")?,
            fold_names: r.switch("identifier.case_insensitive")?,
            name_quote: r.char("lexical.name_quote")?,
            symbols: Vec::new(),
            reserved: HashSet::new(),
            style,
            postfix,
            indent,
            openers,
            closers,
            intros: r.list("block.intro")?,
            terminators: r.list("stmt.terminator")?,
            group: r.pair("syntax.group.open", "syntax.group.close", None)?,
            call,
            call_labels,
            array: r.pair("syntax.array.open", "syntax.array.close", Some("syntax.array.separator"))?,
            index,
            index_text,
            yes: r.list("literal.true")?,
            no: r.list("literal.false")?,
            none: r.list("literal.null")?,
            binary,
            unary,
            pipes,
            ranges,
            syntax_tiers,
            assign: r.list("stmt.assign")?,
            lets,
            mutables,
            annotation,
            type_first,
            ifs,
            elifs,
            elses,
            whiles: r.list("stmt.while")?,
            untils: r.list("stmt.until")?,
            fors,
            ins,
            returns: r.list("stmt.return")?,
            breaks: r.list("stmt.break")?,
            continues: r.list("stmt.continue")?,
            functions,
            returns_marks,
            result_by_name,
            passes: r.list("stmt.pass")?,
            dup: next_list(),
            drop: next_list(),
            swap: next_list(),
            over: next_list(),
            rot: next_list(),
            eval: next_list(),
            program_open: next_list(),
            program_close: next_list(),
            natives,
            placeholders: r.list("builtin.print.placeholder")?,
            args_name,
            memo_name,
            precision_name,
            entry_name,
            kind_names,
        };
        def.sort_lexemes()?;
        Ok(def)
    }

    /// Every lexeme is a symbol the scanner cuts on or a reserved word.
    fn sort_lexemes(&mut self) -> Result<(), String> {
        let (unicode, prefix) = (self.unicode, self.var_prefix);
        let mut symbols: Vec<String> = Vec::new();
        let mut reserved: HashSet<String> = HashSet::new();
        let mut place = |lex: &str| {
            if identifier_like(lex, unicode, prefix) {
                reserved.insert(lex.to_string());
            } else if !symbols.iter().any(|s| s == lex) {
                symbols.push(lex.to_string());
            }
        };
        for lex in self.binary.keys().chain(self.unary.keys()).chain(self.syntax_tiers.keys()) {
            place(lex);
        }
        for pair in [&self.group, &self.call, &self.array, &self.index].into_iter().flatten() {
            place(&pair.open);
            place(&pair.close);
            if let Some(sep) = &pair.sep {
                place(sep);
            }
        }
        let mut lists: Vec<&Vec<String>> = vec![
            &self.intros, &self.assign, &self.terminators, &self.call_labels, &self.annotation, &self.returns_marks,
            &self.dup, &self.drop, &self.swap, &self.over, &self.rot, &self.eval, &self.program_open,
            &self.program_close,
        ];
        if self.style != Style::Indented {
            lists.push(&self.openers);
            lists.push(&self.closers);
        }
        for lex in lists.into_iter().flatten() {
            place(lex);
        }
        let keywords = [
            &self.lets, &self.mutables, &self.ifs, &self.elifs, &self.elses, &self.whiles, &self.untils, &self.fors,
            &self.ins, &self.returns, &self.breaks, &self.continues, &self.functions, &self.passes, &self.yes,
            &self.no, &self.none,
        ];
        for word in keywords.into_iter().flatten() {
            if !identifier_like(word, unicode, prefix) {
                return Err(format!("keyword '{word}' must be shaped like an identifier"));
            }
            reserved.insert(word.clone());
        }
        if self.fold_keywords {
            reserved = reserved.into_iter().map(|w| w.to_lowercase()).collect();
        }
        symbols.sort_by_key(|s| std::cmp::Reverse(s.len()));
        self.symbols = symbols;
        self.reserved = reserved;
        Ok(())
    }

    pub fn has(list: &[String], word: &str) -> bool {
        list.iter().any(|w| w == word)
    }

    pub fn ends_statement(&self, lex: &str) -> bool {
        Def::has(&self.terminators, lex)
    }

    pub fn starts_word(&self, c: char) -> bool {
        c == '_' || if self.unicode { c.is_alphabetic() } else { c.is_ascii_alphabetic() }
    }

    pub fn continues_word(&self, c: char) -> bool {
        c == '_' || if self.unicode { c.is_alphanumeric() } else { c.is_ascii_alphanumeric() }
    }
}
