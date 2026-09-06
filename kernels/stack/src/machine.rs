// The inner loop: one program at a time, one word at a time, over the
// data stack. A call runs the callee's words on the same stack with a
// fresh set of local slots; the globals are one table for the whole run.

use std::collections::HashMap;
use std::rc::Rc;

use num_traits::ToPrimitive;

use crate::code::{Builtin, Op, Place, Program, Word};
use crate::definition::Language;
use crate::number::{self, Arith};
use crate::value::{Kind, Spelling, Value};

pub struct Machine<'a> {
    lang: &'a Language,
    globals: Vec<Value>,
    names: Vec<String>,
    stack: Vec<Value>,
    marks: Vec<usize>,
    cache: HashMap<String, Value>,
    args_slot: Option<usize>,
    memo_slot: Option<usize>,
}

struct Frame {
    locals: Vec<Value>,
    result: Value,
}

type Outcome<T> = Result<T, String>;

impl<'a> Machine<'a> {
    pub fn new(lang: &'a Language, names: Vec<String>) -> Machine<'a> {
        let slot = |name: &Option<String>| name.as_ref().and_then(|n| names.iter().position(|x| x == n));
        Machine {
            lang,
            globals: vec![Value::Unset; names.len()],
            args_slot: slot(&lang.args_name),
            memo_slot: slot(&lang.memo_name),
            names,
            stack: Vec::new(),
            marks: Vec::new(),
            cache: HashMap::new(),
        }
    }

    pub fn set_global(&mut self, name: &str, value: Value) {
        if let Some(slot) = self.names.iter().position(|n| n == name) {
            self.globals[slot] = value;
        }
    }

    pub fn global(&self, name: &str) -> Option<&Value> {
        let slot = self.names.iter().position(|n| n == name)?;
        match &self.globals[slot] {
            Value::Unset => None,
            value => Some(value),
        }
    }

    fn spelling(&self) -> Spelling<'a> {
        let first = |list: &'a [String], fallback: &'a str| list.first().map_or(fallback, String::as_str);
        Spelling {
            yes: first(&self.lang.yes_words, "true"),
            no: first(&self.lang.no_words, "false"),
            none: first(&self.lang.null_words, "null"),
        }
    }

    fn pop(&mut self) -> Outcome<Value> {
        self.stack.pop().ok_or_else(|| "Stack underflow".to_string())
    }

    fn take(&mut self, n: usize) -> Outcome<Vec<Value>> {
        if self.stack.len() < n {
            return Err("Stack underflow".to_string());
        }
        let at = self.stack.len() - n;
        Ok(self.stack.split_off(at))
    }

    /// The value bound at a place: the first local slot that holds one,
    /// else the global.
    fn read(&self, place: &Place, frame: &Frame, name: &str) -> Outcome<Value> {
        for &slot in &place.locals {
            if !matches!(frame.locals[slot], Value::Unset) {
                return Ok(frame.locals[slot].clone());
            }
        }
        match &self.globals[place.global] {
            Value::Unset => Err(format!("Undefined variable: {}", name)),
            value => Ok(value.clone()),
        }
    }

    fn slot_mut<'f>(&'f mut self, place: &Place, frame: &'f mut Frame, name: &str) -> Outcome<&'f mut Value> {
        for &slot in &place.locals {
            if !matches!(frame.locals[slot], Value::Unset) {
                return Ok(&mut frame.locals[slot]);
            }
        }
        match &self.globals[place.global] {
            Value::Unset => Err(format!("Undefined variable '{}'", name)),
            _ => Ok(&mut self.globals[place.global]),
        }
    }

    fn write(&mut self, place: &Place, frame: &mut Frame, value: Value) -> Outcome<()> {
        match place.locals.first() {
            Some(&slot) => frame.locals[slot] = value,
            None => {
                if Some(place.global) == self.args_slot {
                    return Err(format!("Cannot reassign {} (system-provided immutable value)", self.names[place.global]));
                }
                self.globals[place.global] = value;
            }
        }
        Ok(())
    }

    fn array_mut<'f>(&'f mut self, place: &Place, frame: &'f mut Frame, name: &str) -> Outcome<&'f mut Vec<Value>> {
        match self.slot_mut(place, frame, name)? {
            Value::List(items) => Ok(Rc::make_mut(items)),
            _ => Err(format!("Variable '{}' is not an array", name)),
        }
    }

    /// The program a call by name reaches: the youngest binding holding one.
    fn callee(&self, place: &Place, frame: &Frame, name: &str) -> Outcome<Rc<Program>> {
        for &slot in &place.locals {
            if let Value::Program(p) = &frame.locals[slot] {
                return Ok(p.clone());
            }
        }
        match &self.globals[place.global] {
            Value::Program(p) => Ok(p.clone()),
            Value::Unset => Err(format!("Unknown function: {}", name)),
            _ => Err(format!("'{}' is not a function", name)),
        }
    }

    /// Run a program with its arguments; its result is the value returned,
    /// or the last expression statement's, or what it assigned to its own
    /// name where the language says so.
    pub fn run(&mut self, program: &Rc<Program>, args: Vec<Value>) -> Outcome<Value> {
        if program.params.len() != args.len() {
            return Err(format!("Function {} expects {} arguments, got {}", program.name, program.params.len(), args.len()));
        }
        let memoized = self.memo_slot.map_or(false, |slot| matches!(self.globals[slot], Value::Bool(true)));
        let key = memoized.then(|| {
            let mut key = program.name.clone();
            key.push('(');
            for a in &args {
                a.fingerprint(&mut key);
            }
            key
        });
        if let Some(cached) = key.as_ref().and_then(|k| self.cache.get(k)) {
            return Ok(cached.clone());
        }
        let mut frame = Frame { locals: vec![Value::Unset; program.slots], result: Value::Null };
        for (slot, arg) in args.into_iter().enumerate() {
            frame.locals[slot] = arg;
        }
        let result = self.execute(program, &mut frame)?;
        if let Some(key) = key {
            self.cache.insert(key, result.clone());
        }
        Ok(result)
    }

    fn execute(&mut self, program: &Rc<Program>, frame: &mut Frame) -> Outcome<Value> {
        let words = &program.words;
        let mut pc = 0;
        while pc < words.len() {
            match &words[pc] {
                Word::Lit(v) => self.stack.push(v.clone()),
                Word::Load(place, name) => {
                    let v = self.read(place, frame, name)?;
                    self.stack.push(v);
                }
                Word::Store(place) => {
                    let v = self.pop()?;
                    self.write(place, frame, v)?;
                }
                Word::PutAt(place, name) => {
                    let v = self.pop()?;
                    let at = index_of(&self.pop()?)?;
                    let items = self.array_mut(place, frame, name)?;
                    if at >= items.len() {
                        return Err(format!("Array index {} out of bounds (length: {})", at, items.len()));
                    }
                    items[at] = v;
                    self.stack.push(Value::Null);
                }
                Word::Append(place, name) => {
                    let v = self.pop()?;
                    self.array_mut(place, frame, name)?.push(v);
                    self.stack.push(Value::Null);
                }
                Word::Forget { locals, globals } => {
                    for &slot in locals {
                        frame.locals[slot] = Value::Unset;
                    }
                    for &slot in globals {
                        self.globals[slot] = Value::Unset;
                    }
                }
                Word::Op(op) => {
                    let result = match op {
                        Op::Not => {
                            let v = self.pop()?;
                            Value::Bool(!v.truthy())
                        }
                        Op::Neg => {
                            let v = self.pop()?;
                            // Derived: -x is 0 - x, so a real keeps its precision.
                            match number::arith(Arith::Sub, &Value::Int(0), &v) {
                                Some(r) => r?,
                                None => return Err("Cannot negate non-numeric value".to_string()),
                            }
                        }
                        op => {
                            let b = self.pop()?;
                            let a = self.pop()?;
                            self.binary(*op, &a, &b)?
                        }
                    };
                    self.stack.push(result);
                }
                Word::Truth => {
                    let v = self.pop()?;
                    self.stack.push(Value::Bool(v.truthy()));
                }
                Word::Call { place, name, argc } => {
                    let args = self.take(*argc)?;
                    let callee = self.callee(place, frame, name)?;
                    let result = self.run(&callee, args)?;
                    self.stack.push(result);
                }
                Word::Apply { builtin, name, argc } => {
                    let args = self.take(*argc)?;
                    let result = self.apply(*builtin, name, &args)?;
                    self.stack.push(result);
                }
                Word::Jump(to) => {
                    pc = *to;
                    continue;
                }
                Word::Unless(to) => {
                    if !self.pop()?.truthy() {
                        pc = *to;
                        continue;
                    }
                }
                Word::When(to) => {
                    if self.pop()?.truthy() {
                        pc = *to;
                        continue;
                    }
                }
                Word::Return => return self.pop(),
                Word::Exit => break,
                Word::Result => frame.result = self.pop()?,
                Word::Dup => {
                    let v = self.pop()?;
                    self.stack.push(v.clone());
                    self.stack.push(v);
                }
                Word::Drop => {
                    self.pop()?;
                }
                Word::Swap => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.stack.push(b);
                    self.stack.push(a);
                }
                Word::Over => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.stack.push(a.clone());
                    self.stack.push(b);
                    self.stack.push(a);
                }
                Word::Rot => {
                    let c = self.pop()?;
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.stack.push(b);
                    self.stack.push(c);
                    self.stack.push(a);
                }
                Word::Eval => match self.pop()? {
                    Value::Program(p) => {
                        self.run(&p, Vec::new())?;
                    }
                    _ => return Err("eval needs a program".to_string()),
                },
                Word::Run(place, name) => match self.read(place, frame, name)? {
                    Value::Program(p) => {
                        self.run(&p, Vec::new())?;
                    }
                    value => self.stack.push(value),
                },
                Word::Mark => self.marks.push(self.stack.len()),
                Word::Gather => {
                    let mark = self.marks.pop().ok_or_else(|| "Stack underflow".to_string())?;
                    if mark > self.stack.len() {
                        return Err("Stack underflow".to_string());
                    }
                    let items = self.stack.split_off(mark);
                    self.stack.push(Value::list(items));
                }
                Word::Collect(n) => {
                    let items = self.take(*n)?;
                    self.stack.push(Value::list(items));
                }
            }
            pc += 1;
        }
        // Fell off the end, or left with Exit.
        if self.lang.result_by_name {
            if let Some(slot) = program.names.iter().position(|n| *n == program.name) {
                match &frame.locals[slot] {
                    Value::Unset | Value::Program(_) => {}
                    value => return Ok(value.clone()),
                }
            }
        }
        Ok(std::mem::replace(&mut frame.result, Value::Null))
    }

    // ---------- operations ----------

    fn binary(&self, op: Op, a: &Value, b: &Value) -> Outcome<Value> {
        let spelling = self.spelling();
        Ok(match op {
            Op::And => Value::Bool(a.truthy() && b.truthy()),
            Op::Or => Value::Bool(a.truthy() || b.truthy()),
            Op::Eq => Value::Bool(a.same(b)),
            Op::Ne => Value::Bool(!a.same(b)),
            Op::Concat => Value::text(&format!("{}{}", a.render(&spelling), b.render(&spelling))),
            Op::Index => self.index(a, b)?,
            Op::Add if matches!(a, Value::Text(_)) || matches!(b, Value::Text(_)) => {
                Value::text(&format!("{}{}", a.render(&spelling), b.render(&spelling)))
            }
            Op::Add | Op::Sub | Op::Mul | Op::Div | Op::RealDiv | Op::Quot | Op::Rem | Op::Pow => {
                let arith = match op {
                    Op::Add => Arith::Add,
                    Op::Sub => Arith::Sub,
                    Op::Mul => Arith::Mul,
                    Op::Div => Arith::Div,
                    Op::RealDiv => Arith::RealDiv,
                    Op::Quot => Arith::Quot,
                    Op::Rem => Arith::Rem,
                    _ => Arith::Pow,
                };
                match number::arith(arith, a, b) {
                    Some(result) => result?,
                    None => match arith {
                        // The closed operations coerce booleans and null.
                        Arith::Add => Value::integer(a.as_integer()? + b.as_integer()?),
                        Arith::Sub => Value::integer(a.as_integer()? - b.as_integer()?),
                        Arith::Mul => Value::integer(a.as_integer()? * b.as_integer()?),
                        Arith::Div | Arith::RealDiv => return Err("Division requires numeric operands".to_string()),
                        Arith::Quot => return Err("Integer quotient requires numeric operands".to_string()),
                        Arith::Rem => return Err("Modulo requires numeric operands".to_string()),
                        Arith::Pow => return Err("Exponentiation requires numeric operands".to_string()),
                    },
                }
            }
            Op::Lt | Op::Le | Op::Gt | Op::Ge => {
                // Derived from less-than: a > b is b < a, a <= b is not b < a, a >= b is not a < b.
                let less = |x: &Value, y: &Value| -> Outcome<bool> {
                    match number::less(x, y) {
                        Some(r) => Ok(r),
                        None => Ok(x.as_integer()? < y.as_integer()?),
                    }
                };
                Value::Bool(match op {
                    Op::Lt => less(a, b)?,
                    Op::Gt => less(b, a)?,
                    Op::Le => !less(b, a)?,
                    _ => !less(a, b)?,
                })
            }
            Op::Not | Op::Neg => unreachable!("unary"),
        })
    }

    fn index(&self, target: &Value, at: &Value) -> Outcome<Value> {
        let at = index_of(at)?;
        match target {
            Value::List(items) => {
                items.get(at).cloned().ok_or_else(|| format!("Array index {} out of bounds (length: {})", at, items.len()))
            }
            Value::Text(s) if self.lang.index_text => s
                .chars()
                .nth(at)
                .map(|c| Value::text(&c.to_string()))
                .ok_or_else(|| format!("String index {} out of bounds (length: {})", at, s.chars().count())),
            _ => Err("Cannot index non-array value".to_string()),
        }
    }

    // ---------- builtins ----------

    /// Text for print and write: the values joined by spaces, or the first,
    /// a template holding the definition's placeholders, filled from the rest.
    fn render(&self, values: &[Value]) -> String {
        let spelling = self.spelling();
        let holes = &self.lang.placeholders;
        let next_hole = |s: &str| holes.iter().filter_map(|h| s.find(h.as_str()).map(|at| (at, h.len()))).min();
        if let (Some(Value::Text(template)), true) = (values.first(), values.len() > 1) {
            if next_hole(template).is_some() {
                let mut out = String::new();
                let mut rest = values[1..].iter();
                let mut s: &str = template;
                while let Some((at, width)) = next_hole(s) {
                    out.push_str(&s[..at]);
                    match rest.next() {
                        Some(v) => out.push_str(&v.render(&spelling)),
                        None => out.push_str(&s[at..at + width]),
                    }
                    s = &s[at + width..];
                }
                out.push_str(s);
                return out;
            }
        }
        values.iter().map(|v| v.render(&spelling)).collect::<Vec<_>>().join(" ")
    }

    fn apply(&mut self, builtin: Builtin, name: &str, args: &[Value]) -> Outcome<Value> {
        let spelling = self.spelling();
        let count = |n: usize| -> Outcome<()> {
            if args.len() == n {
                Ok(())
            } else {
                Err(format!("{}() expects {} argument{}, got {}", name, n, if n == 1 { "" } else { "s" }, args.len()))
            }
        };
        Ok(match builtin {
            Builtin::Emit => {
                count(1)?;
                match &args[0] {
                    Value::Text(s) => print!("{}", s),
                    _ => return Err(format!("{}() requires a string argument", name)),
                }
                Value::Null
            }
            Builtin::Print => {
                println!("{}", self.render(args));
                Value::Null
            }
            Builtin::Write => {
                print!("{}", self.render(args));
                Value::Null
            }
            Builtin::Range => return Err(format!("{}() spells a range, which belongs in a for loop", name)),
            Builtin::Real => {
                if args.is_empty() || args.len() > 2 {
                    return Err(format!("{}() expects 1 or 2 arguments, got {}", name, args.len()));
                }
                let digits = match args.get(1) {
                    None => number::DEFAULT_DIGITS,
                    Some(Value::Int(n)) if *n >= 0 => *n as usize,
                    Some(Value::Int(_)) | Some(Value::Big(_)) => return Err("Precision must be a positive integer".to_string()),
                    Some(_) => return Err("Precision argument must be an integer".to_string()),
                };
                number::to_real(&args[0], digits).ok_or_else(|| format!("{}() requires a number, rational, or real argument", name))?
            }
            Builtin::Precision => {
                count(1)?;
                match number::digits_of(&args[0]) {
                    Some(d) => Value::Int(d as i64),
                    None => return Err(format!("{}() requires a real argument", name)),
                }
            }
            Builtin::ToText => {
                count(1)?;
                Value::text(&args[0].render(&spelling))
            }
            Builtin::ToInt => {
                count(1)?;
                match number::parts(&args[0]) {
                    Some((top, bottom)) => Value::integer(top / bottom),
                    None => return Err(format!("{}() requires a number argument", name)),
                }
            }
            Builtin::ToReal => {
                count(1)?;
                match &args[0] {
                    v @ Value::Real(_) => v.clone(),
                    v => number::to_real(v, number::DEFAULT_DIGITS).ok_or_else(|| format!("{}() requires a number argument", name))?,
                }
            }
            Builtin::Len => {
                count(1)?;
                match &args[0] {
                    Value::Text(s) => Value::Int(s.chars().count() as i64),
                    Value::List(items) => Value::Int(items.len() as i64),
                    _ => return Err(format!("{}() requires a string or array argument", name)),
                }
            }
            Builtin::CharAt => {
                count(2)?;
                match (&args[0], &args[1]) {
                    (Value::Text(s), Value::Int(i)) => match usize::try_from(*i).ok().and_then(|i| s.chars().nth(i)) {
                        Some(c) => Value::text(&c.to_string()),
                        None => return Err(format!("{} index out of bounds", name)),
                    },
                    (Value::Text(_), _) => return Err(format!("{}() second argument must be an integer", name)),
                    _ => return Err(format!("{}() first argument must be a string", name)),
                }
            }
            Builtin::Ord => {
                count(1)?;
                match &args[0] {
                    Value::Text(s) => match s.chars().next() {
                        Some(c) => Value::Int(c as i64),
                        None => return Err(format!("{}() requires a non-empty string", name)),
                    },
                    _ => return Err(format!("{}() requires a string argument", name)),
                }
            }
            Builtin::Chr => {
                count(1)?;
                let code = match &args[0] {
                    Value::Int(n) => u32::try_from(*n).ok(),
                    Value::Big(_) => None,
                    _ => return Err(format!("{}() requires an integer argument", name)),
                }
                .ok_or_else(|| format!("{}() argument must be a non-negative integer within valid Unicode range", name))?;
                match char::from_u32(code) {
                    Some(c) => Value::text(&c.to_string()),
                    None => return Err(format!("{}() argument {} is not a valid Unicode code point", name, code)),
                }
            }
            Builtin::Error => {
                count(1)?;
                match &args[0] {
                    Value::Text(s) => return Err(s.to_string()),
                    _ => return Err(format!("{}() argument must be a string", name)),
                }
            }
            Builtin::Kind => {
                count(1)?;
                match args[0].kind() {
                    Some(k) => Value::Kind(k),
                    None => return Err(format!("{}(): unknown value type", name)),
                }
            }
            Builtin::Num => {
                count(1)?;
                match number::parts(&args[0]) {
                    Some((top, _)) => Value::integer(top),
                    None => return Err(format!("{}() requires a number argument", name)),
                }
            }
            Builtin::Den => {
                count(1)?;
                match number::parts(&args[0]) {
                    Some((_, bottom)) => Value::integer(bottom),
                    None => return Err(format!("{}() requires a number argument", name)),
                }
            }
            Builtin::Get => {
                count(2)?;
                self.index(&args[0], &args[1])?
            }
            Builtin::Extern => self.extern_call(name, args)?,
            Builtin::Push | Builtin::Put => unreachable!("compiled to Append and PutAt"),
        })
    }

    /// The external capabilities of docs/LUMEN_LANGUAGE_EXTERN_SYSTEM.md.
    fn extern_call(&self, name: &str, args: &[Value]) -> Outcome<Value> {
        let spelling = self.spelling();
        let target = match args.first() {
            Some(Value::Text(s)) => s.to_string(),
            Some(_) => return Err(format!("First argument to {} must be a string (function name)", name)),
            None => return Err(format!("{} requires at least one argument (function name)", name)),
        };
        let rest = &args[1..];
        match target.as_str() {
            "print_native" | "debug_info" | "value_type" => {
                if rest.len() != 1 {
                    return Err(format!("{} expects 1 argument, got {}", target, rest.len()));
                }
                let v = &rest[0];
                match target.as_str() {
                    "print_native" => println!("{}", v.render(&spelling)),
                    "debug_info" => eprintln!("[DEBUG] {}", v.render(&spelling)),
                    _ => {
                        let code = match v {
                            Value::Int(_) | Value::Big(_) | Value::Ratio(_) | Value::Real(_) => 0,
                            Value::Bool(_) => 1,
                            Value::Text(_) => 2,
                            _ => return Err("Unknown value type".to_string()),
                        };
                        return Ok(Value::Int(code));
                    }
                }
                Ok(v.clone())
            }
            other => Err(format!("Unknown external function: {}", other)),
        }
    }
}

fn index_of(value: &Value) -> Outcome<usize> {
    match value {
        Value::Int(n) => usize::try_from(*n).map_err(|_| "Array index out of bounds".to_string()),
        Value::Big(n) => n.to_usize().ok_or_else(|| "Array index out of bounds".to_string()),
        _ => Err("Array index must be a number".to_string()),
    }
}

/// The kind meta-value a system name stands for.
pub fn kind_value(kind: Kind) -> Value {
    Value::Kind(kind)
}

/// The default precision as a value, for the system binding.
pub fn default_precision() -> Value {
    Value::Int(number::DEFAULT_DIGITS as i64)
}
