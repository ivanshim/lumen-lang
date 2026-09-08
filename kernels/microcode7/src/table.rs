// The definition as data. Every label is read into a typed cell, checked
// for shape, and a few tables are derived for the stages.

use std::collections::{HashMap, HashSet};

use serde_json::Value as Json;

use crate::form::Prim;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Blocks {
    Indented,
    Bracketed,
    Worded,
}

#[derive(Debug)]
enum Entry {
    Strings(Vec<String>),
    Flag(bool),
    Count(Option<usize>),
    One(Option<String>),
    Tiers(Vec<Vec<String>>),
}

#[derive(Clone, Copy, Debug)]
pub struct Infix {
    pub prim: Prim,
    pub level: u32,
    pub right_assoc: bool,
}

pub struct Table {
    pub ident: String,
    pub blocks: Blocks,
    /// Reverse Polish: words over one stack, no expressions.
    pub rpn: bool,
    cells: HashMap<&'static str, Entry>,
    pub dyadic: HashMap<String, Infix>,
    pub monadic: HashMap<String, Infix>,
    pub precedence: HashMap<String, u32>,
    pub prims: HashMap<String, Prim>,
    /// `x op= e` for each binary operator (ext.op.assign.compound).
    pub compound: HashMap<String, Prim>,
    /// The labels the definition wrote out, empty ones included.
    given: HashSet<String>,
    pub keywords: HashSet<String>,
    pub signs: Vec<String>,
}

// Label shapes: L list of words, B boolean, N count or null, W word, O word or null, T tiers.
const TAGS: &str = "\
format_version:N language:W extensions:L lexical.comment_line:L lexical.comment_block.open:L \
lexical.comment_block.close:L lexical.string_quotes:L lexical.raw_quotes:L lexical.string_escapes:L lexical.prologue:L \
lexical.name_quote:L lexical.number.decimal_point:L lexical.number.base_marker:L lexical.number.exponent_marker:L lexical.number.hex_prefix:L \
lexical.keywords_case_insensitive:B identifier.unicode:B identifier.variable_prefix:L identifier.case_insensitive:B block.style:W \
block.open:L block.close:L block.intro:L block.indent_size:N stmt.terminator:L \
syntax.notation:W syntax.group.open:L syntax.group.close:L syntax.call.open:L syntax.call.separator:L \
syntax.call.close:L syntax.call.label:L syntax.array.open:L syntax.array.separator:L syntax.array.close:L \
syntax.map.open:L syntax.map.separator:L syntax.map.pair:L syntax.map.close:L system.flag.counts:B literal.true:L \
literal.false:L literal.null:L literal.null.silent:B op.precedence:T op.right_associative:L op.add:L \
op.sub:L op.mul:L op.div:L op.div.result:O op.quot:L \
op.rem:L op.pow:L op.eq:L op.ne:L op.lt:L \
op.le:L op.gt:L op.ge:L op.and:L op.or:L \
op.not:L op.negate:L op.concat:L op.range:L op.index.open:L \
op.index.close:L op.index.strings:B op.pipe:L stmt.assign:L stmt.let:L \
stmt.let.mutable:L stmt.let.annotation:L stmt.let.type_first:B stmt.if:L stmt.elif:L \
stmt.else:L stmt.while:L stmt.until:L stmt.for:L stmt.for.in:L \
stmt.foreach:L stmt.foreach.as:L stmt.foreach.pair:L stmt.return:L stmt.break:L \
stmt.continue:L stmt.function:L stmt.function.returns:L stmt.function.result_by_name:B stmt.pass:L \
stmt.emit:L stack.dup:L stack.drop:L stack.swap:L stack.over:L \
stack.rot:L stack.eval:L stack.program.open:L stack.program.close:L builtin.emit:L \
builtin.print:L builtin.write:L builtin.print.placeholder:L builtin.len:L builtin.char_at:L \
builtin.ord:L builtin.chr:L builtin.typeof:L builtin.error:L builtin.extern:L \
builtin.range:L builtin.real:L builtin.num:L builtin.den:L builtin.push:L \
builtin.get:L builtin.put:L builtin.precision:L builtin.to_string:L builtin.to_int:L \
builtin.to_real:L system.args:L system.memoization:L system.real_default_precision:L system.entry:L \
system.kind.integer:L system.kind.rational:L system.kind.real:L system.kind.string:L system.kind.boolean:L \
system.kind.array:L system.kind.null:L \
";

/// Extension labels beyond the core: optional, an absent one is empty
/// (or off). The reference kernels skip them; this one reads them.
const EXT_TAGS: &str = "\
ext.lexical.epilogue:L ext.builtin.echo:L ext.syntax.call.bare:B ext.op.increment:L ext.op.decrement:L \
ext.lexical.interpolating_quotes:L ext.stmt.for.c:L ext.op.assign.compound:B ext.stmt.static:L ext.stmt.global:L \
ext.stmt.const:L ext.builtin.define:L ext.builtin.var_dump:L ext.stmt.switch:L ext.stmt.case:L \
ext.stmt.default:L ext.stmt.case.mark:L ext.op.ternary:L ext.block.lone_statement:B ext.stmt.function.hoisted:B ext.stmt.function.outermost:B ext.system.request.amiss:L ext.system.request.amiss.boundary:L ext.system.request.amiss.boundary.wrong:L ext.system.request.amiss.part:L ext.system.request.amiss.body.large:L ext.system.request.body:L \
ext.lexical.number.exponent:L ext.op.plus:L ext.stmt.break.levels:B ext.builtin.array:L ext.op.index.append:B ext.stmt.for.collection:B ext.builtin.print_r:L \
ext.stmt.function.returns:L ext.stmt.class:L ext.stmt.class.extends:L ext.stmt.class.new:L ext.stmt.class.this:L \
ext.stmt.class.constructor:L ext.stmt.class.modifier:L ext.stmt.class.shared:L ext.op.member:L ext.op.scope:L \
ext.op.instanceof:L ext.stmt.class.parent:L ext.stmt.class.self:L ext.lexical.name_lead:L ext.stmt.try:L \
ext.stmt.catch:L ext.stmt.finally:L ext.stmt.throw:L ext.stmt.catch.separator:L ext.op.reference:L \
ext.system.request.query:L ext.system.request.form:L ext.system.request.cookies:L ext.system.request.server:L \
ext.system.request.env:L ext.system.request.files:L ext.system.request.all:L ext.system.request.settings:L ext.op.index.absent:B \
ext.stmt.class.interface:L ext.stmt.class.implements:L ext.op.compare:L ext.builtin.unset:L ext.lexical.template:B \
ext.op.otherwise:L ext.op.bit.and:L ext.op.bit.or:L ext.op.bit.xor:L ext.op.bit.not:L \
ext.op.bit.left:L ext.op.bit.right:L ext.op.identical:L ext.op.not_identical:L ext.system.kind.spelled:B ext.builtin.args.all:L \
ext.builtin.args.count:L ext.builtin.args.at:L ext.builtin.args.all.outside:L ext.builtin.args.count.outside:L ext.builtin.args.at.outside:L ext.builtin.args.at.below:L ext.builtin.args.at.beyond:L ext.op.assign.value:B ext.op.index.plain_keys:B \
ext.system.source.file:L ext.system.source.directory:L ext.system.source.line:L ext.system.source.routine:L ext.system.source.class:L ext.system.source.method:L \
ext.system.complaint.warning:L ext.system.complaint.notice:L ext.system.complaint.deprecated:L ext.system.complaint.fatal:L ext.system.fault.class:L ext.system.fault.operands:L ext.op.increment.text:L ext.op.decrement.text:L ext.system.fault.class.arithmetic:L ext.system.fault.class.division:L ext.system.fault.class.kind:L ext.system.fault.class.value:L ext.builtin.time_limit:L ext.system.kind.brief:L ext.builtin.file.read:L ext.builtin.file.write:L \
ext.builtin.file.exists:L ext.builtin.file.remove:L ext.builtin.eval:L ext.builtin.include:L ext.builtin.include.once:L ext.builtin.output.hold:L ext.builtin.output.held:L ext.builtin.output.drop:L ext.builtin.output.depth:L ext.builtin.at_end:L ext.builtin.complaint.handler:L ext.builtin.complaint.say:L ext.op.hush:L ext.op.name_by_value:L ext.op.cast:B ext.op.member.by_value:B ext.op.index.text:B ext.system.globals:L ext.op.reference.unshared.written:L ext.op.reference.unshared.given:L ext.op.reference.unshared.handed:L ext.stmt.block.instead:L ext.stmt.block.instead.close:L ext.op.spelled:B ext.system.class.folded:B ext.stmt.unpack:L ext.builtin.isset:L ext.builtin.empty:L ext.stmt.do:L ext.op.index.makes:B ext.system.untrue.text:L ext.system.untrue.empty_array:B ext.builtin.exit:L \
ext.lexical.escape.codepoint:L ext.lexical.escape.codepoint.open:L ext.lexical.escape.codepoint.close:L ext.lexical.escape.codepoint.amiss:L ext.lexical.escape.codepoint.beyond:L \
ext.lexical.number.binary_prefix:L ext.lexical.number.octal_prefix:L ext.lexical.number.octal_lead:B \
ext.lexical.number.separator:L ext.system.integer.bits:N ext.system.real.bits:N ext.system.real.digits:N \
";

fn tag_shapes(table: &'static str) -> Vec<(&'static str, char)> {
    table.split_whitespace().map(|e| {
        let (k, s) = e.rsplit_once(':').unwrap();
        (k, s.chars().next().unwrap())
    }).collect()
}

fn cell_of(key: &str, shape: char, json: &Json) -> Result<Entry, String> {
    Ok(match (shape, json) {
        ('L', j) => Entry::Strings(strings_at(key, j)?),
        ('B', Json::Bool(b)) => Entry::Flag(*b),
        ('N', Json::Null) => Entry::Count(None),
        ('N', Json::Number(n)) if n.as_u64().is_some() => Entry::Count(n.as_u64().map(|n| n as usize)),
        ('W', Json::String(s)) if !s.is_empty() => Entry::One(Some(s.clone())),
        ('O', Json::Null) => Entry::One(None),
        ('O', Json::String(s)) if !s.is_empty() => Entry::One(Some(s.clone())),
        ('T', Json::Array(tiers)) => Entry::Tiers(tiers.iter().map(|t| strings_at(key, t)).collect::<Result<_, _>>()?),
        _ => return Err(format!("label '{key}' has the wrong shape")),
    })
}

/// Labels this kernel gives no meaning to: read past, as an ext.* label is.
#[allow(dead_code)]
const MUST_BE_EMPTY: [&str; 8] = [
    "syntax.map.open", "syntax.map.separator", "syntax.map.pair", "syntax.map.close",
    "stmt.foreach", "stmt.foreach.as", "stmt.foreach.pair", "stmt.emit",
];

/// Builtin labels and the operation each names.
pub const BUILTIN_LABELS: [(&str, Prim); 48] = [
    ("builtin.emit", Prim::Echo), ("builtin.print", Prim::Say), ("builtin.write", Prim::Out), ("builtin.len", Prim::Length),
    ("builtin.char_at", Prim::CharAtIndex), ("builtin.ord", Prim::CodeOf), ("builtin.chr", Prim::CharOf), ("builtin.typeof", Prim::SortOf),
    ("builtin.error", Prim::Raise), ("builtin.extern", Prim::External), ("builtin.range", Prim::Span), ("builtin.real", Prim::MakeReal),
    ("builtin.precision", Prim::Places), ("builtin.to_string", Prim::AsText), ("builtin.to_int", Prim::AsInt),
    ("builtin.to_real", Prim::AsReal), ("builtin.num", Prim::Numer), ("builtin.den", Prim::Denom), ("builtin.push", Prim::Append),
    ("builtin.get", Prim::Fetch), ("builtin.put", Prim::Replace), ("ext.builtin.echo", Prim::Tell),
    ("ext.builtin.define", Prim::Define), ("ext.builtin.var_dump", Prim::Dump), ("ext.builtin.array", Prim::Gather),
    ("ext.builtin.print_r", Prim::Portray), ("ext.builtin.unset", Prim::Erase), ("ext.builtin.isset", Prim::Standing), ("ext.builtin.empty", Prim::Hollow), ("ext.builtin.exit", Prim::Quit),
    ("ext.builtin.args.all", Prim::Handed), ("ext.builtin.args.count", Prim::HowMany),
    ("ext.builtin.args.at", Prim::HandedAt), ("ext.builtin.time_limit", Prim::Clock),
    ("ext.builtin.eval", Prim::Weigh), ("ext.builtin.include", Prim::Bring), ("ext.builtin.include.once", Prim::BringOnce),
    ("ext.builtin.output.hold", Prim::KeepOut), ("ext.builtin.output.held", Prim::KeptOut),
    ("ext.builtin.output.drop", Prim::LooseOut), ("ext.builtin.output.depth", Prim::DeepOut),
    ("ext.builtin.at_end", Prim::Afterward), ("ext.builtin.complaint.handler", Prim::Hearer), ("ext.builtin.complaint.say", Prim::Complain),
    ("ext.builtin.file.read", Prim::Slurp), ("ext.builtin.file.write", Prim::Spill),
    ("ext.builtin.file.exists", Prim::There), ("ext.builtin.file.remove", Prim::Gone),
];

const BINARY_LABELS: [(&str, Prim); 24] = [
    ("op.add", Prim::Plus), ("op.sub", Prim::Minus), ("op.mul", Prim::Times), ("op.div", Prim::Over), ("op.quot", Prim::IntDiv),
    ("op.rem", Prim::Mod), ("op.pow", Prim::Power), ("op.eq", Prim::Eq), ("op.ne", Prim::Ne), ("op.lt", Prim::Lt), ("op.le", Prim::Le),
    ("op.gt", Prim::Gt), ("op.ge", Prim::Ge), ("op.and", Prim::Both), ("op.or", Prim::Either), ("op.concat", Prim::Join),
    ("ext.op.compare", Prim::Rank), ("ext.op.bit.and", Prim::BitsBoth), ("ext.op.bit.or", Prim::BitsEither),
    ("ext.op.bit.xor", Prim::BitsOne), ("ext.op.bit.left", Prim::BitsUp), ("ext.op.bit.right", Prim::BitsDown),
    ("ext.op.identical", Prim::Selfsame), ("ext.op.not_identical", Prim::Unlike),
];

fn top_object(text: &str) -> Result<serde_json::Map<String, Json>, String> {
    match serde_json::from_str(text).map_err(|e| format!("definition is not valid JSON: {e}"))? {
        Json::Object(m) => Ok(m),
        _ => Err("definition must be a JSON object".to_string()),
    }
}

pub fn identify(text: &str) -> Result<(String, Vec<String>), String> {
    let m = top_object(text)?;
    let name = m.get("language").and_then(Json::as_str).filter(|s| !s.is_empty()).ok_or("label 'language' must be a non-empty string")?;
    let exts = m.get("extensions").and_then(Json::as_array).ok_or("label 'extensions' must be a list of strings")?;
    Ok((name.to_string(), exts.iter().filter_map(|e| e.as_str().map(str::to_string)).collect()))
}

fn strings_at(key: &str, json: &Json) -> Result<Vec<String>, String> {
    let Json::Array(items) = json else { return Err(format!("label '{key}' must be a list of non-empty strings")) };
    items
        .iter()
        .map(|i| match i.as_str() {
            Some(s) if !s.is_empty() => Ok(s.to_string()),
            _ => Err(format!("label '{key}' must be a list of non-empty strings")),
        })
        .collect()
}

impl Table {
    pub fn parse(text: &str) -> Result<Table, String> {
        let map = top_object(text)?;
        let mut cells = HashMap::new();
        let mut known = tag_shapes(TAGS);
        for (key, shape) in &known {
            let json = map.get(*key).ok_or_else(|| format!("missing label '{key}'"))?;
            cells.insert(*key, cell_of(key, *shape, json)?);
        }
        for (key, shape) in tag_shapes(EXT_TAGS) {
            let cell = match map.get(key) {
                Some(json) => cell_of(key, shape, json)?,
                None if shape == 'B' => Entry::Flag(false),
                None => Entry::Strings(Vec::new()),
            };
            cells.insert(key, cell);
            known.push((key, shape));
        }
        let mut strange: Vec<&str> = map.keys().map(String::as_str).filter(|k| !k.starts_with('$') && !known.iter().any(|(l, _)| l == k)).collect();
        strange.sort();
        if !strange.is_empty() {
            return Err(format!("unknown label(s): {}", strange.join(", ")));
        }
        let mut table = Table {
            ident: String::new(),
            blocks: Blocks::Indented,
            rpn: false,
            cells,
            dyadic: HashMap::new(),
            monadic: HashMap::new(),
            precedence: HashMap::new(),
            prims: HashMap::new(),
            compound: HashMap::new(),
            given: map.keys().cloned().collect(),
            keywords: HashSet::new(),
            signs: Vec::new(),
        };
        table.ident = table.lone("language").unwrap_or("").to_string();
        table.check()?;
        table.precedence_tables()?;
        Ok(table)
    }

    pub fn strings(&self, key: &str) -> &[String] {
        match self.cells.get(key) {
            Some(Entry::Strings(l)) => l,
            _ => panic!("'{key}' is not a list label"),
        }
    }

    pub fn single(&self, key: &str) -> Option<&str> {
        self.strings(key).first().map(String::as_str)
    }

    /// Whether the definition gave this label at all, even empty: an
    /// empty list can still say something.
    pub fn cell_present(&self, key: &str) -> bool {
        self.given.contains(key)
    }

    pub fn has_any(&self, key: &str) -> bool {
        !self.strings(key).is_empty()
    }

    pub fn spells(&self, key: &str, lexeme: &str) -> bool {
        self.strings(key).iter().any(|w| w == lexeme)
    }

    pub fn flag(&self, key: &str) -> bool {
        matches!(self.cells.get(key), Some(Entry::Flag(true)))
    }

    pub fn count(&self, key: &str) -> Option<usize> {
        match self.cells.get(key) {
            Some(Entry::Count(n)) => *n,
            _ => None,
        }
    }

    pub fn lone(&self, key: &str) -> Option<&str> {
        match self.cells.get(key) {
            Some(Entry::One(w)) => w.as_deref(),
            _ => None,
        }
    }

    fn tiers(&self) -> &[Vec<String>] {
        match self.cells.get("op.precedence") {
            Some(Entry::Tiers(t)) => t,
            _ => &[],
        }
    }

    pub fn letter(&self, key: &str) -> Option<char> {
        let mut it = self.single(key)?.chars();
        match (it.next(), it.next()) {
            (Some(c), None) => Some(c),
            _ => None,
        }
    }

    pub fn letters(&self, key: &str) -> Vec<char> {
        self.strings(key).iter().filter_map(|w| if w.chars().count() == 1 { w.chars().next() } else { None }).collect()
    }

    pub fn banner(&self) -> String {
        let mut it = self.ident.chars();
        it.next().map_or("Error".to_string(), |c| format!("{}{}Error", c.to_uppercase(), it.as_str()))
    }

    pub fn begins_name(&self, c: char) -> bool {
        c == '_' || if self.flag("identifier.unicode") { c.is_alphabetic() } else { c.is_ascii_alphabetic() }
    }

    pub fn extends_name(&self, c: char) -> bool {
        c == '_' || if self.flag("identifier.unicode") { c.is_alphanumeric() } else { c.is_ascii_alphanumeric() }
    }

    pub fn name_like(&self, s: &str) -> bool {
        let mut it = s.chars();
        let mut first = it.next();
        if first.is_some() && first == self.letter("identifier.variable_prefix") {
            first = it.next();
        }
        first.map_or(false, |c| self.begins_name(c)) && it.all(|c| self.extends_name(c))
    }

    fn check(&mut self) -> Result<(), String> {
        if self.count("format_version") != Some(1) {
            return Err("format_version must be 1".to_string());
        }
        let singles = ["lexical.string_quotes", "lexical.raw_quotes", "lexical.string_escapes", "lexical.name_quote",
            "lexical.number.decimal_point", "lexical.number.base_marker", "lexical.number.exponent_marker", "identifier.variable_prefix",
            "ext.lexical.number.exponent", "ext.lexical.interpolating_quotes"];
        for key in singles {
            if let Some(w) = self.strings(key).iter().find(|w| w.chars().count() != 1) {
                return Err(format!("label '{key}' takes single characters, got '{w}'"));
            }
        }
        let woven = self.strings("ext.lexical.interpolating_quotes");
        if let Some(q) = woven.iter().find(|q| !self.spells("lexical.string_quotes", q)) {
            return Err(format!("ext.lexical.interpolating_quotes '{q}' is not among lexical.string_quotes"));
        }
        if !woven.is_empty() && (!self.has_any("syntax.group.open") || !self.has_any("op.concat")) {
            return Err("ext.lexical.interpolating_quotes needs syntax.group and op.concat".to_string());
        }
        if self.flag("ext.syntax.call.bare") && !self.has_any("syntax.call.open") {
            return Err("ext.syntax.call.bare needs syntax.call".to_string());
        }
        if self.strings("lexical.comment_block.open").len() != self.strings("lexical.comment_block.close").len() {
            return Err("lexical.comment_block.open and .close must pair up position by position".to_string());
        }
        if let Some(p) = self.single("lexical.number.hex_prefix") {
            if p.chars().count() != 2 || !p.starts_with(|c: char| c.is_ascii_digit()) {
                return Err(format!("lexical.number.hex_prefix must be a digit followed by one letter, got '{p}'"));
            }
        }
        self.blocks = match self.lone("block.style") {
            Some("indentation") => Blocks::Indented,
            Some("braces") => Blocks::Bracketed,
            Some("keyword") => Blocks::Worded,
            other => return Err(format!("block.style must be 'indentation', 'braces' or 'keyword', got '{}'", other.unwrap_or(""))),
        };
        self.rpn = match self.lone("syntax.notation") {
            Some("infix") => false,
            Some("postfix") => true,
            other => return Err(format!("syntax.notation must be 'infix' or 'postfix', got '{}'", other.unwrap_or(""))),
        };
        let (o, c) = (self.strings("block.open").len(), self.strings("block.close").len());
        let fits = match self.blocks {
            Blocks::Indented => o == 0 && c == 0 && self.count("block.indent_size").map_or(false, |n| n > 0),
            Blocks::Bracketed => o > 0 && o == c,
            Blocks::Worded => o == 0 && c > 0,
        };
        if !fits {
            return Err("block.open, block.close and block.indent_size do not fit block.style".to_string());
        }
        let postfix = self.rpn;
        if postfix && !self.tiers().is_empty() {
            return Err("a postfix language takes no op.precedence".to_string());
        }
        let stack_labels = ["stack.dup", "stack.drop", "stack.swap", "stack.over", "stack.rot", "stack.eval", "stack.program.open", "stack.program.close"];
        if !postfix && stack_labels.iter().any(|l| self.has_any(l)) {
            return Err("the stack.* labels need syntax.notation 'postfix'".to_string());
        }
        for (a, b) in [("syntax.group.open", "syntax.group.close"), ("syntax.call.open", "syntax.call.close"),
                       ("syntax.array.open", "syntax.array.close"), ("op.index.open", "op.index.close"),
                       ("stack.program.open", "stack.program.close")] {
            if self.strings(a).len() != self.strings(b).len() && (self.strings(a).is_empty() || self.strings(b).is_empty()) {
                return Err(format!("labels '{a}' and '{b}' must be given together"));
            }
        }
        if self.has_any("syntax.call.label") && !self.has_any("syntax.call.open") {
            return Err("syntax.call.label needs syntax.call.open".to_string());
        }
        if self.flag("op.index.strings") && !self.has_any("op.index.open") {
            return Err("op.index.strings needs op.index.open".to_string());
        }
        if !matches!(self.lone("op.div.result"), None | Some("rational") | Some("real") | Some("whole_or_real")) {
            return Err("op.div.result must be 'rational', 'real', 'whole_or_real' or null".to_string());
        }
        if !self.has_any("stmt.let") && (self.has_any("stmt.let.mutable") || self.has_any("stmt.let.annotation") || self.flag("stmt.let.type_first")) {
            return Err("stmt.let.mutable, stmt.let.annotation and stmt.let.type_first need stmt.let".to_string());
        }
        if self.flag("stmt.let.type_first") {
            for l in ["stmt.let.mutable", "stmt.let.annotation", "stmt.function", "stmt.function.returns"] {
                if self.has_any(l) {
                    return Err(format!("stmt.let.type_first leaves no place for {l}; leave it empty"));
                }
            }
        }
        if !self.has_any("stmt.if") && (self.has_any("stmt.elif") || self.has_any("stmt.else")) {
            return Err("stmt.elif and stmt.else need stmt.if".to_string());
        }
        if self.has_any("stmt.for") != self.has_any("stmt.for.in") && !(postfix && !self.has_any("stmt.for.in")) {
            return Err("stmt.for and stmt.for.in must be given together".to_string());
        }
        if self.flag("stmt.function.result_by_name") && !self.has_any("stmt.function") {
            return Err("stmt.function.result_by_name needs stmt.function".to_string());
        }
        let keywords = ["stmt.let", "stmt.let.mutable", "stmt.if", "stmt.elif", "stmt.else", "stmt.while", "stmt.until", "stmt.for",
            "stmt.for.in", "stmt.return", "stmt.break", "stmt.continue", "stmt.function", "stmt.pass", "literal.true", "literal.false", "literal.null"];
        for key in keywords {
            if let Some(w) = self.strings(key).iter().find(|w| !self.name_like(w)) {
                return Err(format!("keyword '{w}' must be shaped like an identifier"));
            }
        }
        for key in ["system.args", "system.memoization", "system.real_default_precision", "system.entry", "system.kind.integer",
                    "system.kind.rational", "system.kind.real", "system.kind.string", "system.kind.boolean", "system.kind.array", "system.kind.null"] {
            if let Some(w) = self.single(key).filter(|w| !self.name_like(w)) {
                return Err(format!("system name '{w}' must be shaped like an identifier"));
            }
        }
        Ok(())
    }

    fn precedence_tables(&mut self) -> Result<(), String> {
        let postfix = self.rpn;
        let table = self.tiers().to_vec();
        let place = |lex: &str, last: bool| -> Option<u32> {
            if postfix {
                return Some(1);
            }
            let at = if last { table.iter().rposition(|t| t.contains(&lex.to_string())) } else { table.iter().position(|t| t.contains(&lex.to_string())) };
            at.map(|i| i as u32 + 1)
        };
        let real_division = matches!(self.lone("op.div.result"), Some("real") | Some("whole_or_real"));
        for (label, op) in BINARY_LABELS {
            let op = if label == "op.div" && real_division { Prim::OverReal } else { op };
            for lex in self.strings(label).to_vec() {
                let tier = place(&lex, false).ok_or_else(|| format!("'{lex}' ({label}) does not appear in op.precedence"))?;
                let right = self.spells("op.right_associative", &lex);
                if self.dyadic.insert(lex.clone(), Infix { prim: op, level: tier, right_assoc: right }).is_some() {
                    return Err(format!("'{lex}' is listed under two binary operator labels"));
                }
            }
        }
        for (label, op) in [("op.not", Prim::Invert), ("op.negate", Prim::Negate), ("ext.op.bit.not", Prim::BitsOver)] {
            for lex in self.strings(label).to_vec() {
                let tier = place(&lex, true).ok_or_else(|| format!("'{lex}' ({label}) does not appear in op.precedence"))?;
                if self.monadic.insert(lex.clone(), Infix { prim: op, level: tier, right_assoc: false }).is_some() {
                    return Err(format!("'{lex}' is listed under two unary operator labels"));
                }
            }
        }
        if self.flag("ext.op.assign.compound") {
            // An operator directly followed by the assignment sign, where
            // that is not itself an operator (`<=`) and the operator does
            // not end in the sign (`==`, so `===` stays an operator).
            for assign in self.strings("stmt.assign").to_vec() {
                for (lex, op) in &self.dyadic {
                    let spelled = format!("{lex}{assign}");
                    if !lex.ends_with(assign.as_str()) && !self.dyadic.contains_key(&spelled) && !self.monadic.contains_key(&spelled) {
                        self.compound.insert(spelled, op.prim);
                    }
                }
            }
        }
        match self.strings("ext.op.ternary").len() {
            0 | 2 => {}
            _ => return Err("ext.op.ternary takes exactly two signs, the question and the mark".to_string()),
        }
        if self.has_any("ext.stmt.switch") && (!self.has_any("ext.stmt.case") || !self.has_any("ext.stmt.case.mark")) {
            return Err("ext.stmt.switch needs ext.stmt.case and ext.stmt.case.mark".to_string());
        }
        if self.has_any("ext.stmt.class") && (!self.has_any("ext.op.member") || !self.has_any("ext.stmt.class.new")) {
            return Err("ext.stmt.class needs ext.op.member and ext.stmt.class.new".to_string());
        }
        if self.has_any("stmt.foreach") && !self.has_any("stmt.foreach.as") {
            return Err("stmt.foreach needs stmt.foreach.as".to_string());
        }
        if self.has_any("stmt.foreach.pair") && !self.has_any("syntax.map.pair") {
            return Err("stmt.foreach.pair needs syntax.map.pair".to_string());
        }
        for label in ["op.range", "op.pipe", "ext.op.hush"] {
            for lex in self.strings(label).to_vec() {
                let tier = place(&lex, false).ok_or_else(|| format!("'{lex}' ({label}) does not appear in op.precedence"))?;
                self.precedence.insert(lex, tier);
            }
        }
        for lex in table.iter().flatten() {
            if !self.dyadic.contains_key(lex) && !self.monadic.contains_key(lex) && !self.precedence.contains_key(lex) {
                return Err(format!("op.precedence lists '{lex}', which is under no operator label"));
            }
        }
        for lex in self.strings("op.right_associative") {
            if !self.dyadic.contains_key(lex) {
                return Err(format!("op.right_associative lists '{lex}', which is not a binary operator"));
            }
        }
        let quotes = self.letters("lexical.string_quotes");
        for (label, op) in BUILTIN_LABELS {
            for lex in self.strings(label).to_vec() {
                let fine = lex.starts_with(|c: char| c == '_' || c.is_alphabetic()) && !lex.chars().any(|c| c.is_whitespace() || quotes.contains(&c));
                if !fine {
                    return Err(format!("builtin name '{lex}' must begin like an identifier and hold no spaces or quotes"));
                }
                if self.prims.insert(lex, op).is_some() {
                    return Err(format!("a builtin name is listed under two builtin labels"));
                }
            }
        }
        let mut all: Vec<String> = self.dyadic.keys().chain(self.monadic.keys()).chain(self.precedence.keys()).chain(self.compound.keys()).cloned().collect();
        let symbol_labels = ["syntax.group.open", "syntax.group.close", "syntax.call.open", "syntax.call.separator", "syntax.call.close",
            "syntax.call.label", "syntax.array.open", "syntax.array.separator", "syntax.array.close", "op.index.open", "op.index.close",
            "block.intro", "stmt.assign", "stmt.terminator", "stmt.let.annotation", "stmt.function.returns", "stack.dup", "stack.drop",
            "stack.swap", "stack.over", "stack.rot", "stack.eval", "stack.program.open", "stack.program.close", "stmt.let",
            "stmt.let.mutable", "stmt.if", "stmt.elif", "stmt.else", "stmt.while", "stmt.until", "stmt.for", "stmt.for.in",
            "stmt.return", "stmt.break", "stmt.continue", "stmt.function", "stmt.pass", "literal.true", "literal.false", "literal.null",
            "ext.op.increment", "ext.op.decrement", "ext.stmt.case.mark", "ext.op.ternary", "ext.stmt.for.c", "ext.stmt.static",
            "ext.stmt.global", "ext.stmt.const", "ext.stmt.switch", "ext.stmt.case", "ext.stmt.default", "ext.op.plus",
            "syntax.map.open", "syntax.map.separator", "syntax.map.pair", "syntax.map.close", "stmt.foreach", "stmt.foreach.as",
            "ext.stmt.function.returns", "ext.op.member", "ext.op.scope", "ext.stmt.class", "ext.stmt.class.extends",
            "ext.stmt.class.new", "ext.stmt.class.modifier", "ext.stmt.class.shared", "ext.op.instanceof",
            "ext.stmt.class.parent", "ext.stmt.class.self", "ext.stmt.class.interface", "ext.stmt.class.implements",
            "ext.stmt.try", "ext.stmt.catch", "ext.stmt.finally",
            "ext.stmt.throw", "ext.stmt.catch.separator", "ext.op.reference", "ext.op.otherwise", "ext.op.hush", "ext.op.name_by_value", "ext.stmt.unpack"];
        for key in symbol_labels {
            all.extend(self.strings(key).iter().cloned());
        }
        if self.blocks != Blocks::Indented {
            all.extend(self.strings("block.open").iter().cloned());
            all.extend(self.strings("block.close").iter().cloned());
        }
        let fold = self.flag("lexical.keywords_case_insensitive");
        for lex in all {
            if self.name_like(&lex) {
                self.keywords.insert(if fold { lex.to_lowercase() } else { lex });
            } else if !self.signs.contains(&lex) {
                self.signs.push(lex);
            }
        }
        self.signs.sort_by(|a, b| b.len().cmp(&a.len()).then(a.cmp(b)));
        Ok(())
    }
}
