// The four forms, run.
//
// A literal is itself, a program literal becoming a closure over the
// current frame. A load walks up the frames. An assign walks up and
// writes. A call of an operation applies the kernel's mechanics; a call
// of a program value makes a frame and runs the body. A program in tail
// position of another replaces it in the same native frame, so a loop
// written as a program that calls itself runs in constant stack. Return,
// break and continue travel up as signals until a program that catches
// them stops them.

use std::collections::HashMap;
use std::rc::Rc;

use num_traits::ToPrimitive;

use crate::arith::{self, Sum};
use crate::spec::Spec;
use crate::tree::{Catch, Node, Op, Program, Slot, Target};
use crate::value::{Frame, Sort, Value, Words};

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

type R<T = Value> = Result<T, Signal>;

enum Step {
    Done(Value),
    Tail(Rc<Program>, Rc<Frame>, Vec<Value>),
}

pub struct Runner<'a> {
    spec: &'a Spec,
    pub top: Rc<Frame>,
    names: Vec<String>,
    cache: HashMap<String, Value>,
    args_slot: Option<usize>,
    memo_slot: Option<usize>,
}

fn up(frame: &Rc<Frame>, depth: usize) -> Rc<Frame> {
    let mut f = frame.clone();
    for _ in 0..depth {
        f = f.parent.clone().expect("a frame above");
    }
    f
}

impl<'a> Runner<'a> {
    pub fn new(spec: &'a Spec, names: Vec<String>) -> Runner<'a> {
        let find = |key: &str| spec.one(key).and_then(|n| names.iter().position(|x| x == n));
        Runner {
            spec,
            top: Frame::new(names.len(), None),
            args_slot: find("system.args"),
            memo_slot: find("system.memoization"),
            names,
            cache: HashMap::new(),
        }
    }

    pub fn set(&mut self, name: &str, value: Value) {
        if let Some(i) = self.names.iter().position(|n| n == name) {
            self.top.slots.borrow_mut()[i] = value;
        }
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        let i = self.names.iter().position(|n| n == name)?;
        match &self.top.slots.borrow()[i] {
            Value::Empty => None,
            v => Some(v.clone()),
        }
    }

    fn words(&self) -> Words<'a> {
        Words {
            yes: self.spec.one("literal.true").unwrap_or("true"),
            no: self.spec.one("literal.false").unwrap_or("false"),
            none: self.spec.one("literal.null").unwrap_or("null"),
        }
    }

    pub fn run_top(&mut self, body: &Node) -> Result<(), String> {
        let top = self.top.clone();
        match self.eval(body, &top) {
            Ok(_) | Err(Signal::Return(_)) | Err(Signal::Break) | Err(Signal::Continue) => Ok(()),
            Err(Signal::Fail(e)) => Err(e),
        }
    }

    // ---------- bindings

    fn read(&self, slot: &Slot, frame: &Rc<Frame>) -> Result<Value, String> {
        let f = up(frame, slot.depth);
        let v = f.slots.borrow()[slot.index].clone();
        if !matches!(v, Value::Empty) {
            return Ok(v);
        }
        if let Some(g) = slot.global {
            let v = self.top.slots.borrow()[g].clone();
            if !matches!(v, Value::Empty) {
                return Ok(v);
            }
        }
        Err(format!("Undefined variable: {}", slot.name))
    }

    fn write(&self, slot: &Slot, frame: &Rc<Frame>, value: Value) -> Result<(), String> {
        let f = up(frame, slot.depth);
        if Rc::ptr_eq(&f, &self.top) && Some(slot.index) == self.args_slot {
            return Err(format!("Cannot reassign {} (system-provided immutable value)", slot.name));
        }
        f.slots.borrow_mut()[slot.index] = value;
        Ok(())
    }

    /// The frame and index an array lives in, for writing it in place.
    fn cell(&self, slot: &Slot, frame: &Rc<Frame>) -> Result<(Rc<Frame>, usize), String> {
        let f = up(frame, slot.depth);
        if !matches!(f.slots.borrow()[slot.index], Value::Empty) {
            return Ok((f, slot.index));
        }
        match slot.global {
            Some(g) if !matches!(self.top.slots.borrow()[g], Value::Empty) => Ok((self.top.clone(), g)),
            _ => Err(format!("Undefined variable '{}'", slot.name)),
        }
    }

    // ---------- evaluation

    fn eval(&mut self, node: &Node, frame: &Rc<Frame>) -> R {
        match node {
            Node::Literal(Value::Code(p)) => Ok(Value::Closure(p.clone(), frame.clone())),
            Node::Literal(v) => Ok(v.clone()),
            Node::Load(slot) => Ok(self.read(slot, frame)?),
            Node::Assign(slot, value) => {
                let v = self.eval(value, frame)?;
                self.write(slot, frame, v.clone())?;
                Ok(v)
            }
            Node::Call(Target::Program(target), args) => {
                let (p, env) = self.closure(target, frame)?;
                let values = self.values(args, frame)?;
                self.call(p, env, values)
            }
            Node::Call(Target::Op(op, name), args) => match op {
                Op::Last => {
                    let mut last = Value::Null;
                    for a in args {
                        last = self.eval(a, frame)?;
                    }
                    Ok(last)
                }
                Op::If => {
                    let (p, env) = self.choose(args, frame)?;
                    self.call(p, env, Vec::new())
                }
                Op::And | Op::Or => {
                    let left = self.eval(&args[0], frame)?.truth();
                    if (*op == Op::And && !left) || (*op == Op::Or && left) {
                        return Ok(Value::Bool(left));
                    }
                    // The right side is a program where the source could
                    // short-circuit, a value where it could not (postfix).
                    let right = match self.eval(&args[1], frame)? {
                        Value::Closure(p, env) => self.call(p, env, Vec::new())?,
                        v => v,
                    };
                    Ok(Value::Bool(right.truth()))
                }
                Op::Return => {
                    let v = match args.first() {
                        Some(a) => self.eval(a, frame)?,
                        None => Value::Null,
                    };
                    Err(Signal::Return(v))
                }
                Op::Break => Err(Signal::Break),
                Op::Continue => Err(Signal::Continue),
                Op::Push | Op::Put => {
                    let Some(Node::Load(slot)) = args.first() else {
                        return Err(format!("First argument to {}() must be an array variable name", name).into());
                    };
                    let values = self.values(&args[1..], frame)?;
                    let want = if *op == Op::Push { 1 } else { 2 };
                    if values.len() != want {
                        return Err(format!("{}() expects {} arguments, got {}", name, want + 1, values.len() + 1).into());
                    }
                    let (f, i) = self.cell(slot, frame)?;
                    let mut slots = f.slots.borrow_mut();
                    let Value::List(items) = &mut slots[i] else {
                        return Err(format!("Variable '{}' is not an array", slot.name).into());
                    };
                    let items = Rc::make_mut(items);
                    let mut values = values;
                    if want == 1 {
                        items.push(values.pop().unwrap());
                    } else {
                        let v = values.pop().unwrap();
                        let at = index(&values.pop().unwrap())?;
                        if at >= items.len() {
                            return Err(format!("Array index {} out of bounds (length: {})", at, items.len()).into());
                        }
                        items[at] = v;
                    }
                    Ok(Value::Null)
                }
                op => {
                    let values = self.values(args, frame)?;
                    Ok(self.operate(*op, name, &values)?)
                }
            },
        }
    }

    fn values(&mut self, args: &[Node], frame: &Rc<Frame>) -> R<Vec<Value>> {
        let mut out = Vec::with_capacity(args.len());
        for a in args {
            out.push(self.eval(a, frame)?);
        }
        Ok(out)
    }

    fn closure(&mut self, node: &Node, frame: &Rc<Frame>) -> R<(Rc<Program>, Rc<Frame>)> {
        match self.eval(node, frame)? {
            Value::Closure(p, env) => Ok((p, env)),
            Value::Empty => Err("Unknown function".to_string().into()),
            _ => match node {
                Node::Load(slot) => Err(format!("'{}' is not a function", slot.name).into()),
                _ => Err("eval needs a program".to_string().into()),
            },
        }
    }

    fn choose(&mut self, args: &[Node], frame: &Rc<Frame>) -> R<(Rc<Program>, Rc<Frame>)> {
        let test = self.eval(&args[0], frame)?.truth();
        self.closure(&args[if test { 1 } else { 2 }], frame)
    }

    /// One step in tail position: a value, or the program to run next.
    fn step(&mut self, node: &Node, frame: &Rc<Frame>) -> R<Step> {
        match node {
            Node::Call(Target::Program(target), args) => {
                let (p, env) = self.closure(target, frame)?;
                let values = self.values(args, frame)?;
                Ok(Step::Tail(p, env, values))
            }
            Node::Call(Target::Op(Op::Last, _), args) if !args.is_empty() => {
                for a in &args[..args.len() - 1] {
                    self.eval(a, frame)?;
                }
                self.step(&args[args.len() - 1], frame)
            }
            Node::Call(Target::Op(Op::If, _), args) => {
                let (p, env) = self.choose(args, frame)?;
                Ok(Step::Tail(p, env, Vec::new()))
            }
            other => Ok(Step::Done(self.eval(other, frame)?)),
        }
    }

    /// Run a program: a frame under the closure's, the parameters bound,
    /// the body stepped. A tail call replaces the program; what the
    /// replaced programs caught is still caught.
    pub fn call(&mut self, program: Rc<Program>, env: Rc<Frame>, args: Vec<Value>) -> R {
        let memo = program.catches == Catch::Return && self.memo_slot.map_or(false, |i| matches!(self.top.slots.borrow()[i], Value::Bool(true)));
        let key = memo.then(|| {
            let mut k = format!("{}(", program.name);
            args.iter().for_each(|a| a.cache_key(&mut k));
            k
        });
        if let Some(hit) = key.as_ref().and_then(|k| self.cache.get(k)) {
            return Ok(hit.clone());
        }
        let (mut program, mut env, mut args) = (program, env, args);
        let mut caught: u8 = 0;
        let result = loop {
            if args.len() != program.params.len() {
                return Err(format!("Function {} expects {} arguments, got {}", program.name, program.params.len(), args.len()).into());
            }
            let frame = Frame::new(program.names.len(), Some(env));
            {
                let mut slots = frame.slots.borrow_mut();
                for (i, a) in program.param_slots.iter().zip(args) {
                    slots[*i] = a;
                }
            }
            caught |= match program.catches {
                Catch::Nothing => 0,
                Catch::Return => 1,
                Catch::Break => 2,
                Catch::Continue => 4,
            };
            match self.step(&program.body, &frame) {
                Ok(Step::Done(v)) => {
                    // A language whose functions yield what they assigned to their own name.
                    if program.catches == Catch::Return && self.spec.on("stmt.function.result_by_name") {
                        if let Some(i) = program.names.iter().position(|n| *n == program.name) {
                            let own = frame.slots.borrow()[i].clone();
                            if !matches!(own, Value::Empty | Value::Closure(..)) {
                                break own;
                            }
                        }
                    }
                    break v;
                }
                Ok(Step::Tail(p, e, a)) => {
                    program = p;
                    env = e;
                    args = a;
                }
                Err(Signal::Return(v)) if caught & 1 != 0 => break v,
                Err(Signal::Break) if caught & 2 != 0 => break Value::Null,
                Err(Signal::Continue) if caught & 4 != 0 => break Value::Null,
                Err(e) => return Err(e),
            }
        };
        if let Some(k) = key {
            self.cache.insert(k, result.clone());
        }
        Ok(result)
    }

    // ---------- operations

    fn operate(&mut self, op: Op, name: &str, v: &[Value]) -> Result<Value, String> {
        let w = self.words();
        let n = |k: usize| -> Result<(), String> {
            if v.len() == k { Ok(()) } else { Err(format!("{}() expects {} argument{}, got {}", name, k, if k == 1 { "" } else { "s" }, v.len())) }
        };
        Ok(match op {
            Op::Array => Value::List(Rc::new(v.to_vec())),
            Op::Not => Value::Bool(!v[0].truth()),
            Op::Neg => match arith::apply(Sum::Sub, &Value::Int(0), &v[0]) {
                Some(r) => r?,
                None => return Err("Cannot negate non-numeric value".to_string()),
            },
            Op::Eq => Value::Bool(v[0].same(&v[1])),
            Op::Ne => Value::Bool(!v[0].same(&v[1])),
            Op::Concat => Value::str(&format!("{}{}", v[0].text(w), v[1].text(w))),
            Op::Index => self.index(&v[0], &v[1])?,
            Op::Add if matches!(v[0], Value::Str(_)) || matches!(v[1], Value::Str(_)) => Value::str(&format!("{}{}", v[0].text(w), v[1].text(w))),
            Op::Add | Op::Sub | Op::Mul | Op::Div | Op::DivReal | Op::Quot | Op::Rem | Op::Pow => {
                let sum = match op {
                    Op::Add => Sum::Add,
                    Op::Sub => Sum::Sub,
                    Op::Mul => Sum::Mul,
                    Op::Div => Sum::Div,
                    Op::DivReal => Sum::DivReal,
                    Op::Quot => Sum::Quot,
                    Op::Rem => Sum::Rem,
                    _ => Sum::Pow,
                };
                match arith::apply(sum, &v[0], &v[1]) {
                    Some(r) => r?,
                    None => match sum {
                        Sum::Add => Value::big(v[0].integer()? + v[1].integer()?),
                        Sum::Sub => Value::big(v[0].integer()? - v[1].integer()?),
                        Sum::Mul => Value::big(v[0].integer()? * v[1].integer()?),
                        Sum::Div | Sum::DivReal => return Err("Division requires numeric operands".to_string()),
                        Sum::Quot => return Err("Integer quotient requires numeric operands".to_string()),
                        Sum::Rem => return Err("Modulo requires numeric operands".to_string()),
                        Sum::Pow => return Err("Exponentiation requires numeric operands".to_string()),
                    },
                }
            }
            Op::Lt | Op::Le | Op::Gt | Op::Ge => {
                let below = |a: &Value, b: &Value| -> Result<bool, String> {
                    match arith::less(a, b) {
                        Some(r) => Ok(r),
                        None => Ok(a.integer()? < b.integer()?),
                    }
                };
                Value::Bool(match op {
                    Op::Lt => below(&v[0], &v[1])?,
                    Op::Gt => below(&v[1], &v[0])?,
                    Op::Le => !below(&v[1], &v[0])?,
                    _ => !below(&v[0], &v[1])?,
                })
            }
            Op::Emit => {
                n(1)?;
                match &v[0] {
                    Value::Str(s) => print!("{}", s),
                    _ => return Err(format!("{}() requires a string argument", name)),
                }
                Value::Null
            }
            Op::Print => {
                println!("{}", self.render(v));
                Value::Null
            }
            Op::Write => {
                print!("{}", self.render(v));
                Value::Null
            }
            Op::Range => return Err(format!("{}() spells a range, which belongs in a for loop", name)),
            Op::Real => {
                if v.is_empty() || v.len() > 2 {
                    return Err(format!("{}() expects 1 or 2 arguments, got {}", name, v.len()));
                }
                let digits = match v.get(1) {
                    None => arith::DIGITS,
                    Some(Value::Int(d)) if *d >= 0 => *d as usize,
                    Some(Value::Int(_)) | Some(Value::Big(_)) => return Err("Precision must be a positive integer".to_string()),
                    Some(_) => return Err("Precision argument must be an integer".to_string()),
                };
                arith::real_of(&v[0], digits).ok_or_else(|| format!("{}() requires a number, rational, or real argument", name))?
            }
            Op::Precision => {
                n(1)?;
                match &v[0] {
                    Value::Exact(e) if e.digits.is_some() => Value::Int(e.digits.unwrap() as i64),
                    _ => return Err(format!("{}() requires a real argument", name)),
                }
            }
            Op::ToString => {
                n(1)?;
                Value::str(&v[0].text(w))
            }
            Op::ToInt => {
                n(1)?;
                let e = arith::exact(&v[0]).ok_or_else(|| format!("{}() requires a number argument", name))?;
                Value::big(e.num / e.den)
            }
            Op::ToReal => {
                n(1)?;
                match &v[0] {
                    x @ Value::Exact(e) if e.digits.is_some() => x.clone(),
                    x => arith::real_of(x, arith::DIGITS).ok_or_else(|| format!("{}() requires a number argument", name))?,
                }
            }
            Op::Len => {
                n(1)?;
                match &v[0] {
                    Value::Str(s) => Value::Int(s.chars().count() as i64),
                    Value::List(l) => Value::Int(l.len() as i64),
                    _ => return Err(format!("{}() requires a string or array argument", name)),
                }
            }
            Op::CharAt => {
                n(2)?;
                match (&v[0], &v[1]) {
                    (Value::Str(s), Value::Int(i)) => match usize::try_from(*i).ok().and_then(|i| s.chars().nth(i)) {
                        Some(c) => Value::str(&c.to_string()),
                        None => return Err(format!("{} index out of bounds", name)),
                    },
                    (Value::Str(_), _) => return Err(format!("{}() second argument must be an integer", name)),
                    _ => return Err(format!("{}() first argument must be a string", name)),
                }
            }
            Op::Ord => {
                n(1)?;
                match &v[0] {
                    Value::Str(s) => match s.chars().next() {
                        Some(c) => Value::Int(c as i64),
                        None => return Err(format!("{}() requires a non-empty string", name)),
                    },
                    _ => return Err(format!("{}() requires a string argument", name)),
                }
            }
            Op::Chr => {
                n(1)?;
                let code = match &v[0] {
                    Value::Int(i) => u32::try_from(*i).ok(),
                    Value::Big(_) => None,
                    _ => return Err(format!("{}() requires an integer argument", name)),
                }
                .ok_or_else(|| format!("{}() argument must be a non-negative integer within valid Unicode range", name))?;
                match char::from_u32(code) {
                    Some(c) => Value::str(&c.to_string()),
                    None => return Err(format!("{}() argument {} is not a valid Unicode code point", name, code)),
                }
            }
            Op::Error => {
                n(1)?;
                return match &v[0] {
                    Value::Str(s) => Err(s.to_string()),
                    _ => Err(format!("{}() argument must be a string", name)),
                };
            }
            Op::Kind => {
                n(1)?;
                match v[0].sort() {
                    Some(s) => Value::Sort(s),
                    None => return Err(format!("{}(): unknown value type", name)),
                }
            }
            Op::Num => {
                n(1)?;
                Value::big(arith::exact(&v[0]).ok_or_else(|| format!("{}() requires a number argument", name))?.num)
            }
            Op::Den => {
                n(1)?;
                Value::big(arith::exact(&v[0]).ok_or_else(|| format!("{}() requires a number argument", name))?.den)
            }
            Op::Get => {
                n(2)?;
                self.index(&v[0], &v[1])?
            }
            Op::Extern => {
                let target = match v.first() {
                    Some(Value::Str(s)) => s.to_string(),
                    Some(_) => return Err(format!("First argument to {} must be a string (function name)", name)),
                    None => return Err(format!("{} requires at least one argument (function name)", name)),
                };
                if !matches!(target.as_str(), "print_native" | "debug_info" | "value_type") {
                    return Err(format!("Unknown external function: {}", target));
                }
                if v.len() != 2 {
                    return Err(format!("{} expects 1 argument, got {}", target, v.len() - 1));
                }
                let x = &v[1];
                match target.as_str() {
                    "print_native" => println!("{}", x.text(w)),
                    "debug_info" => eprintln!("[DEBUG] {}", x.text(w)),
                    _ => {
                        return match x.sort() {
                            Some(Sort::Integer | Sort::Rational | Sort::Real) => Ok(Value::Int(0)),
                            Some(Sort::Boolean) => Ok(Value::Int(1)),
                            Some(Sort::Text) => Ok(Value::Int(2)),
                            _ => Err("Unknown value type".to_string()),
                        }
                    }
                }
                x.clone()
            }
            Op::Last | Op::If | Op::And | Op::Or | Op::Return | Op::Break | Op::Continue | Op::Push | Op::Put => unreachable!("handled in eval"),
        })
    }

    fn index(&self, target: &Value, at: &Value) -> Result<Value, String> {
        let i = index(at)?;
        match target {
            Value::List(l) => l.get(i).cloned().ok_or_else(|| format!("Array index {} out of bounds (length: {})", i, l.len())),
            Value::Str(s) if self.spec.on("op.index.strings") => s
                .chars()
                .nth(i)
                .map(|c| Value::str(&c.to_string()))
                .ok_or_else(|| format!("String index {} out of bounds (length: {})", i, s.chars().count())),
            _ => Err("Cannot index non-array value".to_string()),
        }
    }

    fn render(&self, v: &[Value]) -> String {
        let w = self.words();
        let holes = self.spec.list("builtin.print.placeholder");
        let find = |s: &str| holes.iter().filter_map(|h| s.find(h.as_str()).map(|p| (p, h.len()))).min();
        if let (Some(Value::Str(t)), true) = (v.first(), v.len() > 1) {
            if find(t).is_some() {
                let mut out = String::new();
                let mut rest = v[1..].iter();
                let mut s: &str = t;
                while let Some((p, k)) = find(s) {
                    out.push_str(&s[..p]);
                    match rest.next() {
                        Some(x) => out.push_str(&x.text(w)),
                        None => out.push_str(&s[p..p + k]),
                    }
                    s = &s[p + k..];
                }
                out.push_str(s);
                return out;
            }
        }
        v.iter().map(|x| x.text(w)).collect::<Vec<_>>().join(" ")
    }
}

fn index(v: &Value) -> Result<usize, String> {
    match v {
        Value::Int(i) => usize::try_from(*i).map_err(|_| "Array index out of bounds".to_string()),
        Value::Big(n) => n.to_usize().ok_or_else(|| "Array index out of bounds".to_string()),
        _ => Err("Array index must be a number".to_string()),
    }
}
