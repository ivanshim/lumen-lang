use crate::language::prelude::*;
// Postfix evaluation: the words of a reverse Polish language (RPLumen) act
// on one stack. A literal pushes itself, an operator pops its operands and
// pushes the result, a control word takes its condition from the top, a
// quoted name is data for the next word, and a bare word runs the program
// stored under it or pushes its value. The stack lives in this module;
// every word is a statement node, so bodies are ordinary statement lists
// and programs (`« ... »`) are functions of no parameters.

use std::any::Any;
use std::cell::{Cell, RefCell};

use num_bigint::BigInt;
use num_traits::ToPrimitive;

use crate::kernel::ast::{Control, ExprNode, StmtNode};
use crate::kernel::parser::Parser;
use crate::kernel::runtime::{Env, RuntimeValue, Value};
use crate::language::definition::def;
use crate::language::expressions::arithmetic::{self, Arith};
use crate::language::expressions::comparison::{self, Cmp};
use crate::language::expressions::variable::{call_builtin, call_user_function};
use crate::language::statements::functions::define_function;
use crate::language::definition::BlockStyle;
use crate::language::structure::structural::{consume_separators, expect_close, DEDENT, EOF, INDENT, NEWLINE};
use crate::language::values::{as_bool, as_number, LumenArray, LumenBool, LumenNull, LumenNumber};

// --------------------
// The stack
// --------------------

thread_local! {
    static STACK: RefCell<Vec<Value>> = RefCell::new(Vec::new());
    /// Counter naming the programs a source defines.
    static PROGRAMS: Cell<usize> = Cell::new(0);
}

fn push(value: Value) {
    STACK.with(|stack| stack.borrow_mut().push(value));
}

fn pop() -> LumenResult<Value> {
    STACK.with(|stack| stack.borrow_mut().pop()).ok_or_else(|| "Stack underflow".to_string())
}

fn depth() -> usize {
    STACK.with(|stack| stack.borrow().len())
}

/// Gather what was pushed since the mark into one array.
fn gather(mark: usize) -> LumenResult<()> {
    STACK.with(|stack| {
        let mut stack = stack.borrow_mut();
        if mark > stack.len() {
            return Err("Stack underflow".to_string());
        }
        let gathered = stack.split_off(mark);
        stack.push(Box::new(LumenArray::new(gathered)));
        Ok(())
    })
}

// --------------------
// Program values
// --------------------

/// A program value: `« ... »`, held by the name its body is registered
/// under. Running it is calling that function with no arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct LumenProgram {
    pub name: String,
}

impl RuntimeValue for LumenProgram {
    fn clone_boxed(&self) -> Box<dyn RuntimeValue> {
        Box::new(self.clone())
    }

    fn as_debug_string(&self) -> String {
        format!("Program({})", self.name)
    }

    fn as_display_string(&self) -> String {
        "<function()>".to_string()
    }

    fn eq_value(&self, other: &dyn RuntimeValue) -> Result<bool, String> {
        Ok(other.as_any().downcast_ref::<LumenProgram>().map_or(false, |p| p.name == self.name))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

fn run_program(value: &Value, env: &mut Env) -> LumenResult<()> {
    match value.as_any().downcast_ref::<LumenProgram>() {
        Some(program) => call_user_function(&program.name, Vec::new(), env).map(|_| ()),
        None => Err("eval needs a program".to_string()),
    }
}

// --------------------
// Words
// --------------------

type Body = Vec<Box<dyn StmtNode>>;

#[derive(Debug, Clone, Copy)]
enum Binary {
    Arith(Arith),
    Cmp(Cmp),
    And,
    Or,
}

/// One word of a postfix program.
#[derive(Debug)]
enum Word {
    Literal(Box<dyn ExprNode>),
    Assign(String),
    If { then: Body, otherwise: Option<Body> },
    While { condition: Body, body: Body },
    DoUntil { body: Body, condition: Body },
    For { var: String, body: Body },
    Return,
    Break,
    Continue,
    Program(String),
    Array(Body),
    Dup,
    Drop,
    Swap,
    Over,
    Rot,
    Eval,
    Binary(Binary),
    Not,
    Negate,
    Builtin { name: String, arity: usize, yields: bool },
    PushTo(String),
    PutTo(String),
    Name(String),
}

/// Run a body, handing any control transfer to the caller.
fn run(body: &Body, env: &mut Env) -> LumenResult<Control> {
    for stmt in body {
        match stmt.exec(env)? {
            Control::None | Control::ExprValue(_) => {}
            other => return Ok(other),
        }
    }
    Ok(Control::None)
}

fn truth(value: &Value) -> LumenResult<bool> {
    Ok(as_bool(value.as_ref())?.value)
}

fn boolean(value: bool) -> Value {
    Box::new(LumenBool::new(value))
}

fn integer(value: i32) -> Value {
    Box::new(LumenNumber::new(BigInt::from(value)))
}

fn index_of(value: &Value) -> LumenResult<usize> {
    match as_number(value.as_ref()) {
        Ok(n) => n.value.to_usize().ok_or_else(|| "Array index out of bounds".to_string()),
        Err(_) => Err("Array index must be a number".to_string()),
    }
}

fn array_mut<'e>(env: &'e mut Env, name: &str) -> LumenResult<&'e mut LumenArray> {
    let slot = env.get_mut(name).ok_or_else(|| format!("Undefined variable '{}'", name))?;
    slot.as_any_mut().downcast_mut::<LumenArray>().ok_or_else(|| format!("Variable '{}' is not an array", name))
}

/// A builtin over popped arguments: indexing as a function is this
/// module's, everything else is the shared builtin table.
fn builtin(name: &str, values: Vec<Value>) -> LumenResult<Value> {
    if def().is("builtin.get", name) {
        let idx = index_of(&values[1])?;
        return match values[0].as_any().downcast_ref::<LumenArray>() {
            Some(array) => array
                .elements
                .get(idx)
                .cloned()
                .ok_or_else(|| format!("Array index {} out of bounds (length: {})", idx, array.elements.len())),
            None => Err("Cannot index non-array value".to_string()),
        };
    }
    call_builtin(name, values)
}

impl StmtNode for Word {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        match self {
            Word::Literal(expr) => push(expr.eval(env)?),
            Word::Assign(name) => {
                let value = pop()?;
                env.assign(name, value)?;
            }
            Word::If { then, otherwise } => {
                if truth(&pop()?)? {
                    return run(then, env);
                } else if let Some(otherwise) = otherwise {
                    return run(otherwise, env);
                }
            }
            Word::While { condition, body } => loop {
                if let Control::Return(v) = run(condition, env)? {
                    return Ok(Control::Return(v));
                }
                if !truth(&pop()?)? {
                    break;
                }
                match run(body, env)? {
                    Control::Break => break,
                    Control::Return(v) => return Ok(Control::Return(v)),
                    _ => {}
                }
            },
            Word::DoUntil { body, condition } => loop {
                match run(body, env)? {
                    Control::Break => break,
                    Control::Return(v) => return Ok(Control::Return(v)),
                    _ => {}
                }
                if let Control::Return(v) = run(condition, env)? {
                    return Ok(Control::Return(v));
                }
                if truth(&pop()?)? {
                    break;
                }
            },
            // Counts from the lower bound up to, not including, the upper.
            Word::For { var, body } => {
                let limit = pop()?;
                let start = pop()?;
                env.assign(var, start)?;
                loop {
                    let current = env.get(var)?;
                    if !truth(&comparison::apply(Cmp::Lt, &current, &limit)?)? {
                        break;
                    }
                    match run(body, env)? {
                        Control::Break => break,
                        Control::Return(v) => return Ok(Control::Return(v)),
                        _ => {}
                    }
                    let current = env.get(var)?;
                    env.assign(var, arithmetic::apply(Arith::Add, current, integer(1))?)?;
                }
            }
            Word::Return => return Ok(Control::Return(Box::new(LumenNull))),
            Word::Break => return Ok(Control::Break),
            Word::Continue => return Ok(Control::Continue),
            Word::Program(name) => push(Box::new(LumenProgram { name: name.clone() })),
            Word::Array(body) => {
                let mark = depth();
                match run(body, env)? {
                    Control::None | Control::ExprValue(_) => {}
                    other => return Ok(other),
                }
                gather(mark)?;
            }
            Word::Dup => {
                let a = pop()?;
                push(a.clone());
                push(a);
            }
            Word::Drop => {
                pop()?;
            }
            Word::Swap => {
                let b = pop()?;
                let a = pop()?;
                push(b);
                push(a);
            }
            Word::Over => {
                let b = pop()?;
                let a = pop()?;
                push(a.clone());
                push(b);
                push(a);
            }
            Word::Rot => {
                let c = pop()?;
                let b = pop()?;
                let a = pop()?;
                push(b);
                push(c);
                push(a);
            }
            Word::Eval => {
                let program = pop()?;
                run_program(&program, env)?;
            }
            Word::Binary(op) => {
                let b = pop()?;
                let a = pop()?;
                push(match op {
                    Binary::Arith(op) => arithmetic::apply(*op, a, b)?,
                    Binary::Cmp(op) => comparison::apply(*op, &a, &b)?,
                    Binary::And => boolean(truth(&a)? && truth(&b)?),
                    Binary::Or => boolean(truth(&a)? || truth(&b)?),
                });
            }
            Word::Not => {
                let a = pop()?;
                push(boolean(!truth(&a)?));
            }
            // Derived: -x is 0 - x, so a real keeps its precision.
            Word::Negate => {
                let a = pop()?;
                push(arithmetic::apply(Arith::Sub, integer(0), a)?);
            }
            Word::Builtin { name, arity, yields } => {
                let mut values = Vec::with_capacity(*arity);
                for _ in 0..*arity {
                    values.push(pop()?);
                }
                values.reverse();
                let result = builtin(name, values)?;
                if *yields {
                    push(result);
                }
            }
            Word::PushTo(name) => {
                let value = pop()?;
                array_mut(env, name)?.elements.push(value);
            }
            Word::PutTo(name) => {
                let value = pop()?;
                let idx = index_of(&pop()?)?;
                let array = array_mut(env, name)?;
                if idx >= array.elements.len() {
                    return Err(format!("Array index {} out of bounds (length: {})", idx, array.elements.len()));
                }
                array.elements[idx] = value;
            }
            // A bare word: run the program bound to it, or push its value.
            Word::Name(name) => {
                let value = env.get(name)?;
                if value.as_any().is::<LumenProgram>() {
                    run_program(&value, env)?;
                } else {
                    push(value);
                }
            }
        }
        Ok(Control::None)
    }
}

// --------------------
// Parsing
// --------------------

/// Every token is a word; this handler matches all of them.
pub struct WordHandler;

/// The next word: a run of word characters, a reserved word, or a symbol.
fn next_word(parser: &mut Parser) -> LumenResult<String> {
    if parser.peek().lexeme == EOF {
        return Err(err_at(parser, "Expected a word"));
    }
    match parser.take_word() {
        Some(word) => Ok(word),
        None => Ok(parser.advance().lexeme),
    }
}

fn expect_intro(parser: &mut Parser) -> LumenResult<()> {
    if def().is("block.intro", &parser.peek().lexeme) {
        parser.advance();
        Ok(())
    } else {
        Err(err_at(parser, &format!("Expected '{}'", def().first("block.intro"))))
    }
}

fn expect_lexeme(parser: &mut Parser, lexeme: &str) -> LumenResult<()> {
    if parser.peek().lexeme == lexeme {
        parser.advance();
        Ok(())
    } else {
        Err(err_at(parser, &format!("Expected '{}'", lexeme)))
    }
}

/// What a run of words is part of, which says where it ends besides the
/// stop words: a block ends at its dedent and refuses a deeper indent, a
/// program value runs to its bracket with indentation passing, a line ends
/// at the line end.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Run {
    Block,
    Program,
    Line,
}

/// Words up to one of `stops` (not consumed) or the end.
fn words(parser: &mut Parser, registry: &Registry, stops: &[String], run: Run) -> LumenResult<Body> {
    let mut body = Vec::new();
    loop {
        if run == Run::Line {
            loop {
                parser.skip_tokens();
                if def().is("stmt.terminator", &parser.peek().lexeme) {
                    parser.advance();
                } else {
                    break;
                }
            }
        } else {
            consume_separators(parser);
        }
        let lexeme = parser.peek().lexeme.clone();
        if lexeme == EOF || stops.iter().any(|s| *s == lexeme) {
            return Ok(body);
        }
        match (lexeme.as_str(), run) {
            (NEWLINE, Run::Line) | (DEDENT, Run::Block | Run::Line) | (INDENT, Run::Line) => return Ok(body),
            (INDENT | DEDENT, Run::Program) => {
                parser.advance();
                continue;
            }
            (INDENT, Run::Block) => return Err(err_at(parser, "Unexpected indentation")),
            _ => {}
        }
        let stmt = registry
            .find_stmt(parser)
            .ok_or_else(|| err_at(parser, "Unknown word"))?
            .parse(parser, registry)?;
        body.push(stmt);
    }
}

/// The body a control word governs, in the block style: indented lines,
/// a bracketed run, or the words up to a closer.
fn governed(parser: &mut Parser, registry: &Registry) -> LumenResult<Body> {
    let d = def();
    match d.block_style {
        BlockStyle::Indentation => {
            consume_separators(parser);
            if parser.peek().lexeme != INDENT {
                return Err(err_at(parser, "Expected an indented block"));
            }
            parser.advance();
            let body = words(parser, registry, &[], Run::Block)?;
            if parser.peek().lexeme != DEDENT {
                return Err(err_at(parser, "Expected the end of an indented block"));
            }
            parser.advance();
            Ok(body)
        }
        BlockStyle::Braces => {
            parser.skip_tokens();
            let opens = d.list("block.open");
            let i = opens
                .iter()
                .position(|o| *o == parser.peek().lexeme)
                .ok_or_else(|| err_at(parser, &format!("Expected '{}' to open a block", opens[0])))?;
            parser.advance();
            let close = d.list("block.close")[i].clone();
            let body = words(parser, registry, std::slice::from_ref(&close), Run::Block)?;
            expect_lexeme(parser, &close)?;
            Ok(body)
        }
        BlockStyle::Keyword => {
            let body = words(parser, registry, d.list("block.close"), Run::Block)?;
            expect_close(parser)?;
            Ok(body)
        }
    }
}

/// A loop's condition, run again each pass: the words up to where the
/// body begins, the line end, the block opener or the intro word by style.
fn condition(parser: &mut Parser, registry: &Registry) -> LumenResult<Body> {
    let d = def();
    match d.block_style {
        BlockStyle::Indentation => words(parser, registry, &[], Run::Line),
        BlockStyle::Braces => words(parser, registry, d.list("block.open"), Run::Block),
        BlockStyle::Keyword => {
            let test = words(parser, registry, d.list("block.intro"), Run::Block)?;
            expect_intro(parser)?;
            Ok(test)
        }
    }
}

/// Whether an else word follows past line ends; if so, step onto it.
fn take_else(parser: &mut Parser) -> bool {
    consume_separators(parser);
    if def().is("stmt.else", &parser.peek().lexeme) {
        parser.advance();
        return true;
    }
    false
}

impl WordHandler {
    /// A quoted name and the word that takes it.
    fn named(&self, parser: &mut Parser, registry: &Registry, quote: char) -> LumenResult<Box<dyn StmtNode>> {
        let d = def();
        parser.advance(); // the opening quote
        let name = parser.take_word().ok_or_else(|| err_at(parser, "Expected a name after the name quote"))?;
        if d.is_reserved(&name) {
            return Err(err_at(parser, &format!("'{}' is a word of the language, not a name", name)));
        }
        expect_lexeme(parser, &quote.to_string())?;
        consume_separators(parser);
        if registry.find_prefix(parser).is_some() {
            return Err(err_at(parser, &format!("The name '{}' must be followed by a word that takes it", name)));
        }
        let word = next_word(parser)?;
        if d.is("stmt.assign", &word) || d.is("stmt.let", &word) {
            return Ok(Box::new(Word::Assign(name)));
        }
        if d.is("stmt.for", &word) {
            let body = governed(parser, registry)?;
            return Ok(Box::new(Word::For { var: name, body }));
        }
        if d.is("builtin.push", &word) {
            return Ok(Box::new(Word::PushTo(name)));
        }
        if d.is("builtin.put", &word) {
            return Ok(Box::new(Word::PutTo(name)));
        }
        Err(err_at(parser, &format!("'{}' does not take a name, but '{}' was given", word, name)))
    }

    fn word(&self, parser: &mut Parser, registry: &Registry, word: String) -> LumenResult<Box<dyn StmtNode>> {
        let d = def();
        let closes = d.list("block.close");

        // Control words: the condition is the top of the stack; the body
        // is the block after the word. In the keyword style one closer
        // ends both arms of an if.
        if d.is("stmt.if", &word) {
            if d.block_style == BlockStyle::Keyword {
                let mut stops = closes.to_vec();
                stops.extend(d.list("stmt.else").iter().cloned());
                let then = words(parser, registry, &stops, Run::Block)?;
                let otherwise = if d.is("stmt.else", &parser.peek().lexeme) {
                    parser.advance();
                    Some(words(parser, registry, closes, Run::Block)?)
                } else {
                    None
                };
                expect_close(parser)?;
                return Ok(Box::new(Word::If { then, otherwise }));
            }
            let then = governed(parser, registry)?;
            let otherwise = if take_else(parser) { Some(governed(parser, registry)?) } else { None };
            return Ok(Box::new(Word::If { then, otherwise }));
        }
        if d.is("stmt.while", &word) {
            let condition = condition(parser, registry)?;
            let body = governed(parser, registry)?;
            return Ok(Box::new(Word::While { condition, body }));
        }
        if d.is("stmt.until", &word) {
            // Head first as in Lumen; the condition is tested after the body.
            let condition = condition(parser, registry)?;
            let body = governed(parser, registry)?;
            return Ok(Box::new(Word::DoUntil { body, condition }));
        }
        if d.is("stmt.return", &word) {
            return Ok(Box::new(Word::Return));
        }
        if d.is("stmt.break", &word) {
            return Ok(Box::new(Word::Break));
        }
        if d.is("stmt.continue", &word) {
            return Ok(Box::new(Word::Continue));
        }
        if d.is("stmt.for", &word) || d.is("builtin.push", &word) || d.is("builtin.put", &word) {
            return Err(err_at(parser, &format!("'{}' needs a quoted name before it", word)));
        }

        // A program value: its body is a function of no parameters.
        if let Some(i) = d.list("stack.program.open").iter().position(|o| *o == word) {
            let close = &d.list("stack.program.close")[i];
            let body = words(parser, registry, std::slice::from_ref(close), Run::Program)?;
            expect_lexeme(parser, close)?;
            let name = PROGRAMS.with(|n| {
                n.set(n.get() + 1);
                format!("#program{}", n.get())
            });
            define_function(name.clone(), Vec::new(), body);
            return Ok(Box::new(Word::Program(name)));
        }

        // An array literal gathers what its body pushes.
        if d.is("syntax.array.open", &word) {
            let close = d.first("syntax.array.close");
            let body = words(parser, registry, std::slice::from_ref(&close.to_string()), Run::Block)?;
            expect_lexeme(parser, close)?;
            return Ok(Box::new(Word::Array(body)));
        }

        // Stack words.
        let stack_words = [
            ("stack.dup", Word::Dup),
            ("stack.drop", Word::Drop),
            ("stack.swap", Word::Swap),
            ("stack.over", Word::Over),
            ("stack.rot", Word::Rot),
            ("stack.eval", Word::Eval),
        ];
        for (label, node) in stack_words {
            if d.is(label, &word) {
                return Ok(Box::new(node));
            }
        }

        // Operators.
        let binaries = [
            ("op.add", Binary::Arith(Arith::Add)),
            ("op.sub", Binary::Arith(Arith::Sub)),
            ("op.mul", Binary::Arith(Arith::Mul)),
            ("op.div", Binary::Arith(Arith::Div)),
            ("op.quot", Binary::Arith(Arith::Quot)),
            ("op.rem", Binary::Arith(Arith::Rem)),
            ("op.pow", Binary::Arith(Arith::Pow)),
            ("op.concat", Binary::Arith(Arith::Concat)),
            ("op.eq", Binary::Cmp(Cmp::Eq)),
            ("op.ne", Binary::Cmp(Cmp::Ne)),
            ("op.lt", Binary::Cmp(Cmp::Lt)),
            ("op.le", Binary::Cmp(Cmp::Le)),
            ("op.gt", Binary::Cmp(Cmp::Gt)),
            ("op.ge", Binary::Cmp(Cmp::Ge)),
            ("op.and", Binary::And),
            ("op.or", Binary::Or),
        ];
        for (label, op) in binaries {
            if d.is(label, &word) {
                return Ok(Box::new(Word::Binary(op)));
            }
        }
        if d.is("op.not", &word) {
            return Ok(Box::new(Word::Not));
        }
        if d.is("op.negate", &word) {
            return Ok(Box::new(Word::Negate));
        }

        // Builtins: arguments come off the stack; a result goes back.
        if d.is_builtin(&word) {
            let consumes = ["builtin.emit", "builtin.print", "builtin.write", "builtin.error"];
            let pairs = ["builtin.char_at", "builtin.get", "builtin.real"];
            if d.is("builtin.extern", &word) || d.is("builtin.range", &word) {
                return Err(err_at(parser, &format!("'{}' has no postfix form", word)));
            }
            let (arity, yields) = if consumes.iter().any(|l| d.is(l, &word)) {
                (1, false)
            } else if pairs.iter().any(|l| d.is(l, &word)) {
                (2, true)
            } else {
                (1, true)
            };
            return Ok(Box::new(Word::Builtin { name: word, arity, yields }));
        }

        // Any other word names a binding; a symbol is nothing.
        if word.chars().next().map_or(false, word_start) {
            return Ok(Box::new(Word::Name(word)));
        }
        Err(err_at(parser, &format!("Unexpected '{}'", word)))
    }
}

impl StmtHandler for WordHandler {
    fn matches(&self, _parser: &Parser) -> bool {
        true
    }

    fn parse(&self, parser: &mut Parser, registry: &Registry) -> LumenResult<Box<dyn StmtNode>> {
        // A quoted name is data for the next word, which must take one.
        if let Some(quote) = def().first_char("lexical.name_quote") {
            if parser.peek().lexeme == quote.to_string() {
                return self.named(parser, registry, quote);
            }
        }
        // A literal pushes itself.
        if let Some(prefix) = registry.find_prefix(parser) {
            let expr = prefix.parse(parser, registry)?;
            return Ok(Box::new(Word::Literal(expr)));
        }
        let word = next_word(parser)?;
        self.word(parser, registry, word)
    }
}

pub fn register(reg: &mut Registry) {
    reg.register_stmt(Box::new(WordHandler));
}
