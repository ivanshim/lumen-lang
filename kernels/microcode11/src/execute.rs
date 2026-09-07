// Stage 4, execute: the tree to values.
//
// Each form has fixed mechanics; the definition supplies only the words
// values print with, the print placeholders, whether strings index, and
// the names of the system bindings. Control transfer travels as the error
// side of a Result: a loop catches break and continue, a call catches
// return, and everything else passes them up.

use std::collections::HashMap;
use std::rc::Rc;

use num_traits::ToPrimitive;

use crate::numeric::{self, Kind as Arith};
use crate::spec::Spec;
use crate::tree::{Callee, Exit, Form, Native, Node, Op, Program, Slot};
use crate::value::{Literals, Tag, Value};

pub enum Signal {
    Fail(String),
    Return(Value),
    Break,
    Continue,
}

impl From<String> for Signal {
    fn from(s: String) -> Signal {
        Signal::Fail(s)
    }
}

type Res<T = Value> = Result<T, Signal>;

pub struct Machine<'a> {
    spec: &'a Spec,
    pub globals: Vec<Value>,
    names: Vec<String>,
    cache: HashMap<String, Value>,
    args_slot: Option<usize>,
    memo_slot: Option<usize>,
}

struct Frame {
    locals: Vec<Value>,
}

impl<'a> Machine<'a> {
    pub fn new(spec: &'a Spec, names: Vec<String>) -> Machine<'a> {
        let find = |label: &str| spec.first(label).and_then(|n| names.iter().position(|x| x == n));
        Machine {
            spec,
            globals: vec![Value::Empty; names.len()],
            args_slot: find("system.args"),
            memo_slot: find("system.memoization"),
            names,
            cache: HashMap::new(),
        }
    }

    pub fn bind_global(&mut self, name: &str, value: Value) {
        if let Some(i) = self.names.iter().position(|n| n == name) {
            self.globals[i] = value;
        }
    }

    pub fn global(&self, name: &str) -> Option<&Value> {
        let i = self.names.iter().position(|n| n == name)?;
        match &self.globals[i] {
            Value::Empty => None,
            v => Some(v),
        }
    }

    fn literals(&self) -> Literals<'a> {
        Literals {
            yes: self.spec.first("literal.true").unwrap_or("true"),
            no: self.spec.first("literal.false").unwrap_or("false"),
            none: self.spec.first("literal.null").unwrap_or("null"),
        }
    }

    /// Run a program: the value returned, else the body's value, else
    /// what it assigned to its own name where the language says so.
    pub fn run(&mut self, program: &Rc<Program>, args: Vec<Value>) -> Result<Value, String> {
        if args.len() != program.params.len() {
            return Err(format!("Function {} expects {} arguments, got {}", program.name, program.params.len(), args.len()));
        }
        let memo = self.memo_slot.map_or(false, |i| matches!(self.globals[i], Value::Truth(true)));
        let key = memo.then(|| {
            let mut k = format!("{}(", program.name);
            for a in &args {
                a.key(&mut k);
            }
            k
        });
        if let Some(hit) = key.as_ref().and_then(|k| self.cache.get(k)) {
            return Ok(hit.clone());
        }
        let mut frame = Frame { locals: vec![Value::Empty; program.slot_names.len()] };
        for (slot, a) in program.param_slots.iter().zip(args) {
            frame.locals[*slot] = a;
        }
        let result = match self.eval(&program.body, &mut frame) {
            Ok(value) => {
                let own = program.slot_names.iter().position(|n| *n == program.name);
                match own.map(|i| &frame.locals[i]) {
                    Some(v) if self.spec.flag("stmt.function.result_by_name") && !matches!(v, Value::Empty | Value::Routine(_)) => v.clone(),
                    _ => value,
                }
            }
            Err(Signal::Return(value)) => value,
            Err(Signal::Break) | Err(Signal::Continue) => Value::Nothing,
            Err(Signal::Fail(e)) => return Err(e),
        };
        if let Some(k) = key {
            self.cache.insert(k, result.clone());
        }
        Ok(result)
    }

    /// Run the top level; a break or return there ends the program.
    pub fn run_top(&mut self, program: &Program) -> Result<(), String> {
        let mut frame = Frame { locals: vec![Value::Empty; program.slot_names.len()] };
        match self.eval(&program.body, &mut frame) {
            Ok(_) | Err(Signal::Return(_)) | Err(Signal::Break) | Err(Signal::Continue) => Ok(()),
            Err(Signal::Fail(e)) => Err(e),
        }
    }

    // ---- bindings

    fn read(&self, slot: &Slot, frame: &Frame) -> Result<Value, String> {
        for &i in &slot.locals {
            if !matches!(frame.locals[i], Value::Empty) {
                return Ok(frame.locals[i].clone());
            }
        }
        match &self.globals[slot.global] {
            Value::Empty => Err(format!("Undefined variable: {}", slot.name)),
            v => Ok(v.clone()),
        }
    }

    fn write(&mut self, slot: &Slot, frame: &mut Frame, value: Value) -> Result<(), String> {
        match slot.locals.first() {
            Some(&i) => frame.locals[i] = value,
            None => {
                if Some(slot.global) == self.args_slot {
                    return Err(format!("Cannot reassign {} (system-provided immutable value)", slot.name));
                }
                self.globals[slot.global] = value;
            }
        }
        Ok(())
    }

    fn array_mut<'f>(&'f mut self, slot: &Slot, frame: &'f mut Frame) -> Result<&'f mut Vec<Value>, String> {
        let cell: &mut Value = match slot.locals.iter().find(|&&i| !matches!(frame.locals[i], Value::Empty)) {
            Some(&i) => &mut frame.locals[i],
            None => match &self.globals[slot.global] {
                Value::Empty => return Err(format!("Undefined variable '{}'", slot.name)),
                _ => &mut self.globals[slot.global],
            },
        };
        match cell {
            Value::Array(items) => Ok(Rc::make_mut(items)),
            _ => Err(format!("Variable '{}' is not an array", slot.name)),
        }
    }

    fn routine(&self, slot: &Slot, frame: &Frame) -> Result<Rc<Program>, String> {
        for &i in &slot.locals {
            if let Value::Routine(p) = &frame.locals[i] {
                return Ok(p.clone());
            }
        }
        match &self.globals[slot.global] {
            Value::Routine(p) => Ok(p.clone()),
            Value::Empty => Err(format!("Unknown function: {}", slot.name)),
            _ => Err(format!("'{}' is not a function", slot.name)),
        }
    }

    // ---- the forms

    fn eval(&mut self, node: &Node, frame: &mut Frame) -> Res {
        match &node.form {
            Form::Sequence(items) => {
                let mut last = Value::Nothing;
                for item in items {
                    last = self.eval(item, frame)?;
                }
                Ok(last)
            }
            Form::Scope { forget, body } => {
                let outcome = self.eval(body, frame);
                for slot in forget {
                    match slot.locals.first() {
                        Some(&i) => frame.locals[i] = Value::Empty,
                        None => self.globals[slot.global] = Value::Empty,
                    }
                }
                outcome
            }
            Form::Branch { test, then, otherwise } => {
                if self.eval(test, frame)?.is_true() {
                    self.eval(then, frame)
                } else if let Some(o) = otherwise {
                    self.eval(o, frame)
                } else {
                    Ok(Value::Nothing)
                }
            }
            Form::Loop { test, body, step } => {
                while self.eval(test, frame)?.is_true() {
                    match self.eval(body, frame) {
                        Ok(_) | Err(Signal::Continue) => {}
                        Err(Signal::Break) => return Ok(Value::Nothing),
                        Err(other) => return Err(other),
                    }
                    if let Some(step) = step {
                        match self.eval(step, frame) {
                            Ok(_) | Err(Signal::Continue) => {}
                            Err(Signal::Break) => return Ok(Value::Nothing),
                            Err(other) => return Err(other),
                        }
                    }
                }
                Ok(Value::Nothing)
            }
            Form::Assign { to, value } => {
                let v = self.eval(value, frame)?;
                self.write(to, frame, v.clone())?;
                Ok(v)
            }
            Form::AssignIndex { to, index, value } => {
                let i = self.eval(index, frame)?;
                let v = self.eval(value, frame)?;
                let i = position(&i)?;
                let items = self.array_mut(to, frame)?;
                if i >= items.len() {
                    return Err(format!("Array index {} out of bounds (length: {})", i, items.len()).into());
                }
                items[i] = v.clone();
                Ok(v)
            }
            Form::Call { callee, args } => self.call(callee, args, frame),
            Form::Operate { op, args } => self.operate(*op, args, frame),
            Form::Leave { how, value } => {
                let v = match value {
                    Some(v) => self.eval(v, frame)?,
                    None => Value::Nothing,
                };
                Err(match how {
                    Exit::Return => Signal::Return(v),
                    Exit::Break => Signal::Break,
                    Exit::Continue => Signal::Continue,
                })
            }
            Form::Literal(v) => Ok(v.clone()),
            Form::Load(slot) => Ok(self.read(slot, frame)?),
        }
    }

    fn call(&mut self, callee: &Callee, args: &[Node], frame: &mut Frame) -> Res {
        match callee {
            Callee::Native(Native::Push, name) | Callee::Native(Native::Put, name) => {
                let Some(Node { form: Form::Load(slot), .. }) = args.first() else {
                    return Err(format!("First argument to {}() must be an array variable name", name).into());
                };
                let mut values = Vec::new();
                for a in &args[1..] {
                    values.push(self.eval(a, frame)?);
                }
                let wanted = if matches!(callee, Callee::Native(Native::Push, _)) { 1 } else { 2 };
                if values.len() != wanted {
                    return Err(format!("{}() expects {} arguments, got {}", name, wanted + 1, values.len() + 1).into());
                }
                let items = self.array_mut(slot, frame)?;
                if wanted == 1 {
                    items.push(values.pop().unwrap());
                } else {
                    let v = values.pop().unwrap();
                    let i = position(&values.pop().unwrap())?;
                    if i >= items.len() {
                        return Err(format!("Array index {} out of bounds (length: {})", i, items.len()).into());
                    }
                    items[i] = v;
                }
                Ok(Value::Nothing)
            }
            Callee::Native(native, name) => {
                let mut values = Vec::with_capacity(args.len());
                for a in args {
                    values.push(self.eval(a, frame)?);
                }
                Ok(self.native(*native, name, &values)?)
            }
            Callee::Named(slot) => {
                let mut values = Vec::with_capacity(args.len());
                for a in args {
                    values.push(self.eval(a, frame)?);
                }
                let program = self.routine(slot, frame)?;
                Ok(self.run(&program, values)?)
            }
            Callee::Value(node) => {
                let program = match self.eval(node, frame)? {
                    Value::Routine(p) => p,
                    _ => return Err("eval needs a program".to_string().into()),
                };
                let mut values = Vec::with_capacity(args.len());
                for a in args {
                    values.push(self.eval(a, frame)?);
                }
                Ok(self.run(&program, values)?)
            }
        }
    }

    fn operate(&mut self, op: Op, args: &[Node], frame: &mut Frame) -> Res {
        let lit = self.literals();
        Ok(match op {
            Op::Array => {
                let mut items = Vec::with_capacity(args.len());
                for a in args {
                    items.push(self.eval(a, frame)?);
                }
                Value::Array(Rc::new(items))
            }
            Op::Not => Value::Truth(!self.eval(&args[0], frame)?.is_true()),
            Op::Neg => {
                let v = self.eval(&args[0], frame)?;
                match numeric::compute(Arith::Sub, &Value::Small(0), &v) {
                    Some(r) => r?,
                    None => return Err("Cannot negate non-numeric value".to_string().into()),
                }
            }
            Op::And => {
                let a = self.eval(&args[0], frame)?;
                if !a.is_true() {
                    return Ok(Value::Truth(false));
                }
                Value::Truth(self.eval(&args[1], frame)?.is_true())
            }
            Op::Or => {
                let a = self.eval(&args[0], frame)?;
                if a.is_true() {
                    return Ok(Value::Truth(true));
                }
                Value::Truth(self.eval(&args[1], frame)?.is_true())
            }
            _ => {
                let a = self.eval(&args[0], frame)?;
                let b = self.eval(&args[1], frame)?;
                match op {
                    Op::Eq => Value::Truth(a.equals(&b)),
                    Op::Ne => Value::Truth(!a.equals(&b)),
                    Op::Concat => Value::from_text(&format!("{}{}", a.show(lit), b.show(lit))),
                    Op::Index => self.index(&a, &b)?,
                    Op::Add if matches!(a, Value::Text(_)) || matches!(b, Value::Text(_)) => {
                        Value::from_text(&format!("{}{}", a.show(lit), b.show(lit)))
                    }
                    Op::Lt | Op::Le | Op::Gt | Op::Ge => {
                        let below = |x: &Value, y: &Value| -> Result<bool, String> {
                            match numeric::below(x, y) {
                                Some(r) => Ok(r),
                                None => Ok(x.whole()? < y.whole()?),
                            }
                        };
                        Value::Truth(match op {
                            Op::Lt => below(&a, &b)?,
                            Op::Gt => below(&b, &a)?,
                            Op::Le => !below(&b, &a)?,
                            _ => !below(&a, &b)?,
                        })
                    }
                    _ => {
                        let kind = match op {
                            Op::Add => Arith::Add,
                            Op::Sub => Arith::Sub,
                            Op::Mul => Arith::Mul,
                            Op::Div => Arith::Div,
                            Op::DivReal => Arith::DivReal,
                            Op::Quot => Arith::Quot,
                            Op::Rem => Arith::Rem,
                            _ => Arith::Pow,
                        };
                        match numeric::compute(kind, &a, &b) {
                            Some(r) => r?,
                            None => match kind {
                                Arith::Add => Value::from_big(a.whole()? + b.whole()?),
                                Arith::Sub => Value::from_big(a.whole()? - b.whole()?),
                                Arith::Mul => Value::from_big(a.whole()? * b.whole()?),
                                Arith::Div | Arith::DivReal => return Err("Division requires numeric operands".to_string().into()),
                                Arith::Quot => return Err("Integer quotient requires numeric operands".to_string().into()),
                                Arith::Rem => return Err("Modulo requires numeric operands".to_string().into()),
                                Arith::Pow => return Err("Exponentiation requires numeric operands".to_string().into()),
                            },
                        }
                    }
                }
            }
        })
    }

    fn index(&self, target: &Value, at: &Value) -> Result<Value, String> {
        let i = position(at)?;
        match target {
            Value::Array(items) => items.get(i).cloned().ok_or_else(|| format!("Array index {} out of bounds (length: {})", i, items.len())),
            Value::Text(s) if self.spec.flag("op.index.strings") => s
                .chars()
                .nth(i)
                .map(|c| Value::from_text(&c.to_string()))
                .ok_or_else(|| format!("String index {} out of bounds (length: {})", i, s.chars().count())),
            _ => Err("Cannot index non-array value".to_string()),
        }
    }

    // ---- builtins

    fn print_text(&self, values: &[Value]) -> String {
        let lit = self.literals();
        let holes = self.spec.words("builtin.print.placeholder");
        let find = |s: &str| holes.iter().filter_map(|h| s.find(h.as_str()).map(|p| (p, h.len()))).min();
        if let (Some(Value::Text(template)), true) = (values.first(), values.len() > 1) {
            if find(template).is_some() {
                let mut out = String::new();
                let mut rest = values[1..].iter();
                let mut s: &str = template;
                while let Some((p, w)) = find(s) {
                    out.push_str(&s[..p]);
                    match rest.next() {
                        Some(v) => out.push_str(&v.show(lit)),
                        None => out.push_str(&s[p..p + w]),
                    }
                    s = &s[p + w..];
                }
                out.push_str(s);
                return out;
            }
        }
        values.iter().map(|v| v.show(lit)).collect::<Vec<_>>().join(" ")
    }

    fn native(&mut self, native: Native, name: &str, args: &[Value]) -> Result<Value, String> {
        let lit = self.literals();
        let exactly = |n: usize| -> Result<(), String> {
            if args.len() == n {
                Ok(())
            } else {
                Err(format!("{}() expects {} argument{}, got {}", name, n, if n == 1 { "" } else { "s" }, args.len()))
            }
        };
        let number_arg = |v: &Value, what: &str| -> Result<(), String> {
            if numeric::ratio(v).is_some() { Ok(()) } else { Err(format!("{}() requires a {} argument", name, what)) }
        };
        Ok(match native {
            Native::Emit => {
                exactly(1)?;
                match &args[0] {
                    Value::Text(s) => print!("{}", s),
                    _ => return Err(format!("{}() requires a string argument", name)),
                }
                Value::Nothing
            }
            Native::Print => {
                println!("{}", self.print_text(args));
                Value::Nothing
            }
            Native::Write => {
                print!("{}", self.print_text(args));
                Value::Nothing
            }
            Native::Range => return Err(format!("{}() spells a range, which belongs in a for loop", name)),
            Native::Real => {
                if args.is_empty() || args.len() > 2 {
                    return Err(format!("{}() expects 1 or 2 arguments, got {}", name, args.len()));
                }
                let digits = match args.get(1) {
                    None => numeric::DEFAULT_DIGITS,
                    Some(Value::Small(n)) if *n >= 0 => *n as usize,
                    Some(Value::Small(_)) | Some(Value::Large(_)) => return Err("Precision must be a positive integer".to_string()),
                    Some(_) => return Err("Precision argument must be an integer".to_string()),
                };
                number_arg(&args[0], "number, rational, or real")?;
                numeric::as_real(&args[0], digits).unwrap()
            }
            Native::Precision => {
                exactly(1)?;
                match &args[0] {
                    Value::Fraction(f) if f.digits.is_some() => Value::Small(f.digits.unwrap() as i64),
                    _ => return Err(format!("{}() requires a real argument", name)),
                }
            }
            Native::ToString => {
                exactly(1)?;
                Value::from_text(&args[0].show(lit))
            }
            Native::ToInt => {
                exactly(1)?;
                number_arg(&args[0], "number")?;
                let r = numeric::ratio(&args[0]).unwrap();
                Value::from_big(r.num / r.den)
            }
            Native::ToReal => {
                exactly(1)?;
                number_arg(&args[0], "number")?;
                match &args[0] {
                    v @ Value::Fraction(f) if f.digits.is_some() => v.clone(),
                    v => numeric::as_real(v, numeric::DEFAULT_DIGITS).unwrap(),
                }
            }
            Native::Len => {
                exactly(1)?;
                match &args[0] {
                    Value::Text(s) => Value::Small(s.chars().count() as i64),
                    Value::Array(a) => Value::Small(a.len() as i64),
                    _ => return Err(format!("{}() requires a string or array argument", name)),
                }
            }
            Native::CharAt => {
                exactly(2)?;
                match (&args[0], &args[1]) {
                    (Value::Text(s), Value::Small(i)) => match usize::try_from(*i).ok().and_then(|i| s.chars().nth(i)) {
                        Some(c) => Value::from_text(&c.to_string()),
                        None => return Err(format!("{} index out of bounds", name)),
                    },
                    (Value::Text(_), _) => return Err(format!("{}() second argument must be an integer", name)),
                    _ => return Err(format!("{}() first argument must be a string", name)),
                }
            }
            Native::Ord => {
                exactly(1)?;
                match &args[0] {
                    Value::Text(s) => match s.chars().next() {
                        Some(c) => Value::Small(c as i64),
                        None => return Err(format!("{}() requires a non-empty string", name)),
                    },
                    _ => return Err(format!("{}() requires a string argument", name)),
                }
            }
            Native::Chr => {
                exactly(1)?;
                let code = match &args[0] {
                    Value::Small(n) => u32::try_from(*n).ok(),
                    Value::Large(_) => None,
                    _ => return Err(format!("{}() requires an integer argument", name)),
                }
                .ok_or_else(|| format!("{}() argument must be a non-negative integer within valid Unicode range", name))?;
                match char::from_u32(code) {
                    Some(c) => Value::from_text(&c.to_string()),
                    None => return Err(format!("{}() argument {} is not a valid Unicode code point", name, code)),
                }
            }
            Native::Error => {
                exactly(1)?;
                return match &args[0] {
                    Value::Text(s) => Err(s.to_string()),
                    _ => Err(format!("{}() argument must be a string", name)),
                };
            }
            Native::Kind => {
                exactly(1)?;
                match args[0].tag() {
                    Some(t) => Value::Tag(t),
                    None => return Err(format!("{}(): unknown value type", name)),
                }
            }
            Native::Num => {
                exactly(1)?;
                number_arg(&args[0], "number")?;
                Value::from_big(numeric::ratio(&args[0]).unwrap().num)
            }
            Native::Den => {
                exactly(1)?;
                number_arg(&args[0], "number")?;
                Value::from_big(numeric::ratio(&args[0]).unwrap().den)
            }
            Native::Get => {
                exactly(2)?;
                self.index(&args[0], &args[1])?
            }
            Native::Extern => {
                let target = match args.first() {
                    Some(Value::Text(s)) => s.to_string(),
                    Some(_) => return Err(format!("First argument to {} must be a string (function name)", name)),
                    None => return Err(format!("{} requires at least one argument (function name)", name)),
                };
                let rest = &args[1..];
                if !matches!(target.as_str(), "print_native" | "debug_info" | "value_type") {
                    return Err(format!("Unknown external function: {}", target));
                }
                if rest.len() != 1 {
                    return Err(format!("{} expects 1 argument, got {}", target, rest.len()));
                }
                let v = &rest[0];
                match target.as_str() {
                    "print_native" => println!("{}", v.show(lit)),
                    "debug_info" => eprintln!("[DEBUG] {}", v.show(lit)),
                    _ => {
                        return match v.tag() {
                            Some(Tag::Integer | Tag::Rational | Tag::Real) => Ok(Value::Small(0)),
                            Some(Tag::Boolean) => Ok(Value::Small(1)),
                            Some(Tag::Text) => Ok(Value::Small(2)),
                            _ => Err("Unknown value type".to_string()),
                        }
                    }
                }
                v.clone()
            }
            Native::Push | Native::Put => unreachable!("handled with their target"),
        })
    }
}

fn position(v: &Value) -> Result<usize, String> {
    match v {
        Value::Small(n) => usize::try_from(*n).map_err(|_| "Array index out of bounds".to_string()),
        Value::Large(n) => n.to_usize().ok_or_else(|| "Array index out of bounds".to_string()),
        _ => Err("Array index must be a number".to_string()),
    }
}
