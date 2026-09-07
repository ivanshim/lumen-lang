// The machine: one loop over the words of a program, one data stack for
// the whole run. A call runs the callee's words on the same stack with a
// fresh set of local slots; globals are one table.

use std::collections::HashMap;
use std::rc::Rc;

use num_traits::ToPrimitive;

use crate::language::Def;
use crate::numbers::{self, Calc};
use crate::values::{Kind, Spelling, Value};
use crate::words::{Arg, Native, Op, Program, Slot, Word};

pub struct Machine<'a> {
    def: &'a Def,
    globals: Vec<Value>,
    names: Vec<String>,
    stack: Vec<Value>,
    cache: HashMap<String, Value>,
    args_slot: Option<usize>,
    memo_slot: Option<usize>,
}

type Outcome<T> = Result<T, String>;

impl<'a> Machine<'a> {
    pub fn new(def: &'a Def, names: Vec<String>) -> Machine<'a> {
        let find = |wanted: &Option<String>| wanted.as_ref().and_then(|w| names.iter().position(|n| n == w));
        Machine {
            def,
            globals: vec![Value::Empty; names.len()],
            stack: Vec::new(),
            cache: HashMap::new(),
            args_slot: find(&def.args_name),
            memo_slot: find(&def.memo_name),
            names,
        }
    }

    pub fn set_global(&mut self, name: &str, v: Value) {
        if let Some(i) = self.names.iter().position(|n| n == name) {
            self.globals[i] = v;
        }
    }

    pub fn global(&self, name: &str) -> Option<&Value> {
        let i = self.names.iter().position(|n| n == name)?;
        match &self.globals[i] {
            Value::Empty => None,
            v => Some(v),
        }
    }

    fn spelling(&self) -> Spelling<'a> {
        let word = |list: &'a [String], fallback: &'a str| list.first().map_or(fallback, String::as_str);
        Spelling { yes: word(&self.def.yes, "true"), no: word(&self.def.no, "false"), none: word(&self.def.none, "null") }
    }

    fn pop(&mut self) -> Outcome<Value> {
        self.stack.pop().ok_or_else(|| "Stack underflow".to_string())
    }

    fn pop_many(&mut self, n: usize) -> Outcome<Vec<Value>> {
        if self.stack.len() < n {
            return Err("Stack underflow".to_string());
        }
        let at = self.stack.len() - n;
        Ok(self.stack.split_off(at))
    }

    /// A load: the first local slot holding a value, else the global. A
    /// taking load moves the value out and leaves a hole.
    fn fetch(&mut self, slot: &Slot, frame: &mut [Value]) -> Outcome<Value> {
        for &s in &slot.locals {
            if !matches!(frame[s], Value::Empty) {
                return Ok(if slot.take { std::mem::replace(&mut frame[s], Value::Hole) } else { frame[s].clone() });
            }
        }
        let g = &mut self.globals[slot.global];
        match g {
            Value::Empty if slot.take => Err(format!("Undefined variable '{}'", slot.name)),
            Value::Empty => Err(format!("Undefined variable: {}", slot.name)),
            _ if slot.take => Ok(std::mem::replace(g, Value::Hole)),
            v => Ok(v.clone()),
        }
    }

    /// The cell a binding lives in, for reading in place.
    fn cell<'f>(&'f self, slot: &Slot, frame: &'f [Value]) -> Outcome<&'f Value> {
        for &s in &slot.locals {
            if !matches!(frame[s], Value::Empty) {
                return Ok(&frame[s]);
            }
        }
        match &self.globals[slot.global] {
            Value::Empty => Err(format!("Undefined variable: {}", slot.name)),
            v => Ok(v),
        }
    }

    fn cell_mut<'f>(&'f mut self, slot: &Slot, frame: &'f mut [Value]) -> Outcome<&'f mut Value> {
        for &s in &slot.locals {
            if !matches!(frame[s], Value::Empty) {
                return Ok(&mut frame[s]);
            }
        }
        match &self.globals[slot.global] {
            Value::Empty => Err(format!("Undefined variable: {}", slot.name)),
            _ => Ok(&mut self.globals[slot.global]),
        }
    }

    /// A store: into the hole a taking load left, if one is addressed;
    /// else the first local, or the global when there is none.
    fn assign(&mut self, slot: &Slot, frame: &mut [Value], v: Value) -> Outcome<()> {
        for &s in &slot.locals {
            if matches!(frame[s], Value::Hole) {
                frame[s] = v;
                return Ok(());
            }
        }
        if matches!(self.globals[slot.global], Value::Hole) {
            self.globals[slot.global] = v;
            return Ok(());
        }
        match slot.locals.first() {
            Some(&s) => frame[s] = v,
            None => {
                if Some(slot.global) == self.args_slot {
                    return Err(format!("Cannot reassign {} (system-provided immutable value)", slot.name));
                }
                self.globals[slot.global] = v;
            }
        }
        Ok(())
    }

    /// Run a program on its arguments. A function's result, the value it
    /// returned or the last expression statement's, or what it assigned to
    /// its own name where the language says so, is pushed; a postfix
    /// program leaves what it pushed.
    pub fn call(&mut self, program: &Rc<Program>, args: Vec<Value>) -> Outcome<()> {
        let n = args.len();
        self.stack.extend(args);
        self.call_from_stack(program, n)
    }

    /// Cycle 4: the arguments are the top `n` of the stack and move
    /// straight into the frame, one allocation instead of two.
    pub fn call_from_stack(&mut self, program: &Rc<Program>, n: usize) -> Outcome<()> {
        if program.params.len() != n {
            return Err(format!("Function {} expects {} arguments, got {}", program.name, program.params.len(), n));
        }
        if self.stack.len() < n {
            return Err("Stack underflow".to_string());
        }
        let at = self.stack.len() - n;
        let memoized = program.yields && self.memo_slot.map_or(false, |s| matches!(self.globals[s], Value::Bool(true)));
        let key = memoized.then(|| {
            let mut key = format!("{}(", program.name);
            for a in &self.stack[at..] {
                a.key(&mut key);
            }
            key
        });
        if let Some(hit) = key.as_ref().and_then(|k| self.cache.get(k)) {
            self.stack.truncate(at);
            self.stack.push(hit.clone());
            return Ok(());
        }
        let mut frame: Vec<Value> = Vec::with_capacity(program.names.len());
        frame.extend(self.stack.drain(at..));
        frame.resize(program.names.len(), Value::Empty);
        let base = self.stack.len();
        self.execute(program, &mut frame)?;
        if !program.yields {
            return Ok(());
        }
        let mut result = if self.stack.len() > base { self.pop()? } else { Value::Null };
        if self.def.result_by_name {
            if let Some(s) = program.names.iter().position(|n| *n == program.name) {
                match &frame[s] {
                    Value::Empty | Value::Hole | Value::Program(_) => {}
                    v => result = v.clone(),
                }
            }
        }
        self.stack.truncate(base);
        if let Some(key) = key {
            self.cache.insert(key, result.clone());
        }
        self.stack.push(result);
        Ok(())
    }

    fn execute(&mut self, program: &Rc<Program>, frame: &mut [Value]) -> Outcome<()> {
        let words = &program.words;
        let mut pc = 0;
        while pc < words.len() {
            match &words[pc] {
                Word::Lit(v) => self.stack.push(v.clone()),
                Word::Load(slot) => {
                    let v = self.fetch(slot, frame)?;
                    self.stack.push(v);
                }
                Word::Store(slot) => {
                    let v = self.pop()?;
                    self.assign(slot, frame, v)?;
                }
                Word::Apply(op, argc) => self.apply(op, *argc)?,
                Word::Unless(to) => {
                    if !self.pop()?.truthy() {
                        pc = *to;
                        continue;
                    }
                }
                Word::UnlessLess { a, b, to } => {
                    // Operands are read in place: no clone for a binding.
                    let bt = if matches!(b, Arg::Top) { Some(self.pop()?) } else { None };
                    let at = if matches!(a, Arg::Top) { Some(self.pop()?) } else { None };
                    let bv: &Value = match b {
                        Arg::Top => bt.as_ref().expect("popped"),
                        Arg::Lit(v) => v,
                        Arg::Slot(s) => self.cell(s, frame)?,
                    };
                    let av: &Value = match a {
                        Arg::Top => at.as_ref().expect("popped"),
                        Arg::Lit(v) => v,
                        Arg::Slot(s) => self.cell(s, frame)?,
                    };
                    let less = match (av, bv) {
                        (Value::Int(x), Value::Int(y)) => x < y,
                        _ => self.binary(&Op::Lt, av, bv)?.truthy(),
                    };
                    if !less {
                        pc = *to;
                        continue;
                    }
                }
                Word::Jump(to) => {
                    pc = *to;
                    continue;
                }
                Word::Arith { op, a, b, into } => {
                    let bt = if matches!(b, Arg::Top) { Some(self.pop()?) } else { None };
                    let at = if matches!(a, Arg::Top) { Some(self.pop()?) } else { None };
                    let bv: &Value = match b {
                        Arg::Top => bt.as_ref().expect("popped"),
                        Arg::Lit(v) => v,
                        Arg::Slot(s) => self.cell(s, frame)?,
                    };
                    let av: &Value = match a {
                        Arg::Top => at.as_ref().expect("popped"),
                        Arg::Lit(v) => v,
                        Arg::Slot(s) => self.cell(s, frame)?,
                    };
                    let fast = match (av, bv) {
                        (Value::Int(x), Value::Int(y)) => match op {
                            Op::Add => x.checked_add(*y).map(Value::Int),
                            Op::Sub => x.checked_sub(*y).map(Value::Int),
                            Op::Mul => x.checked_mul(*y).map(Value::Int),
                            Op::Lt => Some(Value::Bool(x < y)),
                            Op::Le => Some(Value::Bool(x <= y)),
                            Op::Gt => Some(Value::Bool(x > y)),
                            Op::Ge => Some(Value::Bool(x >= y)),
                            Op::Eq => Some(Value::Bool(x == y)),
                            Op::Ne => Some(Value::Bool(x != y)),
                            Op::Rem if *y != 0 => x.checked_rem(*y).map(Value::Int),
                            Op::Quot if *y != 0 => x.checked_div(*y).map(Value::Int),
                            _ => None,
                        },
                        _ => None,
                    };
                    let r = match fast {
                        Some(v) => v,
                        None => self.binary(op, av, bv)?,
                    };
                    match into {
                        Some(slot) => self.assign(slot, frame, r)?,
                        None => self.stack.push(r),
                    }
                }
                Word::Incr { slot, by } => {
                    let cell = self.cell_mut(slot, frame)?;
                    let fast = match (&*cell, by) {
                        (Value::Int(x), Value::Int(k)) => x.checked_add(*k),
                        _ => None,
                    };
                    match fast {
                        Some(sum) => *cell = Value::Int(sum),
                        None => {
                            let v = cell.clone();
                            let r = self.binary(&Op::Add, &v, by)?;
                            self.assign(slot, frame, r)?;
                        }
                    }
                }
            }
            pc += 1;
        }
        Ok(())
    }

    fn apply(&mut self, op: &Op, argc: usize) -> Outcome<()> {
        let result = match op {
            Op::Not => Value::Bool(!self.pop()?.truthy()),
            Op::Truth => Value::Bool(self.pop()?.truthy()),
            Op::Neg => {
                // 0 - x, so a real keeps its precision.
                let v = self.pop()?;
                match numbers::compute(Calc::Sub, &Value::Int(0), &v) {
                    Some(r) => r?,
                    None => return Err("Cannot negate non-numeric value".to_string()),
                }
            }
            Op::Call(name) => {
                let callee = self.pop()?;
                return match callee {
                    Value::Program(p) => self.call_from_stack(&p, argc - 1),
                    _ => Err(format!("'{}' is not a function", name)),
                };
            }
            Op::Eval => {
                return match self.pop()? {
                    Value::Program(p) => self.call(&p, Vec::new()),
                    _ => Err("eval needs a program".to_string()),
                };
            }
            Op::Run => {
                return match self.pop()? {
                    Value::Program(p) => self.call(&p, Vec::new()),
                    v => {
                        self.stack.push(v);
                        Ok(())
                    }
                };
            }
            Op::Array => Value::list(self.pop_many(argc)?),
            Op::Gather => {
                let mark = self.stack.iter().rposition(|v| matches!(v, Value::Mark)).ok_or("Stack underflow")?;
                let items = self.stack.split_off(mark + 1);
                self.stack.pop();
                Value::list(items)
            }
            Op::Native(native, name) => {
                let args = self.pop_many(argc)?;
                self.native(*native, name, args)?
            }
            binary => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.binary(binary, &a, &b)?
            }
        };
        self.stack.push(result);
        Ok(())
    }

    // ---------- operations ----------

    fn binary(&self, op: &Op, a: &Value, b: &Value) -> Outcome<Value> {
        let sp = self.spelling();
        let joined = || Value::str(&format!("{}{}", a.show(&sp), b.show(&sp)));
        Ok(match op {
            Op::And => Value::Bool(a.truthy() && b.truthy()),
            Op::Or => Value::Bool(a.truthy() || b.truthy()),
            Op::Eq => Value::Bool(a.same(b)),
            Op::Ne => Value::Bool(!a.same(b)),
            Op::Concat => joined(),
            Op::Index => self.index(a, b)?,
            Op::Add if matches!(a, Value::Str(_)) || matches!(b, Value::Str(_)) => joined(),
            Op::Add | Op::Sub | Op::Mul | Op::Div | Op::RealDiv | Op::Quot | Op::Rem | Op::Pow => {
                let calc = match op {
                    Op::Add => Calc::Add,
                    Op::Sub => Calc::Sub,
                    Op::Mul => Calc::Mul,
                    Op::Div => Calc::Div,
                    Op::RealDiv => Calc::RealDiv,
                    Op::Quot => Calc::Quot,
                    Op::Rem => Calc::Rem,
                    _ => Calc::Pow,
                };
                match numbers::compute(calc, a, b) {
                    Some(r) => r?,
                    // The closed operations coerce booleans and null.
                    None => match calc {
                        Calc::Add => Value::whole(a.as_whole()? + b.as_whole()?),
                        Calc::Sub => Value::whole(a.as_whole()? - b.as_whole()?),
                        Calc::Mul => Value::whole(a.as_whole()? * b.as_whole()?),
                        Calc::Div | Calc::RealDiv => return Err("Division requires numeric operands".to_string()),
                        Calc::Quot => return Err("Integer quotient requires numeric operands".to_string()),
                        Calc::Rem => return Err("Modulo requires numeric operands".to_string()),
                        Calc::Pow => return Err("Exponentiation requires numeric operands".to_string()),
                    },
                }
            }
            Op::Lt | Op::Le | Op::Gt | Op::Ge => {
                // From less-than alone: a > b is b < a, a <= b is not b < a.
                let below = |x: &Value, y: &Value| -> Outcome<bool> {
                    match numbers::compare(x, y) {
                        Some(order) => Ok(order == std::cmp::Ordering::Less),
                        None => Ok(x.as_whole()? < y.as_whole()?),
                    }
                };
                Value::Bool(match op {
                    Op::Lt => below(a, b)?,
                    Op::Gt => below(b, a)?,
                    Op::Le => !below(b, a)?,
                    _ => !below(a, b)?,
                })
            }
            other => unreachable!("{other:?} is not binary"),
        })
    }

    fn index(&self, target: &Value, at: &Value) -> Outcome<Value> {
        let i = position(at)?;
        match target {
            Value::List(items) => {
                items.get(i).cloned().ok_or_else(|| format!("Array index {} out of bounds (length: {})", i, items.len()))
            }
            Value::Str(s) if self.def.index_text => s
                .chars()
                .nth(i)
                .map(|c| Value::str(&c.to_string()))
                .ok_or_else(|| format!("String index {} out of bounds (length: {})", i, s.chars().count())),
            _ => Err("Cannot index non-array value".to_string()),
        }
    }

    // ---------- builtins ----------

    /// print and write: the values joined by spaces, or a template holding
    /// the definition's placeholders filled from the rest.
    fn text_of(&self, values: &[Value]) -> String {
        let sp = self.spelling();
        let holes = &self.def.placeholders;
        let hole_in = |s: &str| holes.iter().filter_map(|h| s.find(h.as_str()).map(|at| (at, h.len()))).min();
        if let (Some(Value::Str(template)), true) = (values.first(), values.len() > 1) {
            if hole_in(template).is_some() {
                let mut out = String::new();
                let mut fill = values[1..].iter();
                let mut s: &str = template;
                while let Some((at, width)) = hole_in(s) {
                    out.push_str(&s[..at]);
                    match fill.next() {
                        Some(v) => out.push_str(&v.show(&sp)),
                        None => out.push_str(&s[at..at + width]),
                    }
                    s = &s[at + width..];
                }
                out.push_str(s);
                return out;
            }
        }
        values.iter().map(|v| v.show(&sp)).collect::<Vec<_>>().join(" ")
    }

    fn native(&mut self, native: Native, name: &str, mut args: Vec<Value>) -> Outcome<Value> {
        let sp = self.spelling();
        let arity = |n: usize| -> Outcome<()> {
            if args.len() == n {
                return Ok(());
            }
            Err(format!("{}() expects {} argument{}, got {}", name, n, if n == 1 { "" } else { "s" }, args.len()))
        };
        Ok(match native {
            Native::Emit => {
                arity(1)?;
                let Value::Str(s) = &args[0] else { return Err(format!("{}() requires a string argument", name)) };
                print!("{}", s);
                Value::Null
            }
            Native::Print => {
                println!("{}", self.text_of(&args));
                Value::Null
            }
            Native::Write => {
                print!("{}", self.text_of(&args));
                Value::Null
            }
            Native::Range => return Err(format!("{}() spells a range, which belongs in a for loop", name)),
            Native::Real => {
                if args.is_empty() || args.len() > 2 {
                    return Err(format!("{}() expects 1 or 2 arguments, got {}", name, args.len()));
                }
                let places = match args.get(1) {
                    None => numbers::PLACES,
                    Some(Value::Int(n)) if *n >= 0 => *n as usize,
                    Some(Value::Int(_)) | Some(Value::Big(_)) => return Err("Precision must be a positive integer".to_string()),
                    Some(_) => return Err("Precision argument must be an integer".to_string()),
                };
                numbers::as_real(&args[0], places)
                    .ok_or_else(|| format!("{}() requires a number, rational, or real argument", name))?
            }
            Native::Precision => {
                arity(1)?;
                match &args[0] {
                    Value::Real(r) => Value::Int(r.places as i64),
                    _ => return Err(format!("{}() requires a real argument", name)),
                }
            }
            Native::Text => {
                arity(1)?;
                Value::str(&args[0].show(&sp))
            }
            Native::Int => {
                arity(1)?;
                let (p, q) = numbers::split(&args[0]).ok_or_else(|| format!("{}() requires a number argument", name))?;
                Value::whole(p / q)
            }
            Native::ToReal => {
                arity(1)?;
                match &args[0] {
                    v @ Value::Real(_) => v.clone(),
                    v => numbers::as_real(v, numbers::PLACES).ok_or_else(|| format!("{}() requires a number argument", name))?,
                }
            }
            Native::Len => {
                arity(1)?;
                match &args[0] {
                    Value::Str(s) => Value::Int(s.chars().count() as i64),
                    Value::List(items) => Value::Int(items.len() as i64),
                    _ => return Err(format!("{}() requires a string or array argument", name)),
                }
            }
            Native::CharAt => {
                arity(2)?;
                match (&args[0], &args[1]) {
                    (Value::Str(s), Value::Int(i)) => match usize::try_from(*i).ok().and_then(|i| s.chars().nth(i)) {
                        Some(c) => Value::str(&c.to_string()),
                        None => return Err(format!("{} index out of bounds", name)),
                    },
                    (Value::Str(_), _) => return Err(format!("{}() second argument must be an integer", name)),
                    _ => return Err(format!("{}() first argument must be a string", name)),
                }
            }
            Native::Ord => {
                arity(1)?;
                let Value::Str(s) = &args[0] else { return Err(format!("{}() requires a string argument", name)) };
                match s.chars().next() {
                    Some(c) => Value::Int(c as i64),
                    None => return Err(format!("{}() requires a non-empty string", name)),
                }
            }
            Native::Chr => {
                arity(1)?;
                let code = match &args[0] {
                    Value::Int(n) => u32::try_from(*n).ok(),
                    Value::Big(_) => None,
                    _ => return Err(format!("{}() requires an integer argument", name)),
                };
                let code = code
                    .ok_or_else(|| format!("{}() argument must be a non-negative integer within valid Unicode range", name))?;
                match char::from_u32(code) {
                    Some(c) => Value::str(&c.to_string()),
                    None => return Err(format!("{}() argument {} is not a valid Unicode code point", name, code)),
                }
            }
            Native::Fail => {
                arity(1)?;
                return match &args[0] {
                    Value::Str(s) => Err(s.to_string()),
                    _ => Err(format!("{}() argument must be a string", name)),
                };
            }
            Native::Kind => {
                arity(1)?;
                match args[0].kind() {
                    Some(k) => Value::Kind(k),
                    None => return Err(format!("{}(): unknown value type", name)),
                }
            }
            Native::Num | Native::Den => {
                arity(1)?;
                let (p, q) = numbers::split(&args[0]).ok_or_else(|| format!("{}() requires a number argument", name))?;
                Value::whole(if native == Native::Num { p } else { q })
            }
            Native::Get => {
                arity(2)?;
                self.index(&args[0], &args[1])?
            }
            Native::Push => {
                arity(2)?;
                let Some(Value::List(mut items)) = args.pop() else {
                    return Err(format!("{}() requires an array", name));
                };
                let v = args.pop().expect("the value");
                Rc::make_mut(&mut items).push(v);
                Value::List(items)
            }
            Native::Put => {
                arity(3)?;
                let Some(Value::List(mut items)) = args.pop() else {
                    return Err(format!("{}() requires an array", name));
                };
                let v = args.pop().expect("the value");
                let i = position(&args[0])?;
                let list = Rc::make_mut(&mut items);
                if i >= list.len() {
                    return Err(format!("Array index {} out of bounds (length: {})", i, list.len()));
                }
                list[i] = v;
                Value::List(items)
            }
            Native::Extern => self.extern_call(name, &args)?,
        })
    }

    /// The external capabilities of docs/LUMEN_LANGUAGE_EXTERN_SYSTEM.md.
    fn extern_call(&self, name: &str, args: &[Value]) -> Outcome<Value> {
        let sp = self.spelling();
        let which = match args.first() {
            Some(Value::Str(s)) => s.to_string(),
            Some(_) => return Err(format!("First argument to {} must be a string (function name)", name)),
            None => return Err(format!("{} requires at least one argument (function name)", name)),
        };
        let rest = &args[1..];
        if !matches!(which.as_str(), "print_native" | "debug_info" | "value_type") {
            return Err(format!("Unknown external function: {}", which));
        }
        if rest.len() != 1 {
            return Err(format!("{} expects 1 argument, got {}", which, rest.len()));
        }
        let v = &rest[0];
        match which.as_str() {
            "print_native" => println!("{}", v.show(&sp)),
            "debug_info" => eprintln!("[DEBUG] {}", v.show(&sp)),
            _ => {
                return Ok(Value::Int(match v {
                    Value::Int(_) | Value::Big(_) | Value::Ratio(_) | Value::Real(_) => 0,
                    Value::Bool(_) => 1,
                    Value::Str(_) => 2,
                    _ => return Err("Unknown value type".to_string()),
                }))
            }
        }
        Ok(v.clone())
    }
}

fn position(v: &Value) -> Outcome<usize> {
    match v {
        Value::Int(n) => usize::try_from(*n).map_err(|_| "Array index out of bounds".to_string()),
        Value::Big(n) => n.to_usize().ok_or_else(|| "Array index out of bounds".to_string()),
        _ => Err("Array index must be a number".to_string()),
    }
}

pub fn kind_value(kind: Kind) -> Value {
    Value::Kind(kind)
}

pub fn default_precision() -> Value {
    Value::Int(numbers::PLACES as i64)
}
