// The engine: one loop over the words of a routine, one data stack for
// the whole run. A call runs the callee's words on the same stack with a
// fresh frame of cells; globals are one table. Fused words read their
// operands from cells by reference and never touch the stack for them.

use std::collections::HashMap;
use std::rc::Rc;

use num_traits::ToPrimitive;

use crate::lang::Lang;
use crate::arith::{self, Operation};
use crate::value::{Sort, Wording, Value};
use crate::code::{Operand, Builtin, Action, Routine, Cell, Instr};

pub struct Engine<'a> {
    lang: &'a Lang,
    world: Vec<Value>,
    idents: Vec<String>,
    data: Vec<Value>,
    memo: HashMap<String, Value>,
    /// The arguments of a builtin call, one buffer reused across calls.
    buffer: Vec<Value>,
    args_cell: Option<usize>,
    memo_cell: Option<usize>,
}

type Res<T> = Result<T, String>;

impl<'a> Engine<'a> {
    pub fn new(lang: &'a Lang, idents: Vec<String>) -> Engine<'a> {
        let find = |wanted: &Option<String>| wanted.as_ref().and_then(|w| idents.iter().position(|n| n == w));
        Engine {
            lang,
            world: vec![Value::Blank; idents.len()],
            data: Vec::new(),
            memo: HashMap::new(),
            buffer: Vec::new(),
            args_cell: find(&lang.args_binding),
            memo_cell: find(&lang.memo_binding),
            idents,
        }
    }

    pub fn define(&mut self, name: &str, v: Value) {
        if let Some(i) = self.idents.iter().position(|n| n == name) {
            self.world[i] = v;
        }
    }

    pub fn lookup(&self, name: &str) -> Option<&Value> {
        let i = self.idents.iter().position(|n| n == name)?;
        match &self.world[i] {
            Value::Blank => None,
            v => Some(v),
        }
    }

    fn wording(&self) -> Wording<'a> {
        let word = |list: &'a [String], fallback: &'a str| list.first().map_or(fallback, String::as_str);
        Wording { true_word: word(&self.lang.true_words, "true"), false_word: word(&self.lang.false_words, "false"), null_word: word(&self.lang.null_words, "null") }
    }

    fn drop_top(&mut self) -> Res<Value> {
        self.data.pop().ok_or_else(|| "Stack underflow".to_string())
    }

    fn drop_many(&mut self, n: usize) -> Res<Vec<Value>> {
        if self.data.len() < n {
            return Err("Stack underflow".to_string());
        }
        let at = self.data.len() - n;
        Ok(self.data.split_off(at))
    }

    /// A load: the first local slot holding a value, else the global. A
    /// taking load moves the value out and leaves a hole.
    fn load_cell(&mut self, slot: &Cell, frame: &mut [Value]) -> Res<Value> {
        for &s in &slot.near {
            if !matches!(frame[s], Value::Blank) {
                return Ok(if slot.moving { std::mem::replace(&mut frame[s], Value::Gap) } else { frame[s].clone() });
            }
        }
        let g = &mut self.world[slot.far];
        match g {
            Value::Blank if slot.moving => Err(format!("Undefined variable '{}'", slot.ident)),
            Value::Blank => Err(format!("Undefined variable: {}", slot.ident)),
            _ if slot.moving => Ok(std::mem::replace(g, Value::Gap)),
            v => Ok(v.clone()),
        }
    }

    /// The cell a binding lives in, for reading in place.
    fn peek_cell<'f>(&'f self, slot: &Cell, frame: &'f [Value]) -> Res<&'f Value> {
        for &s in &slot.near {
            if !matches!(frame[s], Value::Blank) {
                return Ok(&frame[s]);
            }
        }
        match &self.world[slot.far] {
            Value::Blank => Err(format!("Undefined variable: {}", slot.ident)),
            v => Ok(v),
        }
    }

    /// Two both read in place: a binding by reference, a constant
    /// from the word, a data value from the ones already popped.
    fn both<'f>(&'f self, a: &'f Operand, b: &'f Operand, at: &'f Option<Value>, bt: &'f Option<Value>, frame: &'f [Value]) -> Res<(&'f Value, &'f Value)> {
        let av: &Value = match a {
            Operand::Top => at.as_ref().expect("popped"),
            Operand::Const(v) => v,
            Operand::Cell(s) => self.peek_cell(s, frame)?,
        };
        let bv: &Value = match b {
            Operand::Top => bt.as_ref().expect("popped"),
            Operand::Const(v) => v,
            Operand::Cell(s) => self.peek_cell(s, frame)?,
        };
        Ok((av, bv))
    }

    fn peek_cell_mut<'f>(&'f mut self, slot: &Cell, frame: &'f mut [Value]) -> Res<&'f mut Value> {
        for &s in &slot.near {
            if !matches!(frame[s], Value::Blank) {
                return Ok(&mut frame[s]);
            }
        }
        match &self.world[slot.far] {
            Value::Blank => Err(format!("Undefined variable: {}", slot.ident)),
            _ => Ok(&mut self.world[slot.far]),
        }
    }

    /// A store: into the hole a taking load left, if one is addressed;
    /// else the first local, or the global when there is none.
    fn store_cell(&mut self, slot: &Cell, frame: &mut [Value], v: Value) -> Res<()> {
        for &s in &slot.near {
            if matches!(frame[s], Value::Gap) {
                frame[s] = v;
                return Ok(());
            }
        }
        if matches!(self.world[slot.far], Value::Gap) {
            self.world[slot.far] = v;
            return Ok(());
        }
        match slot.near.first() {
            Some(&s) => frame[s] = v,
            None => {
                if Some(slot.far) == self.args_cell {
                    return Err(format!("Cannot reassign {} (system-provided immutable value)", slot.ident));
                }
                self.world[slot.far] = v;
            }
        }
        Ok(())
    }

    /// Run a program on its arguments. A function's result, the value it
    /// returned or the last expression statement's, or what it assigned to
    /// its own name where the language says so, is pushed; a postfix
    /// program leaves what it pushed.
    pub fn invoke(&mut self, program: &Rc<Routine>, args: Vec<Value>) -> Res<()> {
        let n = args.len();
        self.data.extend(args);
        self.invoke_top(program, n)
    }

    /// A call whose arguments are the top `n` of the data stack: they move
    /// straight into the frame, one allocation instead of two.
    pub fn invoke_top(&mut self, program: &Rc<Routine>, n: usize) -> Res<()> {
        if program.formals.len() != n {
            return Err(format!("Function {} expects {} arguments, got {}", program.ident, program.formals.len(), n));
        }
        if self.data.len() < n {
            return Err("Stack underflow".to_string());
        }
        let at = self.data.len() - n;
        let memoized = program.returns_value && self.memo_cell.map_or(false, |s| matches!(self.world[s], Value::Flag(true)));
        let key = memoized.then(|| {
            let mut key = format!("{}(", program.ident);
            for a in &self.data[at..] {
                a.memo_key(&mut key);
            }
            key
        });
        if let Some(hit) = key.as_ref().and_then(|k| self.memo.get(k)) {
            self.data.truncate(at);
            self.data.push(hit.clone());
            return Ok(());
        }
        let mut frame: Vec<Value> = Vec::with_capacity(program.idents.len());
        frame.extend(self.data.drain(at..));
        frame.resize(program.idents.len(), Value::Blank);
        let base = self.data.len();
        self.run_instrs(program, &mut frame)?;
        if !program.returns_value {
            return Ok(());
        }
        let mut result = if self.data.len() > base { self.drop_top()? } else { Value::Null };
        if self.lang.named_result {
            if let Some(s) = program.idents.iter().position(|n| *n == program.ident) {
                match &frame[s] {
                    Value::Blank | Value::Gap | Value::Routine(_) => {}
                    v => result = v.clone(),
                }
            }
        }
        self.data.truncate(base);
        if let Some(key) = key {
            self.memo.insert(key, result.clone());
        }
        self.data.push(result);
        Ok(())
    }

    fn run_instrs(&mut self, program: &Rc<Routine>, frame: &mut [Value]) -> Res<()> {
        let instrs = &program.instrs;
        let mut pc = 0;
        while pc < instrs.len() {
            match &instrs[pc] {
                Instr::Const(v) => self.data.push(v.clone()),
                Instr::Read(slot) => {
                    let v = self.load_cell(slot, frame)?;
                    self.data.push(v);
                }
                Instr::Write(slot) => {
                    let v = self.drop_top()?;
                    self.store_cell(slot, frame, v)?;
                }
                Instr::Act(op, argc) => self.perform(op, *argc)?,
                Instr::Skip(to) => {
                    if !self.drop_top()?.is_true() {
                        pc = *to;
                        continue;
                    }
                }
                Instr::SkipCmp { op, a, b, to } => {
                    let bt = if matches!(b, Operand::Top) { Some(self.drop_top()?) } else { None };
                    let at = if matches!(a, Operand::Top) { Some(self.drop_top()?) } else { None };
                    let (av, bv) = self.both(a, b, &at, &bt, frame)?;
                    let holds = match (av, bv) {
                        (Value::Small(x), Value::Small(y)) => match op {
                            Action::Lt => x < y,
                            Action::Le => x <= y,
                            Action::Gt => x > y,
                            Action::Ge => x >= y,
                            Action::Eq => x == y,
                            _ => x != y,
                        },
                        _ => self.dyadic(op, av, bv)?.is_true(),
                    };
                    if !holds {
                        pc = *to;
                        continue;
                    }
                }
                Instr::Dyad { op, a, b } => {
                    let bt = if matches!(b, Operand::Top) { Some(self.drop_top()?) } else { None };
                    let at = if matches!(a, Operand::Top) { Some(self.drop_top()?) } else { None };
                    let (av, bv) = self.both(a, b, &at, &bt, frame)?;
                    let fast = match (av, bv) {
                        (Value::Small(x), Value::Small(y)) => match op {
                            Action::Add => x.checked_add(*y).map(Value::Small),
                            Action::Sub => x.checked_sub(*y).map(Value::Small),
                            Action::Mul => x.checked_mul(*y).map(Value::Small),
                            Action::Lt => Some(Value::Flag(x < y)),
                            Action::Le => Some(Value::Flag(x <= y)),
                            Action::Gt => Some(Value::Flag(x > y)),
                            Action::Ge => Some(Value::Flag(x >= y)),
                            Action::Eq => Some(Value::Flag(x == y)),
                            Action::Ne => Some(Value::Flag(x != y)),
                            Action::Mod if *y != 0 => x.checked_rem(*y).map(Value::Small),
                            Action::IntDiv if *y != 0 => x.checked_div(*y).map(Value::Small),
                            _ => None,
                        },
                        _ => None,
                    };
                    let r = match fast {
                        Some(v) => v,
                        None => self.dyadic(op, av, bv)?,
                    };
                    self.data.push(r);
                }
                Instr::Bump { slot, by } => {
                    let cell = self.peek_cell_mut(slot, frame)?;
                    let fast = match (&*cell, by) {
                        (Value::Small(x), Value::Small(k)) => x.checked_add(*k),
                        _ => None,
                    };
                    match fast {
                        Some(sum) => *cell = Value::Small(sum),
                        None => {
                            let v = cell.clone();
                            let r = self.dyadic(&Action::Add, &v, by)?;
                            self.store_cell(slot, frame, r)?;
                        }
                    }
                }
            }
            pc += 1;
        }
        Ok(())
    }

    fn perform(&mut self, op: &Action, argc: usize) -> Res<()> {
        let result = match op {
            Action::Not => Value::Flag(!self.drop_top()?.is_true()),
            Action::AsBool => Value::Flag(self.drop_top()?.is_true()),
            Action::Negate => {
                // 0 - x, so a real keeps its precision.
                let v = self.drop_top()?;
                match arith::calculate(Operation::Minus, &Value::Small(0), &v) {
                    Some(r) => r?,
                    None => return Err("Cannot negate non-numeric value".to_string()),
                }
            }
            Action::Invoke(name) => {
                let callee = self.drop_top()?;
                return match callee {
                    Value::Routine(p) => self.invoke_top(&p, argc - 1),
                    _ => Err(format!("'{}' is not a function", name)),
                };
            }
            Action::Evaluate => {
                return match self.drop_top()? {
                    Value::Routine(p) => self.invoke(&p, Vec::new()),
                    _ => Err("eval needs a program".to_string()),
                };
            }
            Action::Execute => {
                return match self.drop_top()? {
                    Value::Routine(p) => self.invoke(&p, Vec::new()),
                    v => {
                        self.data.push(v);
                        Ok(())
                    }
                };
            }
            Action::MakeArray => gathered(self.drop_many(argc)?, false),
            Action::MakeMap => gathered(self.drop_many(argc)?, true),
            Action::Tie => {
                let pair = self.drop_many(2)?;
                let mut pair = pair.into_iter();
                let (k, v) = (pair.next().expect("the key"), pair.next().expect("the value"));
                Value::Tie(Rc::new((k, v)))
            }
            Action::KeyAt | Action::ValueAt => {
                let pair = self.drop_many(2)?;
                let at = as_index(&pair[1])?;
                let key = matches!(op, Action::KeyAt);
                match &pair[0] {
                    Value::Array(items) => match items.get(at) {
                        Some(v) if !key => v.clone(),
                        Some(_) => Value::Small(at as i64),
                        None => return Err(format!("Array index {} out of bounds (length: {})", at, items.len())),
                    },
                    Value::Map(pairs) => match pairs.get(at) {
                        Some((k, v)) => if key { k.clone() } else { v.clone() },
                        None => return Err(format!("Array index {} out of bounds (length: {})", at, pairs.len())),
                    },
                    _ => return Err("Cannot walk a value that is not an array".to_string()),
                }
            }
            Action::AtEnd => return Err("An empty index belongs on the left of an assignment".to_string()),
            Action::Extent => match self.drop_top()? {
                Value::Array(items) => Value::Small(items.len() as i64),
                Value::Map(pairs) => Value::Small(pairs.len() as i64),
                _ => return Err("Cannot walk a value that is not an array".to_string()),
            },
            Action::Collect => {
                let mark = self.data.iter().rposition(|v| matches!(v, Value::Fence)).ok_or("Stack underflow")?;
                let items = self.data.split_off(mark + 1);
                self.data.pop();
                Value::array(items)
            }
            Action::Builtin(builtin, name) => {
                if self.data.len() < argc {
                    return Err("Stack underflow".to_string());
                }
                let at = self.data.len() - argc;
                let mut args = std::mem::take(&mut self.buffer);
                args.clear();
                args.extend(self.data.drain(at..));
                let result = self.builtin(*builtin, name, &mut args);
                self.buffer = args;
                result?
            }
            dyadic => {
                let b = self.drop_top()?;
                let a = self.drop_top()?;
                self.dyadic(dyadic, &a, &b)?
            }
        };
        self.data.push(result);
        Ok(())
    }

    // ---------- operations ----------

    fn dyadic(&self, op: &Action, a: &Value, b: &Value) -> Res<Value> {
        let sp = self.wording();
        let joined = || Value::text(&format!("{}{}", a.display(&sp), b.display(&sp)));
        Ok(match op {
            Action::And => Value::Flag(a.is_true() && b.is_true()),
            Action::Or => Value::Flag(a.is_true() || b.is_true()),
            Action::Eq => Value::Flag(a.equals(b)),
            Action::Ne => Value::Flag(!a.equals(b)),
            Action::Join => joined(),
            Action::At => self.element(a, b)?,
            Action::Add if matches!(a, Value::Text(_)) || matches!(b, Value::Text(_)) => joined(),
            Action::Add | Action::Sub | Action::Mul | Action::Div | Action::DivReal | Action::IntDiv | Action::Mod | Action::Power => {
                let calc = match op {
                    Action::Add => Operation::Plus,
                    Action::Sub => Operation::Minus,
                    Action::Mul => Operation::Times,
                    Action::Div => Operation::Over,
                    Action::DivReal => Operation::OverReal,
                    Action::IntDiv => Operation::Floor,
                    Action::Mod => Operation::Remainder,
                    _ => Operation::Raise,
                };
                match arith::calculate(calc, a, b) {
                    Some(r) => r?,
                    // The closed operations coerce booleans and null.
                    None => match calc {
                        Operation::Plus => Value::of_big(a.as_big()? + b.as_big()?),
                        Operation::Minus => Value::of_big(a.as_big()? - b.as_big()?),
                        Operation::Times => Value::of_big(a.as_big()? * b.as_big()?),
                        Operation::Over | Operation::OverReal => return Err("Division requires numeric both".to_string()),
                        Operation::Floor => return Err("Integer quotient requires numeric both".to_string()),
                        Operation::Remainder => return Err("Modulo requires numeric both".to_string()),
                        Operation::Raise => return Err("Exponentiation requires numeric both".to_string()),
                    },
                }
            }
            Action::Lt | Action::Le | Action::Gt | Action::Ge => {
                // From less-than alone: a > b is b < a, a <= b is not b < a.
                let below = |x: &Value, y: &Value| -> Res<bool> {
                    match arith::order_values(x, y) {
                        Some(order) => Ok(order == std::cmp::Ordering::Less),
                        None => Ok(x.as_big()? < y.as_big()?),
                    }
                };
                Value::Flag(match op {
                    Action::Lt => below(a, b)?,
                    Action::Gt => below(b, a)?,
                    Action::Le => !below(b, a)?,
                    _ => !below(a, b)?,
                })
            }
            other => unreachable!("{other:?} is not dyadic"),
        })
    }

    fn element(&self, target: &Value, at: &Value) -> Res<Value> {
        if let Value::Map(pairs) = target {
            let found = pairs.iter().find(|(k, _)| k.equals(at));
            return found.map(|(_, v)| v.clone()).ok_or_else(|| format!("Undefined array key {}", at.plain()));
        }
        let i = as_index(at)?;
        match target {
            Value::Array(items) => {
                items.get(i).cloned().ok_or_else(|| format!("Array index {} out of bounds (length: {})", i, items.len()))
            }
            Value::Text(s) if self.lang.text_indexable => s
                .chars()
                .nth(i)
                .map(|c| Value::text(&c.to_string()))
                .ok_or_else(|| format!("String index {} out of bounds (length: {})", i, s.chars().count())),
            _ => Err("Cannot index non-array value".to_string()),
        }
    }

    // ---------- builtins ----------

    /// print and write: the values joined by spaces, or a template holding
    /// the definition's placeholders filled from the rest.
    fn render(&self, values: &[Value]) -> String {
        let sp = self.wording();
        let holes = &self.lang.holes;
        let hole_in = |s: &str| holes.iter().filter_map(|h| s.find(h.as_str()).map(|at| (at, h.len()))).min();
        if let (Some(Value::Text(template)), true) = (values.first(), values.len() > 1) {
            if hole_in(template).is_some() {
                let mut out = String::new();
                let mut fill = values[1..].iter();
                let mut s: &str = template;
                while let Some((at, width)) = hole_in(s) {
                    out.push_str(&s[..at]);
                    match fill.next() {
                        Some(v) => out.push_str(&v.display(&sp)),
                        None => out.push_str(&s[at..at + width]),
                    }
                    s = &s[at + width..];
                }
                out.push_str(s);
                return out;
            }
        }
        values.iter().map(|v| v.display(&sp)).collect::<Vec<_>>().join(" ")
    }

    fn builtin(&mut self, builtin: Builtin, name: &str, args: &mut Vec<Value>) -> Res<Value> {
        let sp = self.wording();
        let arity = |n: usize| -> Res<()> {
            if args.len() == n {
                return Ok(());
            }
            Err(format!("{}() expects {} argument{}, got {}", name, n, if n == 1 { "" } else { "s" }, args.len()))
        };
        Ok(match builtin {
            Builtin::Echo => {
                arity(1)?;
                let Value::Text(s) = &args[0] else { return Err(format!("{}() requires a string argument", name)) };
                print!("{}", s);
                Value::Null
            }
            Builtin::Say => {
                println!("{}", self.render(&args));
                Value::Null
            }
            Builtin::Out => {
                print!("{}", self.render(&args));
                Value::Null
            }
            Builtin::Tell => {
                let sp = self.wording();
                for v in args.iter() {
                    print!("{}", v.display(&sp));
                }
                Value::Null
            }
            Builtin::Define => return Err(format!("{}() needs a quoted name as its first argument", name)),
            Builtin::Dump => {
                for v in args.iter() {
                    println!("{}", dumped(v, 0));
                }
                Value::Null
            }
            Builtin::Span => return Err(format!("{}() spells a range, which belongs in a for loop", name)),
            Builtin::MakeReal => {
                if args.is_empty() || args.len() > 2 {
                    return Err(format!("{}() expects 1 or 2 arguments, got {}", name, args.len()));
                }
                let places = match args.get(1) {
                    None => arith::DEFAULT_PLACES,
                    Some(Value::Small(n)) if *n >= 0 => *n as usize,
                    Some(Value::Small(_)) | Some(Value::Huge(_)) => return Err("Precision must be a positive integer".to_string()),
                    Some(_) => return Err("Precision argument must be an integer".to_string()),
                };
                arith::to_real(&args[0], places)
                    .ok_or_else(|| format!("{}() requires a number, rational, or real argument", name))?
            }
            Builtin::Places => {
                arity(1)?;
                match &args[0] {
                    Value::Real(r) => Value::Small(r.places as i64),
                    _ => return Err(format!("{}() requires a real argument", name)),
                }
            }
            Builtin::ToText => {
                arity(1)?;
                Value::text(&args[0].display(&sp))
            }
            Builtin::ToInt => {
                arity(1)?;
                let (p, q) = arith::parts(&args[0]).ok_or_else(|| format!("{}() requires a number argument", name))?;
                Value::of_big(p / q)
            }
            Builtin::AsReal => {
                arity(1)?;
                match &args[0] {
                    v @ Value::Real(_) => v.clone(),
                    v => arith::to_real(v, arith::DEFAULT_PLACES).ok_or_else(|| format!("{}() requires a number argument", name))?,
                }
            }
            Builtin::Length => {
                arity(1)?;
                match &args[0] {
                    Value::Text(s) => Value::Small(s.chars().count() as i64),
                    Value::Array(items) => Value::Small(items.len() as i64),
                    Value::Map(pairs) => Value::Small(pairs.len() as i64),
                    _ => return Err(format!("{}() requires a string or array argument", name)),
                }
            }
            Builtin::CharAtIndex => {
                arity(2)?;
                match (&args[0], &args[1]) {
                    (Value::Text(s), Value::Small(i)) => match usize::try_from(*i).ok().and_then(|i| s.chars().nth(i)) {
                        Some(c) => Value::text(&c.to_string()),
                        None => return Err(format!("{} index out of bounds", name)),
                    },
                    (Value::Text(_), _) => return Err(format!("{}() second argument must be an integer", name)),
                    _ => return Err(format!("{}() first argument must be a string", name)),
                }
            }
            Builtin::CodeOf => {
                arity(1)?;
                let Value::Text(s) = &args[0] else { return Err(format!("{}() requires a string argument", name)) };
                match s.chars().next() {
                    Some(c) => Value::Small(c as i64),
                    None => return Err(format!("{}() requires a non-empty string", name)),
                }
            }
            Builtin::CharOf => {
                arity(1)?;
                let code = match &args[0] {
                    Value::Small(n) => u32::try_from(*n).ok(),
                    Value::Huge(_) => None,
                    _ => return Err(format!("{}() requires an integer argument", name)),
                };
                let code = code
                    .ok_or_else(|| format!("{}() argument must be a non-negative integer within valid Unicode range", name))?;
                match char::from_u32(code) {
                    Some(c) => Value::text(&c.to_string()),
                    None => return Err(format!("{}() argument {} is not a valid Unicode code point", name, code)),
                }
            }
            Builtin::Raise => {
                arity(1)?;
                return match &args[0] {
                    Value::Text(s) => Err(s.to_string()),
                    _ => Err(format!("{}() argument must be a string", name)),
                };
            }
            Builtin::SortOf => {
                arity(1)?;
                match args[0].sort() {
                    Some(k) => Value::SortOf(k),
                    None => return Err(format!("{}(): unknown value type", name)),
                }
            }
            Builtin::Numer | Builtin::Denom => {
                arity(1)?;
                let (p, q) = arith::parts(&args[0]).ok_or_else(|| format!("{}() requires a number argument", name))?;
                Value::of_big(if builtin == Builtin::Numer { p } else { q })
            }
            Builtin::Fetch => {
                arity(2)?;
                self.element(&args[0], &args[1])?
            }
            Builtin::Append => {
                arity(2)?;
                let target = args.pop().expect("the array");
                let v = args.pop().expect("the value");
                match target {
                    Value::Array(mut items) => {
                        Rc::make_mut(&mut items).push(v);
                        Value::Array(items)
                    }
                    // The next whole-number key one past the highest.
                    Value::Map(mut pairs) => {
                        let key = next_key(&pairs);
                        Rc::make_mut(&mut pairs).push((Value::Small(key), v));
                        Value::Map(pairs)
                    }
                    _ => return Err(format!("{}() requires an array", name)),
                }
            }
            Builtin::Replace => {
                arity(3)?;
                let target = args.pop().expect("the array");
                let v = args.pop().expect("the value");
                let at = args.pop().expect("the key");
                match target {
                    // A list written at a place it already holds stays a list.
                    Value::Array(mut items) if as_index(&at).map_or(false, |i| i < items.len()) => {
                        let i = as_index(&at)?;
                        Rc::make_mut(&mut items)[i] = v;
                        Value::Array(items)
                    }
                    // Any other key makes it a map, its places the keys.
                    Value::Array(items) => {
                        let mut pairs: Vec<(Value, Value)> =
                            items.iter().enumerate().map(|(i, x)| (Value::Small(i as i64), x.clone())).collect();
                        put_key(&mut pairs, at, v);
                        Value::Map(Rc::new(pairs))
                    }
                    Value::Map(mut pairs) => {
                        put_key(Rc::make_mut(&mut pairs), at, v);
                        Value::Map(pairs)
                    }
                    _ => return Err(format!("{}() requires an array", name)),
                }
            }
            Builtin::Pack => return Err(format!("{}() is a literal, not a call", name)),
            Builtin::Layout => {
                arity(1)?;
                print!("{}", laid_out(&args[0], 0));
                Value::Flag(true)
            }
            Builtin::External => self.external(name, &args)?,
        })
    }

    /// The external capabilities of docs/LUMEN_LANGUAGE_EXTERN_SYSTEM.md.
    fn external(&self, name: &str, args: &[Value]) -> Res<Value> {
        let sp = self.wording();
        let which = match args.first() {
            Some(Value::Text(s)) => s.to_string(),
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
            "print_native" => println!("{}", v.display(&sp)),
            "debug_info" => eprintln!("[DEBUG] {}", v.display(&sp)),
            _ => {
                return Ok(Value::Small(match v {
                    Value::Small(_) | Value::Huge(_) | Value::Frac(_) | Value::Real(_) => 0,
                    Value::Flag(_) => 1,
                    Value::Text(_) => 2,
                    _ => return Err("Unknown value type".to_string()),
                }))
            }
        }
        Ok(v.clone())
    }
}

fn as_index(v: &Value) -> Res<usize> {
    match v {
        Value::Small(n) => usize::try_from(*n).map_err(|_| "Array index out of bounds".to_string()),
        Value::Huge(n) => n.to_usize().ok_or_else(|| "Array index out of bounds".to_string()),
        _ => Err("Array index must be a number".to_string()),
    }
}

pub fn sort_value(kind: Sort) -> Value {
    Value::SortOf(kind)
}

pub fn places_default() -> Value {
    Value::Small(arith::DEFAULT_PLACES as i64)
}

/// A value with its kind, as PHP's var_dump shows it: a number as
/// `int(n)` or `float(x)`, text with its byte length, an array one
/// entry per line, nested arrays indented two more.
fn dumped(v: &Value, depth: usize) -> String {
    let pad = "  ".repeat(depth);
    match v {
        Value::Small(_) | Value::Huge(_) => format!("int({})", v.plain()),
        Value::Real(_) | Value::Frac(_) => format!("float({})", v.plain()),
        Value::Text(s) => format!("string({}) \"{}\"", s.len(), s),
        Value::Flag(b) => format!("bool({})", b),
        Value::Array(items) => {
            let mut out = format!("array({}) {{\n", items.len());
            for (i, item) in items.iter().enumerate() {
                out.push_str(&format!("{pad}  [{i}]=>\n{pad}  {}\n", dumped(item, depth + 1)));
            }
            out.push_str(&pad);
            out.push('}');
            out
        }
        Value::Map(pairs) => {
            let mut out = format!("array({}) {{\n", pairs.len());
            for (k, item) in pairs.iter() {
                // A text key is shown in quotes, a number bare.
                let shown = match k {
                    Value::Text(s) => format!("\"{}\"", s),
                    other => other.plain(),
                };
                out.push_str(&format!("{pad}  [{shown}]=>\n{pad}  {}\n", dumped(item, depth + 1)));
            }
            out.push_str(&pad);
            out.push('}');
            out
        }
        _ => "NULL".to_string(),
    }
}

/// The values of a literal: a map when any of them is a tie or the
/// literal asks for one, otherwise a list. An untied value takes the
/// next whole-number key, as in a list.
fn gathered(items: Vec<Value>, always_map: bool) -> Value {
    if !always_map && !items.iter().any(|v| matches!(v, Value::Tie(_))) {
        return Value::array(items);
    }
    let mut pairs: Vec<(Value, Value)> = Vec::with_capacity(items.len());
    for item in items {
        match item {
            Value::Tie(pair) => {
                let (k, v) = (pair.0.clone(), pair.1.clone());
                put_key(&mut pairs, k, v);
            }
            v => {
                let key = next_key(&pairs);
                pairs.push((Value::Small(key), v));
            }
        }
    }
    Value::Map(Rc::new(pairs))
}

/// One past the highest whole-number key, or zero when there is none.
fn next_key(pairs: &[(Value, Value)]) -> i64 {
    pairs
        .iter()
        .filter_map(|(k, _)| match k {
            Value::Small(n) => Some(*n + 1),
            _ => None,
        })
        .max()
        .unwrap_or(0)
        .max(0)
}

/// Write a key: over the value it already holds, or at the end.
fn put_key(pairs: &mut Vec<(Value, Value)>, key: Value, value: Value) {
    match pairs.iter_mut().find(|(k, _)| k.equals(&key)) {
        Some(slot) => slot.1 = value,
        None => pairs.push((key, value)),
    }
}

/// A value over lines, as PHP's print_r writes it: a scalar bare, an
/// array as `Array` and its places in brackets, each nested array set
/// eight spaces further in and followed by a blank line.
fn laid_out(v: &Value, indent: usize) -> String {
    let pairs: Vec<(String, &Value)> = match v {
        Value::Array(items) => items.iter().enumerate().map(|(i, x)| (i.to_string(), x)).collect(),
        Value::Map(entries) => entries.iter().map(|(k, x)| (k.plain(), x)).collect(),
        other => return other.plain(),
    };
    let pad = " ".repeat(indent);
    let mut out = format!("Array\n{pad}(\n");
    for (key, item) in pairs {
        // What an array lays out ends its own line, so the newline
        // here is the gap PHP leaves after it; for a scalar it ends the line.
        let shown = laid_out(item, indent + 8);
        out.push_str(&format!("{pad}    [{key}] => {shown}\n"));
    }
    out.push_str(&format!("{pad})\n"));
    out
}
