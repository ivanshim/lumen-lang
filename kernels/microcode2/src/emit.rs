// The emitter: the tree written back out in a language.
//
// The same definition that told the reducer how a language is spelled
// tells the emitter how to spell it: every keyword, operator, bracket and
// builtin name is looked up by label, precedence decides where brackets
// go, and the block style decides the layout. A construct the target has
// no spelling for makes the whole program unwritable, and the reason is
// reported. What comes out is the program as the definition describes the
// language, which the kernels accept; where a real compiler would want
// type words the definitions do not carry, none are invented.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Zero};

use crate::reduce::Globals;
use crate::spec::{Spec, Style, BINARY, NATIVES};
use crate::tree::{Callee, Exit, Form, Native, Node, Op, Program};
use crate::value::{real_text, Value};

type Text = Result<String, String>;

/// An expression's text and the tier of its outermost operator, if any.
struct Piece {
    text: String,
    tier: Option<u32>,
}

impl Piece {
    fn atom(text: String) -> Piece {
        Piece { text, tier: None }
    }
}

struct Writer<'a> {
    from: &'a Spec,
    to: &'a Spec,
    lines: Vec<String>,
    /// Names declared so far in each open program, for languages that declare.
    declared: Vec<HashSet<String>>,
    /// How often each name is assigned in the open program.
    assigned: Vec<HashMap<String, usize>>,
    /// Hidden names given a spellable form.
    renamed: HashMap<String, String>,
    temps: usize,
    /// Functions provided by the target's own library, whose definitions are not written.
    provided: HashSet<String>,
}

pub fn emit(program: &Program, _globals: &Globals, from: &Spec, to: &Spec) -> Text {
    let mut w = Writer {
        from,
        to,
        lines: Vec::new(),
        declared: vec![HashSet::new()],
        assigned: vec![HashMap::new()],
        renamed: HashMap::new(),
        temps: 0,
        provided: HashSet::new(),
    };
    w.program(&program.body)?;
    let mut text = w.lines.join("\n");
    text.push('\n');
    if let Some(prologue) = to.first("lexical.prologue") {
        // An import prologue only when the module is used; a marker always.
        let module = prologue.strip_prefix("import ").map(|m| m.trim().to_string());
        if module.map_or(true, |m| text.contains(&format!("{}.", m))) {
            text = format!("{}\n{}", prologue, text);
        }
    }
    Ok(text)
}

// ---------- reachability

/// The names of the functions the body reaches, transitively; a function
/// the target provides is not entered.
fn reachable(body: &[Node], functions: &HashMap<String, &Node>, provided: &HashSet<String>) -> HashSet<String> {
    let mut seen = HashSet::new();
    let mut todo: Vec<&Node> = body.iter().collect();
    while let Some(node) = todo.pop() {
        for name in called_names(node) {
            if seen.insert(name.clone()) && !provided.contains(&name) {
                if let Some(def) = functions.get(&name) {
                    todo.push(def);
                }
            }
        }
    }
    seen
}

fn called_names(node: &Node) -> Vec<String> {
    let mut out = Vec::new();
    walk(node, &mut |n| {
        if let Form::Call { callee: Callee::Named(slot), .. } = &n.form {
            out.push(slot.name.to_string());
        }
        if let Form::Load(slot) = &n.form {
            out.push(slot.name.to_string());
        }
    });
    out
}

fn walk<'n>(node: &'n Node, f: &mut dyn FnMut(&'n Node)) {
    f(node);
    match &node.form {
        Form::Sequence(items) => items.iter().for_each(|n| walk(n, f)),
        Form::Scope { body, .. } => walk(body, f),
        Form::Branch { test, then, otherwise } => {
            walk(test, f);
            walk(then, f);
            if let Some(o) = otherwise {
                walk(o, f);
            }
        }
        Form::Loop { test, body, step } => {
            walk(test, f);
            walk(body, f);
            if let Some(s) = step {
                walk(s, f);
            }
        }
        Form::Assign { value, .. } => walk(value, f),
        Form::AssignIndex { index, value, .. } => {
            walk(index, f);
            walk(value, f);
        }
        Form::Call { callee, args } => {
            if let Callee::Value(v) = callee {
                walk(v, f);
            }
            args.iter().for_each(|n| walk(n, f));
        }
        Form::Operate { args, .. } => args.iter().for_each(|n| walk(n, f)),
        Form::Leave { value, .. } => {
            if let Some(v) = value {
                walk(v, f);
            }
        }
        Form::Literal(Value::Routine(p)) => walk(&p.body, f),
        Form::Literal(_) | Form::Load(_) => {}
    }
}

fn function_def(node: &Node) -> Option<(&str, &Rc<Program>)> {
    match &node.form {
        Form::Assign { to, value } => match &value.form {
            Form::Literal(Value::Routine(p)) => Some((&to.name, p)),
            _ => None,
        },
        _ => None,
    }
}

/// The library label a source function provides, and how the target spells it.
enum Provided {
    Native(Native, String),
    Library,
    Absent,
}

impl<'a> Writer<'a> {
    // ---------- names

    fn spell(&mut self, name: &str, function: bool) -> Text {
        let name = match self.from.glyph("identifier.variable_prefix") {
            Some(p) => name.strip_prefix(p).unwrap_or(name),
            None => name,
        };
        let mut out = if let Some(hidden) = name.strip_prefix('#') {
            match self.renamed.get(name) {
                Some(n) => n.clone(),
                None => {
                    self.temps += 1;
                    let fresh = format!("{}{}", hidden.trim_end_matches(char::is_numeric), self.temps);
                    self.renamed.insert(name.to_string(), fresh.clone());
                    fresh
                }
            }
        } else {
            name.to_string()
        };
        if !self.to.flag("identifier.unicode") && !out.is_ascii() {
            return Err("an identifier beyond ASCII".to_string());
        }
        let clashes = self.to.reserved.contains(&out)
            || self.to.natives.contains_key(&out)
            || (self.to.flag("lexical.keywords_case_insensitive") && self.to.reserved.contains(&out.to_lowercase()));
        if clashes {
            out.push('_');
        }
        if !function {
            if let Some(prefix) = self.to.glyph("identifier.variable_prefix") {
                out.insert(0, prefix);
            }
        }
        Ok(out)
    }

    /// A load of a source system name (ARGS, INTEGER) is the target's word for it.
    fn system_name(&self, name: &str) -> Option<Result<String, String>> {
        let labels = [
            "system.args", "system.memoization", "system.real_default_precision",
            "system.kind.integer", "system.kind.rational", "system.kind.real", "system.kind.string",
            "system.kind.boolean", "system.kind.array", "system.kind.null",
        ];
        for label in labels {
            if self.from.spells(label, name) {
                return Some(self.to.first(label).map(str::to_string).ok_or_else(|| format!("no `{}`", name)));
            }
        }
        None
    }

    fn lexeme(&self, label: &str, what: &str) -> Text {
        self.to.first(label).map(str::to_string).ok_or_else(|| format!("no `{}`", what))
    }

    fn label_of(native: Native) -> &'static str {
        NATIVES.iter().find(|(_, n)| *n == native).map(|(l, _)| *l).unwrap()
    }

    /// The target's name for a builtin: its own, or the library function
    /// its `$library` says provides it.
    fn native_name(&self, native: Native) -> Text {
        let label = Self::label_of(native);
        if let Some(lex) = self.to.first(label) {
            return Ok(lex.to_string());
        }
        if let Some((function, _)) = self.to.library.iter().find(|(_, l)| l.as_str() == label) {
            return Ok(function.clone());
        }
        Err(format!("no `{}`", label.trim_start_matches("builtin.")))
    }

    fn op_label(op: Op) -> &'static str {
        match op {
            Op::Div | Op::DivReal => "op.div",
            Op::Not => "op.not",
            Op::Neg => "op.negate",
            other => BINARY.iter().find(|(_, o)| *o == other).map(|(l, _)| *l).unwrap_or("op.index"),
        }
    }

    /// The target's lexeme for an operation, with its tier.
    fn op_lexeme(&self, op: Op) -> Result<(String, u32), String> {
        let label = Self::op_label(op);
        let lex = self.to.first(label).ok_or_else(|| format!("no `{}`", label))?;
        if matches!(op, Op::Div | Op::DivReal) {
            let real = self.to.text("op.div.result") == Some("real");
            if real != (op == Op::DivReal) {
                return Err(if real { "exact division".to_string() } else { "division yielding a real".to_string() });
            }
        }
        let table = if matches!(op, Op::Not | Op::Neg) { &self.to.prefix } else { &self.to.infix };
        let tier = table.get(lex).map(|o| o.tier).ok_or_else(|| format!("no `{}`", label))?;
        Ok((lex.to_string(), tier))
    }

    /// What the target does about a call to a source library function.
    fn provided(&self, name: &str) -> Provided {
        let Some(label) = self.from.library.get(name) else { return Provided::Absent };
        if let Some(lex) = self.to.first(label) {
            if let Some(native) = self.to.natives.get(lex) {
                return Provided::Native(*native, lex.to_string());
            }
        }
        if self.to.library.values().any(|l| l == label) {
            return Provided::Library;
        }
        Provided::Absent
    }

    // ---------- layout

    fn indent(depth: usize) -> String {
        "    ".repeat(depth)
    }

    fn terminated(&self, text: String) -> String {
        match self.to.first("stmt.terminator") {
            Some(t) if self.to.style == Style::Brace && !text.ends_with(t) => format!("{}{}", text, t),
            _ => text,
        }
    }

    fn line(&mut self, depth: usize, text: String) {
        self.lines.push(format!("{}{}", Self::indent(depth), text));
    }

    fn statement_line(&mut self, depth: usize, text: String) {
        let text = self.terminated(text);
        self.line(depth, text);
    }

    /// `head` and a body in the target's layout; `intro` names a
    /// block-intro word to use if the target has it. Returns the closer to
    /// write, if the style has one and the caller wants to continue the line.
    fn block(&mut self, depth: usize, head: String, body: &Node, intro: Option<&str>) -> Result<Option<String>, String> {
        let to = self.to;
        match to.style {
            Style::Indent => {
                let mark = to.first("block.intro").unwrap_or("");
                self.line(depth, format!("{}{}", head, mark));
                let before = self.lines.len();
                self.body(depth + 1, body)?;
                if self.lines.len() == before {
                    let pass = self.lexeme("stmt.pass", "an empty block")?;
                    self.line(depth + 1, pass);
                }
                Ok(None)
            }
            Style::Brace => {
                let open = to.first("block.open").unwrap();
                let close = to.first("block.close").unwrap();
                let intro = intro.filter(|i| to.spells("block.intro", i)).map(|i| format!(" {}", i)).unwrap_or_default();
                self.line(depth, format!("{}{} {}", head, intro, open));
                self.body(depth + 1, body)?;
                Ok(Some(close.to_string()))
            }
            Style::Keyword => {
                let intro = intro.filter(|i| to.spells("block.intro", i)).map(|i| format!(" {}", i)).unwrap_or_default();
                self.line(depth, format!("{}{}", head, intro));
                self.body(depth + 1, body)?;
                Ok(Some(to.first("block.close").unwrap().to_string()))
            }
            Style::Postfix => {
                self.line(depth, head);
                self.body(depth + 1, body)?;
                Ok(Some(to.first("block.close").unwrap().to_string()))
            }
        }
    }

    /// The word after a block's closer: Pascal's `end;`.
    fn closer_tail(&self) -> String {
        let closer = self.to.first("block.close").unwrap_or("");
        match self.to.first("stmt.terminator") {
            Some(t) if self.to.style == Style::Brace && closer.chars().all(|c| c.is_alphabetic()) => t.to_string(),
            _ => String::new(),
        }
    }

    fn close(&mut self, depth: usize, closer: Option<String>) {
        if let Some(c) = closer {
            let tail = self.closer_tail();
            self.line(depth, format!("{}{}", c, tail));
        }
    }

    // ---------- the program

    fn program(&mut self, body: &Node) -> Result<(), String> {
        let to = self.to;
        if let Some(mark) = to.first("lexical.comment_line") {
            self.lines.push(format!("{} Written by the microcode2 kernel from a {} program; edit the original.", mark, self.from.name));
        }
        let items: Vec<&Node> = match &body.form {
            Form::Sequence(items) => items.iter().collect(),
            _ => vec![body],
        };
        let mut functions: HashMap<String, &Node> = HashMap::new();
        let mut rest: Vec<&Node> = Vec::new();
        // A source entry function (C's main) runs after the top level: its
        // body joins the top level here.
        let source_entry = self.from.first("system.entry").map(str::to_string);
        let mut entry_body: Option<&Node> = None;
        for item in &items {
            match function_def(item) {
                Some((name, program)) if Some(name) == source_entry.as_deref() => {
                    entry_body = Some(&program.body);
                }
                Some((name, _)) => {
                    functions.insert(name.to_string(), item);
                }
                None => rest.push(item),
            }
        }
        if let Some(body) = entry_body {
            match &body.form {
                Form::Sequence(items) => rest.extend(items.iter()),
                _ => rest.push(body),
            }
            // C's `return 0` at the end of main has no meaning at the top level.
            if matches!(rest.last().map(|n| &n.form), Some(Form::Leave { how: Exit::Return, .. })) {
                rest.pop();
            }
        }
        // Only the functions the program reaches are written, and none the
        // target's own library provides.
        for name in functions.keys() {
            if !matches!(self.provided(name), Provided::Absent) {
                self.provided.insert(name.clone());
            }
        }
        let bodies: Vec<Node> = rest.iter().map(|n| (*n).clone_shallow()).collect();
        let mut reached = reachable(&bodies, &functions, &self.provided);
        for name in &self.provided {
            reached.remove(name);
        }
        let mut wanted: Vec<&Node> = items.iter().copied().filter(|n| function_def(n).map_or(false, |(name, _)| reached.contains(name))).collect();
        // A top-level constant nothing reachable reads (a library's tables) is left out.
        let mut used: HashSet<String> = HashSet::new();
        for node in rest.iter().copied().chain(wanted.iter().copied()) {
            walk(node, &mut |n| {
                if let Form::Load(slot) = &n.form {
                    used.insert(slot.name.to_string());
                }
            });
        }
        let rest: Vec<&Node> = rest
            .into_iter()
            .filter(|n| match &n.form {
                Form::Assign { to, value } if matches!(value.form, Form::Literal(_)) => used.contains(&*to.name),
                _ => true,
            })
            .collect();
        // Every assignment count for `let` decisions at the top level.
        self.assigned[0] = count_assignments(&rest);
        let entry = to.first("system.entry").map(str::to_string);
        let pascal_like = to.style == Style::Brace
            && to.first("block.open").map_or(false, |o| o.chars().all(char::is_alphabetic))
            && to.spells("stmt.terminator", ".");
        if let Some(main) = entry {
            for f in wanted.drain(..) {
                self.statement(0, f)?;
                self.lines.push(String::new());
            }
            let body = Node::seq(body.line, rest.iter().map(|n| n.clone_shallow()).collect());
            let main_program = Program { name: main.clone(), params: Vec::new(), param_slots: Vec::new(), slot_names: Vec::new(), body };
            self.function(0, &main, &main_program, true)?;
            return Ok(());
        }
        for f in wanted.drain(..) {
            self.statement(0, f)?;
            self.lines.push(String::new());
        }
        if pascal_like {
            let open = to.first("block.open").unwrap().to_string();
            self.line(0, open);
            for item in &rest {
                self.statement(1, item)?;
            }
            let close = to.first("block.close").unwrap().to_string();
            self.line(0, format!("{}.", close));
        } else {
            for item in &rest {
                self.statement(0, item)?;
            }
        }
        Ok(())
    }

    fn body(&mut self, depth: usize, body: &Node) -> Result<(), String> {
        match &body.form {
            Form::Sequence(items) => {
                let mut i = 0;
                while i < items.len() {
                    if let Some(n) = self.counted_loop(depth, &items[i..])? {
                        i += n;
                        continue;
                    }
                    self.statement(depth, &items[i])?;
                    i += 1;
                }
                Ok(())
            }
            _ => self.statement(depth, body),
        }
    }

    // ---------- statements

    fn statement(&mut self, depth: usize, node: &Node) -> Result<(), String> {
        let to = self.to;
        match &node.form {
            Form::Sequence(_) => self.body(depth, node),
            Form::Scope { body, .. } => {
                if to.style == Style::Brace {
                    let open = to.first("block.open").unwrap().to_string();
                    let close = to.first("block.close").unwrap().to_string();
                    self.line(depth, open);
                    self.body(depth + 1, body)?;
                    self.line(depth, close);
                    Ok(())
                } else {
                    self.body(depth, body)
                }
            }
            Form::Branch { .. } => self.branch(depth, node, None),
            Form::Loop { test, body, step } => {
                if let Some(cond) = until_condition(test, step) {
                    return self.until(depth, cond, body);
                }
                if step.is_some() {
                    return Err("a loop with a step outside a for".to_string());
                }
                if to.style == Style::Postfix {
                    let word = self.lexeme("stmt.while", "while")?;
                    let cond = self.postfix_expr(test)?;
                    let intro = to.first("block.intro").unwrap_or("repeat").to_string();
                    let closer = self.block(depth, format!("{} {} {}", word, cond, intro), body, None)?;
                    self.close(depth, closer);
                    return Ok(());
                }
                let word = self.lexeme("stmt.while", "while")?;
                let cond = self.condition(test)?;
                let closer = self.block(depth, format!("{} {}", word, cond), body, Some("do"))?;
                self.close(depth, closer);
                Ok(())
            }
            Form::Assign { to: slot, value } => {
                if let Form::Literal(Value::Routine(p)) = &value.form {
                    return self.function(depth, &slot.name, p, false);
                }
                if let Some(result) = self.system_name(&slot.name) {
                    let name = result?;
                    let text = self.expr_text(value)?;
                    return Ok(self.statement_line(depth, self.assignment(name, text)?));
                }
                let text = self.expr_text(value)?;
                let assignment = self.binding(&slot.name, text)?;
                self.statement_line(depth, assignment);
                Ok(())
            }
            Form::AssignIndex { to: slot, index, value } => {
                let name = self.spell(&slot.name, false)?;
                if to.style == Style::Postfix {
                    let i = self.postfix_expr(index)?;
                    let v = self.postfix_expr(value)?;
                    let put = self.native_name(Native::Put)?;
                    let q = self.to.first("lexical.name_quote").unwrap_or("'");
                    self.line(depth, format!("{} {} {}{}{} {}", i, v, q, name, q, put));
                    return Ok(());
                }
                let i = self.expr_text(index)?;
                let v = self.expr_text(value)?;
                let assign = self.lexeme("stmt.assign", "assignment")?;
                if let (Some(open), Some(close)) = (to.first("op.index.open"), to.first("op.index.close")) {
                    self.statement_line(depth, format!("{}{}{}{} {} {}", name, open, i, close, assign, v));
                } else if let Ok(put) = self.native_name(Native::Put) {
                    let text = self.call_text(&put, vec![Piece::atom(name), Piece::atom(i), Piece::atom(v)]);
                    self.statement_line(depth, text);
                } else {
                    return Err("indexed assignment".to_string());
                }
                Ok(())
            }
            Form::Leave { how, value } => {
                let label = match how {
                    Exit::Return => "stmt.return",
                    Exit::Break => "stmt.break",
                    Exit::Continue => "stmt.continue",
                };
                let word = self.lexeme(label, label.trim_start_matches("stmt."))?;
                if to.style == Style::Postfix {
                    let text = match value {
                        // A returned call that leaves nothing returns null after it.
                        Some(v) if !self.postfix_yields(v) => {
                            format!("{} {} {}", self.postfix_expr(v)?, self.lexeme("literal.null", "null")?, word)
                        }
                        Some(v) => format!("{} {}", self.postfix_expr(v)?, word),
                        None if *how == Exit::Return => format!("{} {}", self.lexeme("literal.null", "null")?, word),
                        None => word,
                    };
                    self.line(depth, text);
                    return Ok(());
                }
                let text = match value {
                    Some(v) => {
                        let v = self.expr_text(v)?;
                        if to.flag("stmt.function.result_by_name") { format!("{}({})", word, v) } else { format!("{} {}", word, v) }
                    }
                    None => word,
                };
                self.statement_line(depth, text);
                Ok(())
            }
            Form::Call { .. } | Form::Operate { .. } | Form::Literal(_) | Form::Load(_) => {
                if to.style == Style::Postfix {
                    let text = self.postfix_expr(node)?;
                    let text = if self.postfix_yields(node) { format!("{} {}", text, self.lexeme("stack.drop", "drop")?) } else { text };
                    self.line(depth, text);
                    return Ok(());
                }
                let text = self.expr_text(node)?;
                self.statement_line(depth, text);
                Ok(())
            }
        }
    }

    /// `if` with its chain of `elif` and `else`.
    fn branch(&mut self, depth: usize, node: &Node, head_word: Option<String>) -> Result<(), String> {
        let to = self.to;
        let Form::Branch { test, then, otherwise } = &node.form else { unreachable!() };
        if to.style == Style::Postfix {
            let cond = self.postfix_expr(test)?;
            let word = self.lexeme("stmt.if", "if")?;
            self.line(depth, format!("{} {}", cond, word));
            self.body(depth + 1, then)?;
            if let Some(o) = otherwise {
                let else_word = self.lexeme("stmt.else", "else")?;
                self.line(depth, else_word);
                self.body(depth + 1, o)?;
            }
            let close = to.first("block.close").unwrap().to_string();
            self.line(depth, close);
            return Ok(());
        }
        let word = match head_word {
            Some(w) => w,
            None => self.lexeme("stmt.if", "if")?,
        };
        let cond = self.condition(test)?;
        let closer = self.block(depth, format!("{} {}", word, cond), then, Some("then"))?;
        let Some(o) = otherwise else {
            self.close(depth, closer);
            return Ok(());
        };
        let else_word = self.lexeme("stmt.else", "else")?;
        let chained = matches!(o.form, Form::Branch { .. });
        if chained {
            let head = match to.first("stmt.elif") {
                Some(elif) => elif.to_string(),
                None => format!("{} {}", else_word, self.lexeme("stmt.if", "if")?),
            };
            match to.style {
                Style::Indent => self.branch(depth, o, Some(head)),
                Style::Keyword => {
                    // One closer ends the whole chain.
                    let mark = self.lines.len();
                    self.branch(depth, o, Some(head))?;
                    let _ = mark;
                    Ok(())
                }
                _ => {
                    // `} else if cond {` on one line.
                    let closer = closer.unwrap();
                    let mark = self.lines.len();
                    self.branch(depth, o, Some(format!("{} {}", closer, head)))?;
                    let _ = mark;
                    Ok(())
                }
            }
        } else {
            match to.style {
                Style::Indent => {
                    let _ = self.block(depth, else_word, o, None)?;
                    Ok(())
                }
                Style::Keyword => {
                    let closer = self.block(depth, else_word, o, None)?;
                    self.close(depth, closer);
                    Ok(())
                }
                _ => {
                    let closer = closer.unwrap();
                    let closer = self.block(depth, format!("{} {}", closer, else_word), o, None)?;
                    self.close(depth, closer);
                    Ok(())
                }
            }
        }
    }

    /// C-family languages bracket their conditions.
    fn condition(&mut self, test: &Node) -> Text {
        let text = self.expr_text(test)?;
        let bracketed = self.to.style == Style::Brace && self.to.first("block.open") == Some("{");
        match (bracketed, self.to.first("syntax.group.open"), self.to.first("syntax.group.close")) {
            (true, Some(o), Some(c)) if !text.starts_with(o) => Ok(format!("{}{}{}", o, text, c)),
            _ => Ok(text),
        }
    }

    /// `until cond body`, or a loop that breaks on the condition.
    fn until(&mut self, depth: usize, cond: &Node, body: &Node) -> Result<(), String> {
        let to = self.to;
        if let Some(word) = to.first("stmt.until").map(str::to_string) {
            if to.style == Style::Postfix {
                self.line(depth, word);
                self.body(depth + 1, body)?;
                let intro = to.words("block.intro").get(1).cloned().unwrap_or_else(|| "until".to_string());
                let cond = self.postfix_expr(cond)?;
                let close = to.first("block.close").unwrap().to_string();
                self.line(depth, format!("{} {} {}", intro, cond, close));
                return Ok(());
            }
            let text = self.condition(cond)?;
            let closer = self.block(depth, format!("{} {}", word, text), body, Some("do"))?;
            self.close(depth, closer);
            return Ok(());
        }
        // while true { body; if cond { break } }
        let yes = self.lexeme("literal.true", "true")?;
        let word = self.lexeme("stmt.while", "while")?;
        let stop = Node::new(cond.line, Form::Branch {
            test: Box::new(cond.clone_shallow()),
            then: Box::new(Node::new(cond.line, Form::Leave { how: Exit::Break, value: None })),
            otherwise: None,
        });
        let full = Node::seq(body.line, vec![body.clone_shallow(), stop]);
        let cond_text = if to.style == Style::Brace && to.first("block.open") == Some("{") { format!("({})", yes) } else { yes };
        let closer = self.block(depth, format!("{} {}", word, cond_text), &full, Some("do"))?;
        self.close(depth, closer);
        Ok(())
    }

    /// A counted loop, when the items begin with one; returns how many items it took.
    fn counted_loop(&mut self, depth: usize, items: &[Node]) -> Result<Option<usize>, String> {
        if items.len() < 3 {
            return Ok(None);
        }
        let (Form::Assign { to: limit, value: end }, Form::Assign { to: var, value: start }, Form::Loop { test, body, step: Some(step) }) =
            (&items[0].form, &items[1].form, &items[2].form)
        else {
            return Ok(None);
        };
        if !limit.name.starts_with("#end") {
            return Ok(None);
        }
        let is_test = matches!(&test.form, Form::Operate { op: Op::Lt, args } if args.len() == 2
            && matches!(&args[0].form, Form::Load(s) if s.name == var.name)
            && matches!(&args[1].form, Form::Load(s) if s.name == limit.name));
        let is_step = matches!(&step.form, Form::Assign { to, .. } if to.name == var.name);
        if !is_test || !is_step {
            return Ok(None);
        }
        let to = self.to;
        let name = self.spell(&var.name, false)?;
        if to.style == Style::Postfix {
            let a = self.postfix_expr(start)?;
            let b = self.postfix_expr(end)?;
            let q = to.first("lexical.name_quote").unwrap_or("'");
            let word = self.lexeme("stmt.for", "for")?;
            self.line(depth, format!("{} {} {}{}{} {}", a, b, q, name, q, word));
            self.body(depth + 1, body)?;
            let close = to.words("block.close").get(1).cloned().unwrap_or_else(|| "next".to_string());
            self.line(depth, close);
            return Ok(Some(3));
        }
        if to.has("stmt.for") && to.has("stmt.for.in") {
            let a = self.expr_text(start)?;
            let b = self.expr_text(end)?;
            let range = if let Some(op) = to.first("op.range") {
                format!("{}{}{}", a, op, b)
            } else if let Ok(range) = self.native_name(Native::Range) {
                self.call_text(&range, vec![Piece::atom(a), Piece::atom(b)])
            } else {
                return Err("no range".to_string());
            };
            self.declare(&var.name);
            let head = format!("{} {} {} {}", self.lexeme("stmt.for", "for")?, name, self.lexeme("stmt.for.in", "in")?, range);
            let closer = self.block(depth, head, body, Some("do"))?;
            self.close(depth, closer);
            return Ok(Some(3));
        }
        // A while loop with a counter; the step also runs before each continue.
        let start_text = self.expr_text(start)?;
        let init = self.binding(&var.name, start_text)?;
        self.statement_line(depth, init);
        let end_piece = self.expr(end)?;
        let (lt, lt_tier) = self.op_lexeme(Op::Lt)?;
        let var_text = self.expr_text(&Node::new(items[1].line, Form::Load(var.clone())))?;
        let cond = format!("{} {} {}", var_text, lt, self.brackets(end_piece, lt_tier, false)?);
        let word = self.lexeme("stmt.while", "while")?;
        let stepped = with_step_before_continue(body, step);
        let full = Node::seq(body.line, vec![stepped, step.clone_shallow()]);
        let cond = if to.style == Style::Brace && to.first("block.open") == Some("{") { format!("({})", cond) } else { cond };
        let closer = self.block(depth, format!("{} {}", word, cond), &full, Some("do"))?;
        self.close(depth, closer);
        Ok(Some(3))
    }

    // ---------- bindings

    fn declare(&mut self, name: &str) {
        self.declared.last_mut().unwrap().insert(name.to_string());
    }

    fn assignment(&self, name: String, value: String) -> Text {
        Ok(format!("{} {} {}", name, self.lexeme("stmt.assign", "assignment")?, value))
    }

    /// `name = value`, declared where the language declares.
    fn binding(&mut self, name: &str, value: String) -> Text {
        let to = self.to;
        if to.style == Style::Postfix {
            let q = to.first("lexical.name_quote").unwrap_or("'");
            let spelled = self.spell(name, false)?;
            return Ok(format!("{} {}{}{} {}", value, q, spelled, q, self.lexeme("stmt.assign", "assignment")?));
        }
        let spelled = self.spell(name, false)?;
        let assign = self.lexeme("stmt.assign", "assignment")?;
        let already = self.declared.last().unwrap().contains(name);
        if !to.has("stmt.let") || already {
            return Ok(format!("{} {} {}", spelled, assign, value));
        }
        self.declare(name);
        let keyword = to.first("stmt.let").unwrap().to_string();
        let reassigned = self.assigned.last().unwrap().get(name).copied().unwrap_or(0) > 1;
        let mutable = match to.first("stmt.let.mutable") {
            Some(m) if reassigned => format!(" {}", m),
            _ => String::new(),
        };
        Ok(format!("{}{} {} {} {}", keyword, mutable, spelled, assign, value))
    }

    // ---------- functions

    fn function(&mut self, depth: usize, name: &str, program: &Program, is_entry: bool) -> Result<(), String> {
        let spelled = self.spell(name, true)?;
        self.declared.push(program.params.iter().cloned().collect());
        let body_items: Vec<&Node> = match &program.body.form {
            Form::Sequence(items) => items.iter().collect(),
            _ => vec![&program.body],
        };
        self.assigned.push(count_assignments(&body_items));
        let outcome = self.function_body(depth, &spelled, program, is_entry);
        self.declared.pop();
        self.assigned.pop();
        outcome
    }

    fn function_body(&mut self, depth: usize, spelled: &str, program: &Program, is_entry: bool) -> Result<(), String> {
        let to = self.to;
        let mut body = explicit_returns(&program.body);
        // A source that yields what a function assigned to its own name
        // (Pascal) returns that name at the end, unless the target does the same.
        if self.from.flag("stmt.function.result_by_name") && !to.flag("stmt.function.result_by_name") && !is_entry {
            let assigns_self = program.slot_names.iter().any(|n| *n == program.name);
            if assigns_self {
                let slot = crate::tree::Slot { name: Rc::from(program.name.as_str()), locals: vec![program.slot_names.iter().position(|n| *n == program.name).unwrap()], global: 0 };
                let load = Node::new(body.line, Form::Load(slot));
                body = append_return(body, load);
            }
        }
        if to.style == Style::Postfix {
            let open = self.lexeme("stack.program.open", "programs")?;
            let close = self.lexeme("stack.program.close", "programs")?;
            let q = to.first("lexical.name_quote").unwrap_or("'").to_string();
            let assign = self.lexeme("stmt.assign", "assignment")?;
            let mut head = open;
            for p in program.params.iter().rev() {
                head.push_str(&format!(" {}{}{} {}", q, self.spell(p, false)?, q, assign));
            }
            self.line(depth, head);
            self.body(depth + 1, &body)?;
            // Every program leaves exactly one value.
            let ends_with_return = matches!(&body.form, Form::Sequence(items) if items.last().map_or(false, |n| matches!(n.form, Form::Leave { how: Exit::Return, .. })));
            if !ends_with_return {
                let null = self.lexeme("literal.null", "null")?;
                let ret = self.lexeme("stmt.return", "return")?;
                self.line(depth + 1, format!("{} {}", null, ret));
            }
            self.line(depth, format!("{} {}{}{} {}", close, q, spelled, q, assign));
            return Ok(());
        }
        let open = self.lexeme("syntax.call.open", "functions")?;
        let close = self.lexeme("syntax.call.close", "functions")?;
        let sep = to.first("syntax.call.separator").map(|s| format!("{} ", s)).unwrap_or_else(|| " ".to_string());
        let mut params = Vec::new();
        for p in &program.params {
            let spelled_p = self.spell(p, false)?;
            params.push(if to.flag("stmt.let.type_first") {
                format!("{} {}", to.first("stmt.let").unwrap(), spelled_p)
            } else {
                spelled_p
            });
        }
        let head = if to.flag("stmt.let.type_first") {
            format!("{} {}{}{}{}", to.first("stmt.let").unwrap(), spelled, open, params.join(&sep), close)
        } else {
            let keyword = self.lexeme("stmt.function", "functions")?;
            format!("{} {}{}{}{}", keyword, spelled, open, params.join(&sep), close)
        };
        let _ = is_entry;
        let declared_head = to.style == Style::Brace && to.flag("stmt.function.result_by_name");
        if declared_head {
            // Pascal: the header ends with the terminator, then the block.
            let t = to.first("stmt.terminator").unwrap_or(";");
            self.line(depth, format!("{}{}", head, t));
            let open_word = to.first("block.open").unwrap().to_string();
            self.line(depth, open_word);
            self.body(depth + 1, &body)?;
            let close_word = to.first("block.close").unwrap().to_string();
            self.line(depth, format!("{}{}", close_word, t));
            return Ok(());
        }
        let closer = self.block(depth, head, &body, None)?;
        self.close(depth, closer);
        Ok(())
    }

    // ---------- expressions

    fn expr_text(&mut self, node: &Node) -> Text {
        if self.to.style == Style::Postfix {
            return self.postfix_expr(node);
        }
        Ok(self.expr(node)?.text)
    }

    fn brackets(&self, piece: Piece, tier: u32, weaker_ok: bool) -> Text {
        let needs = match piece.tier {
            Some(t) => t < tier || (t == tier && !weaker_ok),
            None => false,
        };
        if needs {
            let o = self.lexeme("syntax.group.open", "grouping")?;
            let c = self.lexeme("syntax.group.close", "grouping")?;
            Ok(format!("{}{}{}", o, piece.text, c))
        } else {
            Ok(piece.text)
        }
    }

    fn call_text(&self, name: &str, args: Vec<Piece>) -> String {
        let to = self.to;
        let open = to.first("syntax.call.open").unwrap_or("(");
        let close = to.first("syntax.call.close").unwrap_or(")");
        let sep = to.first("syntax.call.separator").map(|s| format!("{} ", s)).unwrap_or_else(|| " ".to_string());
        format!("{}{}{}{}", name, open, args.into_iter().map(|p| p.text).collect::<Vec<_>>().join(&sep), close)
    }

    fn expr(&mut self, node: &Node) -> Result<Piece, String> {
        match &node.form {
            Form::Literal(v) => Ok(Piece::atom(self.literal(v)?)),
            Form::Load(slot) => {
                if let Some(system) = self.system_name(&slot.name) {
                    return Ok(Piece::atom(system?));
                }
                if self.provided.contains(&*slot.name) {
                    return Err(format!("`{}` as a value", slot.name));
                }
                Ok(Piece::atom(self.spell(&slot.name, false)?))
            }
            Form::Operate { op, args } => self.operation(*op, args),
            Form::Call { callee, args } => self.call(callee, args),
            Form::Branch { .. } | Form::Loop { .. } | Form::Sequence(_) | Form::Scope { .. } => {
                Err("a statement used as a value".to_string())
            }
            Form::Assign { .. } | Form::AssignIndex { .. } | Form::Leave { .. } => Err("a statement used as a value".to_string()),
        }
    }

    fn operation(&mut self, op: Op, args: &[Node]) -> Result<Piece, String> {
        let to = self.to;
        match op {
            Op::Array => {
                let open = self.lexeme("syntax.array.open", "array literal")?;
                let close = self.lexeme("syntax.array.close", "array literal")?;
                let sep = to.first("syntax.array.separator").map(|s| format!("{} ", s)).unwrap_or_else(|| " ".to_string());
                let mut items = Vec::new();
                for a in args {
                    items.push(self.expr(a)?.text);
                }
                Ok(Piece::atom(format!("{}{}{}", open, items.join(&sep), close)))
            }
            Op::Index => {
                let target = self.expr(&args[0])?;
                let index = self.expr(&args[1])?;
                if let (Some(open), Some(close)) = (to.first("op.index.open"), to.first("op.index.close")) {
                    let target = self.brackets(target, u32::MAX, false)?;
                    return Ok(Piece::atom(format!("{}{}{}{}", target, open, index.text, close)));
                }
                let get = self.native_name(Native::Get).map_err(|_| "indexing".to_string())?;
                Ok(Piece::atom(self.call_text(&get, vec![target, index])))
            }
            Op::Not | Op::Neg => {
                let (lex, tier) = self.op_lexeme(op)?;
                let operand = self.expr(&args[0])?;
                let operand = self.brackets(operand, tier, true)?;
                let joiner = if lex.chars().all(|c| c.is_alphabetic()) { " " } else { "" };
                Ok(Piece { text: format!("{}{}{}", lex, joiner, operand), tier: Some(tier) })
            }
            Op::Concat if !to.has("op.concat") => {
                // `a . b` where the language has only `+`: the rendered operands.
                let mut sides = Vec::new();
                for a in args {
                    let piece = self.expr(a)?;
                    let text_already = matches!(&a.form, Form::Literal(Value::Text(_)));
                    sides.push(if text_already {
                        piece
                    } else {
                        let name = self.native_name(Native::ToString).map_err(|_| "concatenation".to_string())?;
                        Piece::atom(self.call_text(&name, vec![piece]))
                    });
                }
                let (lex, tier) = self.op_lexeme(Op::Add)?;
                let right = sides.pop().unwrap();
                let left = sides.pop().unwrap();
                let left = self.brackets(left, tier, true)?;
                let right = self.brackets(right, tier, false)?;
                Ok(Piece { text: format!("{} {} {}", left, lex, right), tier: Some(tier) })
            }
            _ => {
                let (lex, tier) = self.op_lexeme(op)?;
                let right_assoc = to.infix.get(&lex).map_or(false, |o| o.right);
                let left = self.expr(&args[0])?;
                let right = self.expr(&args[1])?;
                let left = self.brackets(left, tier, !right_assoc)?;
                let right = self.brackets(right, tier, right_assoc)?;
                Ok(Piece { text: format!("{} {} {}", left, lex, right), tier: Some(tier) })
            }
        }
    }

    fn call(&mut self, callee: &Callee, args: &[Node]) -> Result<Piece, String> {
        let mut pieces = Vec::new();
        for a in args {
            pieces.push(self.expr(a)?);
        }
        match callee {
            Callee::Native(native, _) => self.native_call(*native, args, pieces),
            Callee::Named(slot) => {
                match self.provided(&slot.name) {
                    Provided::Native(native, _) => return self.native_call(native, args, pieces),
                    Provided::Library => {
                        let name = self.to.library.iter().find(|(_, l)| *l == self.from.library.get(&*slot.name).unwrap()).map(|(f, _)| f.clone()).unwrap();
                        return Ok(Piece::atom(self.call_text(&name, pieces)));
                    }
                    Provided::Absent => {}
                }
                let name = self.spell(&slot.name, true)?;
                Ok(Piece::atom(self.call_text(&name, pieces)))
            }
            Callee::Value(_) => Err("a program as a value".to_string()),
        }
    }

    fn native_call(&mut self, native: Native, args: &[Node], pieces: Vec<Piece>) -> Result<Piece, String> {
        let to = self.to;
        match native {
            Native::Print | Native::Write if pieces.len() > 1 => self.templated(native, args, pieces),
            Native::Print | Native::Write => {
                if pieces.len() != 1 {
                    return Err("print with several arguments".to_string());
                }
                let is_text = matches!(&args[0].form, Form::Literal(Value::Text(_)));
                let newline = native == Native::Print;
                let holes = to.words("builtin.print.placeholder");
                // A placeholder language: write("{}\n", x) or print("{}", x).
                if let (Some(hole), false) = (holes.first(), is_text) {
                    if let Ok(name) = self.native_name(Native::Write) {
                        let template = self.string(&format!("{}{}", hole, if newline { "\n" } else { "" }))?;
                        return Ok(Piece::atom(self.call_text(&name, vec![Piece::atom(template), pieces.into_iter().next().unwrap()])));
                    }
                    if newline {
                        if let Ok(name) = self.native_name(Native::Print) {
                            let template = self.string(hole)?;
                            return Ok(Piece::atom(self.call_text(&name, vec![Piece::atom(template), pieces.into_iter().next().unwrap()])));
                        }
                    }
                }
                if let Ok(name) = self.native_name(native) {
                    return Ok(Piece::atom(self.call_text(&name, pieces)));
                }
                if newline {
                    // print as write of the text and a newline.
                    let write = self.native_name(Native::Write).map_err(|_| "no `print`".to_string())?;
                    let piece = pieces.into_iter().next().unwrap();
                    let text = if is_text {
                        let Form::Literal(Value::Text(s)) = &args[0].form else { unreachable!() };
                        Piece::atom(self.string(&format!("{}\n", s))?)
                    } else {
                        let (lex, tier) = self.op_lexeme(Op::Concat).map_err(|_| "no `print`".to_string())?;
                        let left = self.brackets(piece, tier, true)?;
                        Piece { text: format!("{} {} {}", left, lex, self.string("\n")?), tier: Some(tier) }
                    };
                    return Ok(Piece::atom(self.call_text(&write, vec![text])));
                }
                // write as emit of the value's text.
                let emit = self.native_name(Native::Emit).map_err(|_| "no `write`".to_string())?;
                let piece = pieces.into_iter().next().unwrap();
                let arg = if is_text {
                    piece
                } else {
                    let name = self.native_name(Native::ToString).map_err(|_| "no `write`".to_string())?;
                    Piece::atom(self.call_text(&name, vec![piece]))
                };
                Ok(Piece::atom(self.call_text(&emit, vec![arg])))
            }
            Native::Emit => {
                let name = match self.native_name(Native::Emit) {
                    Ok(n) => n,
                    Err(_) => self.native_name(Native::Write).map_err(|_| "no `emit`".to_string())?,
                };
                Ok(Piece::atom(self.call_text(&name, pieces)))
            }
            Native::CharAt if !to.has("builtin.char_at") && to.flag("op.index.strings") => {
                let mut it = pieces.into_iter();
                let target = self.brackets(it.next().unwrap(), u32::MAX, false)?;
                let index = it.next().unwrap();
                Ok(Piece::atom(format!("{}{}{}{}", target, to.first("op.index.open").unwrap(), index.text, to.first("op.index.close").unwrap())))
            }
            Native::Range => Err("a range outside a for loop".to_string()),
            other => {
                let name = self.native_name(other)?;
                Ok(Piece::atom(self.call_text(&name, pieces)))
            }
        }
    }

    /// `printf("%d\n", x)`: a template with the source's placeholders and
    /// the values that fill them. The target either has placeholders of its
    /// own, or gets the pieces joined into one text.
    fn templated(&mut self, native: Native, args: &[Node], pieces: Vec<Piece>) -> Result<Piece, String> {
        let to = self.to;
        let Form::Literal(Value::Text(template)) = &args[0].form else { return Err("print with several arguments".to_string()) };
        let holes = self.from.words("builtin.print.placeholder");
        let next_hole = |s: &str| holes.iter().filter_map(|h| s.find(h.as_str()).map(|p| (p, h.len()))).min();
        if next_hole(template).is_none() {
            return Err("print with several arguments".to_string());
        }
        let mut pieces = pieces.into_iter();
        pieces.next();
        if let Some(hole) = to.first("builtin.print.placeholder") {
            let mut out = String::new();
            let mut rest: &str = template;
            while let Some((p, w)) = next_hole(rest) {
                out.push_str(&rest[..p]);
                out.push_str(hole);
                rest = &rest[p + w..];
            }
            out.push_str(rest);
            let name = self.native_name(native)?;
            let mut all = vec![Piece::atom(self.string(&out)?)];
            all.extend(pieces);
            return Ok(Piece::atom(self.call_text(&name, all)));
        }
        // Pieces of text and values, joined.
        let mut parts: Vec<Piece> = Vec::new();
        let mut rest: &str = template;
        while let Some((p, w)) = next_hole(rest) {
            if p > 0 {
                parts.push(Piece::atom(self.string(&rest[..p])?));
            }
            match pieces.next() {
                Some(v) => parts.push(v),
                None => parts.push(Piece::atom(self.string(&rest[p..p + w])?)),
            }
            rest = &rest[p + w..];
        }
        if !rest.is_empty() {
            parts.push(Piece::atom(self.string(rest)?));
        }
        let joined = self.joined(parts)?;
        let synthetic = Node::new(args[0].line, Form::Literal(Value::Nothing));
        self.native_call(native, std::slice::from_ref(&synthetic), vec![joined])
    }

    /// Pieces joined into one text: with the concatenation operator, or
    /// with `+` over rendered values.
    fn joined(&mut self, parts: Vec<Piece>) -> Result<Piece, String> {
        let to = self.to;
        let (lex, tier, render) = if to.has("op.concat") {
            let (lex, tier) = self.op_lexeme(Op::Concat)?;
            (lex, tier, false)
        } else {
            let (lex, tier) = self.op_lexeme(Op::Add)?;
            (lex, tier, true)
        };
        let mut out: Option<String> = None;
        for part in parts {
            let is_text = part.text.starts_with(|c: char| to.glyphs("lexical.string_quotes").contains(&c));
            let text = if render && !is_text {
                let name = self.native_name(Native::ToString).map_err(|_| "concatenation".to_string())?;
                self.call_text(&name, vec![part])
            } else {
                self.brackets(part, tier, out.is_none())?
            };
            out = Some(match out {
                None => text,
                Some(prev) => format!("{} {} {}", prev, lex, text),
            });
        }
        Ok(Piece { text: out.unwrap_or_default(), tier: Some(tier) })
    }

    // ---------- literals

    fn literal(&self, v: &Value) -> Text {
        match v {
            Value::Small(n) => Ok(n.to_string()),
            Value::Large(n) => Ok(n.to_string()),
            Value::Fraction(f) => match f.digits {
                Some(_) => {
                    // A real literal needs a finite decimal expansion.
                    let mut den = f.den.clone();
                    while den.is_multiple_of(&BigInt::from(2)) {
                        den /= 2;
                    }
                    while den.is_multiple_of(&BigInt::from(5)) {
                        den /= 5;
                    }
                    if !den.is_one() {
                        return Err("a real with no finite decimal form".to_string());
                    }
                    let point = self.to.glyph("lexical.number.decimal_point").ok_or_else(|| "no decimal point".to_string())?;
                    let mut text = real_text(&f.num, &f.den, usize::MAX / 2);
                    if !text.contains('.') && f.num.is_zero() {
                        text = "0".to_string();
                    }
                    if !text.contains('.') {
                        text.push_str(".0");
                    }
                    Ok(text.replace('.', &point.to_string()))
                }
                None => Err("a rational as a literal".to_string()),
            },
            Value::Text(s) => self.string(s),
            Value::Truth(true) => self.lexeme("literal.true", "true"),
            Value::Truth(false) => self.lexeme("literal.false", "false"),
            Value::Nothing | Value::Empty => self.lexeme("literal.null", "null"),
            Value::Array(_) => Err("an array as a literal".to_string()),
            Value::Routine(_) => Err("a program as a value".to_string()),
            Value::Tag(_) => Err("a kind as a literal".to_string()),
        }
    }

    fn string(&self, s: &str) -> Text {
        let to = self.to;
        let raw = to.glyphs("lexical.raw_quotes");
        let escapes = to.glyphs("lexical.string_escapes");
        let mut quotes = to.glyphs("lexical.string_quotes");
        quotes.sort_by_key(|q| raw.contains(q));
        if quotes.is_empty() {
            return Err("no string quotes".to_string());
        }
        'quote: for q in quotes {
            let is_raw = raw.contains(&q);
            let mut out = String::new();
            out.push(q);
            for c in s.chars() {
                match c {
                    '\\' => out.push_str("\\\\"),
                    c if c == q => {
                        if !is_raw && (escapes.contains(&q) || escapes.contains(&'\\')) {
                            out.push('\\');
                            out.push(q);
                        } else {
                            continue 'quote;
                        }
                    }
                    '\n' if !is_raw && escapes.contains(&'n') => out.push_str("\\n"),
                    '\t' if !is_raw && escapes.contains(&'t') => out.push_str("\\t"),
                    '\n' | '\t' => continue 'quote,
                    c => out.push(c),
                }
            }
            out.push(q);
            return Ok(out);
        }
        Err("a string the language cannot spell".to_string())
    }

    // ---------- postfix

    fn postfix_yields(&self, node: &Node) -> bool {
        match &node.form {
            Form::Call { callee: Callee::Native(n, _), .. } => !matches!(n, Native::Print | Native::Write | Native::Emit | Native::Error | Native::Push | Native::Put),
            Form::Call { callee: Callee::Named(slot), .. } => match self.provided(&slot.name) {
                Provided::Native(n, _) => !matches!(n, Native::Print | Native::Write | Native::Emit | Native::Error),
                _ => true,
            },
            _ => true,
        }
    }

    fn postfix_expr(&mut self, node: &Node) -> Text {
        let to = self.to;
        let q = to.first("lexical.name_quote").unwrap_or("'").to_string();
        match &node.form {
            Form::Literal(Value::Routine(p)) => {
                let mut lines = std::mem::take(&mut self.lines);
                self.function(0, "<program>", p, false)?;
                let program_lines: Vec<String> = self.lines.drain(..).collect();
                std::mem::swap(&mut self.lines, &mut lines);
                // The definition minus its trailing binding: the program alone.
                let text = program_lines.join(" ");
                let assign = self.lexeme("stmt.assign", "assignment")?;
                let tail = format!(" {}<program>{} {}", q, q, assign);
                Ok(text.trim_end_matches(tail.as_str()).split_whitespace().collect::<Vec<_>>().join(" "))
            }
            Form::Literal(v) => self.literal(v),
            Form::Load(slot) => {
                if let Some(system) = self.system_name(&slot.name) {
                    return system;
                }
                self.spell(&slot.name, false)
            }
            Form::Operate { op, args } => {
                let mut parts = Vec::new();
                for a in args {
                    parts.push(self.postfix_expr(a)?);
                }
                match op {
                    Op::Array => {
                        let open = self.lexeme("syntax.array.open", "array literal")?;
                        let close = self.lexeme("syntax.array.close", "array literal")?;
                        Ok(format!("{} {}{}{}", open, parts.join(" "), if parts.is_empty() { "" } else { " " }, close))
                    }
                    Op::Index => {
                        let get = self.native_name(Native::Get).map_err(|_| "indexing".to_string())?;
                        Ok(format!("{} {} {}", parts[0], parts[1], get))
                    }
                    Op::And | Op::Or => {
                        // Short-circuit as a branch.
                        let word_if = self.lexeme("stmt.if", "if")?;
                        let word_else = self.lexeme("stmt.else", "else")?;
                        let end = to.first("block.close").unwrap();
                        if *op == Op::And {
                            Ok(format!("{} {} {} {} {} {}", parts[0], word_if, parts[1], word_else, self.lexeme("literal.false", "false")?, end))
                        } else {
                            Ok(format!("{} {} {} {} {} {}", parts[0], word_if, self.lexeme("literal.true", "true")?, word_else, parts[1], end))
                        }
                    }
                    other => {
                        let (lex, _) = self.op_lexeme(*other)?;
                        Ok(format!("{} {}", parts.join(" "), lex))
                    }
                }
            }
            Form::Call { callee: Callee::Native(native @ (Native::Print | Native::Write), _), args } if args.len() > 1 => {
                // printf("%d\n", x): the pieces of the template joined by concatenation.
                let Form::Literal(Value::Text(template)) = &args[0].form else { return Err("print with several arguments".to_string()) };
                let holes = self.from.words("builtin.print.placeholder");
                let next_hole = |s: &str| holes.iter().filter_map(|h| s.find(h.as_str()).map(|p| (p, h.len()))).min();
                let (concat, _) = self.op_lexeme(Op::Concat).map_err(|_| "concatenation".to_string())?;
                let mut values = args[1..].iter();
                let mut parts: Vec<String> = Vec::new();
                let mut rest: &str = template;
                while let Some((p, w)) = next_hole(rest) {
                    if p > 0 {
                        parts.push(self.string(&rest[..p])?);
                    }
                    match values.next() {
                        Some(v) => parts.push(self.postfix_expr(v)?),
                        None => parts.push(self.string(&rest[p..p + w])?),
                    }
                    rest = &rest[p + w..];
                }
                if !rest.is_empty() {
                    parts.push(self.string(rest)?);
                }
                let mut text = parts[0].clone();
                for part in &parts[1..] {
                    text = format!("{} {} {}", text, part, concat);
                }
                Ok(format!("{} {}", text, self.native_name(*native)?))
            }
            Form::Call { callee, args } => {
                let mut parts = Vec::new();
                for a in args {
                    parts.push(self.postfix_expr(a)?);
                }
                let word = match callee {
                    Callee::Native(Native::Push, _) | Callee::Native(Native::Put, _) => {
                        let Form::Load(target) = &args[0].form else { unreachable!() };
                        let name = self.spell(&target.name, false)?;
                        let native = if matches!(callee, Callee::Native(Native::Push, _)) { Native::Push } else { Native::Put };
                        return Ok(format!("{} {}{}{} {}", parts[1..].join(" "), q, name, q, self.native_name(native)?));
                    }
                    Callee::Native(Native::Real, _) if args.len() == 1 => {
                        parts.push("15".to_string());
                        self.native_name(Native::Real)?
                    }
                    Callee::Native(Native::Range, _) | Callee::Native(Native::Extern, _) => return Err("no postfix form".to_string()),
                    Callee::Native(n, _) => self.native_name(*n)?,
                    Callee::Named(slot) => match self.provided(&slot.name) {
                        Provided::Native(n, lex) => {
                            let _ = n;
                            lex
                        }
                        Provided::Library => return Err("a library function the target does not spell".to_string()),
                        Provided::Absent => self.spell(&slot.name, true)?,
                    },
                    Callee::Value(v) => {
                        let program = self.postfix_expr(v)?;
                        return Ok(format!("{} {} {}", parts.join(" "), program, self.lexeme("stack.eval", "eval")?));
                    }
                };
                Ok(format!("{}{}{}", parts.join(" "), if parts.is_empty() { "" } else { " " }, word))
            }
            _ => Err("a statement used as a value".to_string()),
        }
    }
}

/// `loop { body; if cond break }` as `until cond body`.
fn until_condition<'n>(test: &'n Node, step: &'n Option<Box<Node>>) -> Option<&'n Node> {
    if !matches!(test.form, Form::Literal(Value::Truth(true))) {
        return None;
    }
    match step.as_deref().map(|s| &s.form) {
        Some(Form::Branch { test, then, otherwise: None }) if matches!(then.form, Form::Leave { how: Exit::Break, .. }) => Some(test),
        _ => None,
    }
}

/// A function body whose trailing expression is written as a return.
fn explicit_returns(body: &Node) -> Node {
    match &body.form {
        Form::Sequence(items) if !items.is_empty() => {
            let mut out: Vec<Node> = items.iter().map(Node::clone_shallow).collect();
            let last = out.pop().unwrap();
            let last = match &last.form {
                Form::Call { callee: Callee::Native(Native::Print | Native::Write | Native::Emit | Native::Error | Native::Push | Native::Put, _), .. } => last,
                Form::Call { .. } | Form::Operate { .. } | Form::Load(_) | Form::Literal(_) => {
                    Node::new(last.line, Form::Leave { how: Exit::Return, value: Some(Box::new(last)) })
                }
                Form::Branch { test, then, otherwise } => Node::new(last.line, Form::Branch {
                    test: Box::new(test.clone_shallow()),
                    then: Box::new(explicit_returns(then)),
                    otherwise: otherwise.as_ref().map(|o| Box::new(explicit_returns(o))),
                }),
                _ => last,
            };
            out.push(last);
            Node::seq(body.line, out)
        }
        _ => body.clone_shallow(),
    }
}

/// The body with a return of `value` at its end.
fn append_return(body: Node, value: Node) -> Node {
    let ret = Node::new(value.line, Form::Leave { how: Exit::Return, value: Some(Box::new(value)) });
    match body.form {
        Form::Sequence(mut items) => {
            items.push(ret);
            Node::seq(body.line, items)
        }
        other => Node::seq(body.line, vec![Node::new(body.line, other), ret]),
    }
}

/// The body with the step before each continue that belongs to this loop.
fn with_step_before_continue(body: &Node, step: &Node) -> Node {
    match &body.form {
        Form::Sequence(items) => Node::seq(body.line, items.iter().map(|i| with_step_before_continue(i, step)).collect()),
        Form::Leave { how: Exit::Continue, .. } => Node::seq(body.line, vec![step.clone_shallow(), body.clone_shallow()]),
        Form::Branch { test, then, otherwise } => Node::new(body.line, Form::Branch {
            test: Box::new(test.clone_shallow()),
            then: Box::new(with_step_before_continue(then, step)),
            otherwise: otherwise.as_ref().map(|o| Box::new(with_step_before_continue(o, step))),
        }),
        _ => body.clone_shallow(),
    }
}

fn count_assignments(items: &[&Node]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for item in items {
        walk(item, &mut |n| {
            if let Form::Assign { to, value } = &n.form {
                if !matches!(value.form, Form::Literal(Value::Routine(_))) {
                    *counts.entry(to.name.to_string()).or_insert(0) += 1;
                }
            }
        });
    }
    counts
}

impl Node {
    /// A structural copy; programs inside are shared.
    pub fn clone_shallow(&self) -> Node {
        let form = match &self.form {
            Form::Sequence(items) => Form::Sequence(items.iter().map(Node::clone_shallow).collect()),
            Form::Scope { forget, body } => Form::Scope { forget: forget.clone(), body: Box::new(body.clone_shallow()) },
            Form::Branch { test, then, otherwise } => Form::Branch {
                test: Box::new(test.clone_shallow()),
                then: Box::new(then.clone_shallow()),
                otherwise: otherwise.as_ref().map(|o| Box::new(o.clone_shallow())),
            },
            Form::Loop { test, body, step } => Form::Loop {
                test: Box::new(test.clone_shallow()),
                body: Box::new(body.clone_shallow()),
                step: step.as_ref().map(|s| Box::new(s.clone_shallow())),
            },
            Form::Assign { to, value } => Form::Assign { to: to.clone(), value: Box::new(value.clone_shallow()) },
            Form::AssignIndex { to, index, value } => Form::AssignIndex {
                to: to.clone(),
                index: Box::new(index.clone_shallow()),
                value: Box::new(value.clone_shallow()),
            },
            Form::Call { callee, args } => Form::Call {
                callee: match callee {
                    Callee::Native(n, name) => Callee::Native(*n, name.clone()),
                    Callee::Named(slot) => Callee::Named(slot.clone()),
                    Callee::Value(v) => Callee::Value(Box::new(v.clone_shallow())),
                },
                args: args.iter().map(Node::clone_shallow).collect(),
            },
            Form::Operate { op, args } => Form::Operate { op: *op, args: args.iter().map(Node::clone_shallow).collect() },
            Form::Leave { how, value } => Form::Leave { how: *how, value: value.as_ref().map(|v| Box::new(v.clone_shallow())) },
            Form::Literal(v) => Form::Literal(v.clone()),
            Form::Load(slot) => Form::Load(slot.clone()),
        };
        Node { line: self.line, form }
    }
}
