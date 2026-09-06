// Stage 4: Execute — instruction tree → values.
//
// The seven primitives are interpreted here with fixed mechanics. Which
// surface names reach the built-ins, and which bindings are system values,
// is read from the schema; the operations themselves are the kernel's.

use std::cmp::Ordering;
use std::rc::Rc;

use num_bigint::BigInt;
use num_traits::ToPrimitive;

use super::env::Environment;
use super::instruction::{Instruction, Target, TransferKind};
use super::numeric;
use super::value::{Function, LiteralWords, Value};
use crate::schema::{Builtin, LanguageSchema, Op};

/// Significant digits of a real when the program gives no precision.
pub const DEFAULT_REAL_PRECISION: usize = 15;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    Normal,
    Return,
    Break,
    Continue,
}

type Outcome = Result<(Value, Flow), String>;

/// Evaluate an instruction for its value, propagating any control transfer
/// raised while evaluating it.
macro_rules! eval {
    ($instr:expr, $env:expr, $schema:expr) => {{
        let (value, flow) = execute($instr, $env, $schema)?;
        if flow != Flow::Normal {
            return Ok((value, flow));
        }
        value
    }};
}

pub fn execute(instr: &Instruction, env: &mut Environment, schema: &LanguageSchema) -> Outcome {
    match instr {
        Instruction::Sequence(items) => {
            let mut last = Value::Null;
            for item in items {
                let (value, flow) = execute(item, env, schema)?;
                last = value;
                if flow != Flow::Normal {
                    return Ok((last, flow));
                }
            }
            Ok((last, Flow::Normal))
        }

        Instruction::Scope(inner) => env.in_frame(|env| execute(inner, env, schema)),

        Instruction::Branch { condition, then_branch, else_branch } => {
            let cond = eval!(condition, env, schema);
            if cond.to_bool() {
                execute(then_branch, env, schema)
            } else if let Some(other) = else_branch {
                execute(other, env, schema)
            } else {
                Ok((Value::Null, Flow::Normal))
            }
        }

        Instruction::Assign { target, value } => {
            let name = match target {
                Target::Name(name) | Target::Index { name, .. } => name,
            };
            if schema.system.args.as_deref() == Some(name.as_str()) {
                return Err(format!("Cannot reassign {} (system-provided immutable value)", name));
            }
            match target {
                Target::Name(name) => {
                    let v = eval!(value, env, schema);
                    env.bind(name.clone(), v.clone());
                    Ok((v, Flow::Normal))
                }
                Target::Index { name, index } => {
                    let idx = eval!(index, env, schema);
                    let v = eval!(value, env, schema);
                    let idx = array_index(&idx)?;
                    let slot = env.lookup_mut(name).ok_or_else(|| format!("Undefined variable '{}'", name))?;
                    match slot {
                        Value::Array(items) => {
                            if idx >= items.len() {
                                return Err(format!("Array index {} out of bounds (length: {})", idx, items.len()));
                            }
                            items[idx] = v.clone();
                            Ok((v, Flow::Normal))
                        }
                        _ => Err(format!("Variable '{}' is not an array", name)),
                    }
                }
            }
        }

        Instruction::Invoke { function, args } => invoke(function, args, env, schema),

        Instruction::Operate { op, operands } => operate(*op, operands, env, schema),

        Instruction::Transfer { kind, value } => {
            let v = match value {
                Some(v) => eval!(v, env, schema),
                None => Value::Null,
            };
            let flow = match kind {
                TransferKind::Return => Flow::Return,
                TransferKind::Break => Flow::Break,
                TransferKind::Continue => Flow::Continue,
            };
            Ok((v, flow))
        }

        Instruction::Loop { condition, body, step } => {
            loop {
                let cond = eval!(condition, env, schema);
                if !cond.to_bool() {
                    break;
                }
                let (value, flow) = execute(body, env, schema)?;
                match flow {
                    Flow::Break => return Ok((value, Flow::Normal)),
                    Flow::Return => return Ok((value, Flow::Return)),
                    Flow::Normal | Flow::Continue => {}
                }
                if let Some(step) = step {
                    let (value, flow) = execute(step, env, schema)?;
                    match flow {
                        Flow::Break => return Ok((value, Flow::Normal)),
                        Flow::Return => return Ok((value, Flow::Return)),
                        Flow::Normal | Flow::Continue => {}
                    }
                }
            }
            Ok((Value::Null, Flow::Normal))
        }

        Instruction::Literal(value) => Ok((value.clone(), Flow::Normal)),

        Instruction::Variable(name) => Ok((env.value(name)?, Flow::Normal)),
    }
}

// ---------------- Invoke ----------------

pub(crate) fn invoke(function: &str, args: &[Instruction], env: &mut Environment, schema: &LanguageSchema) -> Outcome {
    if let Some(builtin) = schema.functions.get(function).copied().or_else(|| internal_builtin(function)) {
        return builtin_call(builtin, function, args, env, schema);
    }

    let mut values = Vec::with_capacity(args.len());
    for arg in args {
        values.push(eval!(arg, env, schema));
    }

    let def = match env.lookup_function(function) {
        Some(Value::Function(def)) => def.clone(),
        _ => match env.lookup(function) {
            Some(_) => return Err(format!("'{}' is not a function", function)),
            None => return Err(format!("Unknown function: {}", function)),
        },
    };
    call_function(&def, function, values, env, schema)
}

/// The stack mechanics of a postfix language, invoked under names the
/// reducer emits and no definition can spell.
fn internal_builtin(name: &str) -> Option<Builtin> {
    Some(match name {
        "<push>" => Builtin::Push,
        "<pop>" => Builtin::Pop,
        "<word>" => Builtin::Word,
        "<eval>" => Builtin::Eval,
        "<depth>" => Builtin::Depth,
        "<gather>" => Builtin::Gather,
        _ => return None,
    })
}

/// Call a function value with evaluated arguments.
fn call_function(
    def: &Rc<Function>,
    function: &str,
    values: Vec<Value>,
    env: &mut Environment,
    schema: &LanguageSchema,
) -> Outcome {
    if def.params.len() != values.len() {
        return Err(format!("Function {} expects {} arguments, got {}", function, def.params.len(), values.len()));
    }

    let memoize = schema
        .system
        .memoization
        .as_deref()
        .and_then(|name| env.lookup(name))
        .map_or(false, |v| matches!(v, Value::Bool(true)));
    let key = if memoize { Some(memo_key(function, &values)) } else { None };
    if let Some(key) = &key {
        if let Some(cached) = env.cached_result(key) {
            return Ok((cached, Flow::Normal));
        }
    }

    let (result, flow) = env.in_frame(|env| {
        for (param, value) in def.params.iter().zip(values.iter()) {
            env.bind(param.clone(), value.clone());
        }
        let (value, flow) = execute(&def.body, env, schema)?;
        // Pascal: a body that ends without returning yields what it
        // assigned to the function's own name.
        if flow == Flow::Normal && schema.statements.function_result_by_name {
            if let Some(named) = env.lookup(function) {
                if !matches!(named, Value::Function(_)) {
                    return Ok((named.clone(), flow));
                }
            }
        }
        Ok((value, flow))
    })?;

    if let Some(key) = key {
        env.cache_result(key, result.clone());
    }
    match flow {
        Flow::Return | Flow::Normal => Ok((result, Flow::Normal)),
        other => Ok((result, other)),
    }
}

fn memo_key(function: &str, args: &[Value]) -> (String, String) {
    let fingerprint = args.iter().map(|v| format!("{:?}", v)).collect::<Vec<_>>().join("|");
    (function.to_string(), fingerprint)
}

fn builtin_call(
    builtin: Builtin,
    name: &str,
    args: &[Instruction],
    env: &mut Environment,
    schema: &LanguageSchema,
) -> Outcome {
    use Builtin::*;
    // push(array, value), put(array, index, value) and the stack mechanics
    // mutate a named array in place, so their first argument is a binding
    // name rather than a value.
    let named = |args: &[Instruction], n: usize| -> Result<String, String> {
        if args.len() != n {
            return Err(format!("{}() expects {} arguments, got {}", name, n, args.len()));
        }
        match &args[0] {
            Instruction::Variable(n) => Ok(n.clone()),
            _ => Err(format!("First argument to {}() must be an array variable name", name)),
        }
    };
    fn array_of<'e>(env: &'e mut Environment, target: &str) -> Result<&'e mut Vec<Value>, String> {
        match env.lookup_mut(target) {
            Some(Value::Array(items)) => Ok(items),
            Some(_) => Err(format!("Variable '{}' is not an array", target)),
            None => Err(format!("Undefined variable '{}'", target)),
        }
    }
    match builtin {
        Push => {
            let target = named(args, 2)?;
            let value = eval!(&args[1], env, schema);
            array_of(env, &target)?.push(value);
            return Ok((Value::Null, Flow::Normal));
        }
        Put => {
            let target = named(args, 3)?;
            let idx = eval!(&args[1], env, schema);
            let value = eval!(&args[2], env, schema);
            let idx = array_index(&idx)?;
            let items = array_of(env, &target)?;
            if idx >= items.len() {
                return Err(format!("Array index {} out of bounds (length: {})", idx, items.len()));
            }
            items[idx] = value;
            return Ok((Value::Null, Flow::Normal));
        }
        Pop => {
            let target = named(args, 1)?;
            let value = array_of(env, &target)?.pop().ok_or_else(|| "Stack underflow".to_string())?;
            return Ok((value, Flow::Normal));
        }
        Depth => {
            let target = named(args, 1)?;
            let depth = array_of(env, &target)?.len();
            return Ok((Value::Number(BigInt::from(depth)), Flow::Normal));
        }
        // Gather what was pushed since the mark into one array.
        Gather => {
            let target = named(args, 2)?;
            let mark = eval!(&args[1], env, schema);
            let mark = array_index(&mark)?;
            let items = array_of(env, &target)?;
            if mark > items.len() {
                return Err("Stack underflow".to_string());
            }
            let gathered = items.split_off(mark);
            items.push(Value::Array(gathered));
            return Ok((Value::Null, Flow::Normal));
        }
        // A bare word: run the program bound to it, or push its value.
        Word => {
            let target = named(args, 2)?;
            let word = match &args[1] {
                Instruction::Variable(w) => w.clone(),
                _ => return Err("A word must be a name".to_string()),
            };
            return match env.value(&word)? {
                Value::Function(def) => {
                    let (_, flow) = call_function(&def, &word, Vec::new(), env, schema)?;
                    Ok((Value::Null, flow))
                }
                value => {
                    array_of(env, &target)?.push(value);
                    Ok((Value::Null, Flow::Normal))
                }
            };
        }
        Eval => {
            named(args, 2)?;
            let program = eval!(&args[1], env, schema);
            return match program {
                Value::Function(def) => {
                    let (_, flow) = call_function(&def, name, Vec::new(), env, schema)?;
                    Ok((Value::Null, flow))
                }
                _ => Err(format!("{} needs a program", name)),
            };
        }
        _ => {}
    }

    let mut values = Vec::with_capacity(args.len());
    for arg in args {
        values.push(eval!(arg, env, schema));
    }
    let result = builtin_apply(builtin, name, &values, schema)?;
    Ok((result, Flow::Normal))
}

/// The literal spellings a language renders values with: the first entry
/// of each literal label, or the kernel's own word when there is none.
fn words(schema: &LanguageSchema) -> LiteralWords<'_> {
    fn first<'a>(list: &'a [String], fallback: &'static str) -> &'a str {
        list.first().map_or(fallback, String::as_str)
    }
    LiteralWords {
        true_word: first(&schema.literals.true_words, "true"),
        false_word: first(&schema.literals.false_words, "false"),
        null_word: first(&schema.literals.null_words, "null"),
    }
}

/// Text for print and write: the values joined by spaces, or, when the
/// first value is a string holding placeholders from the definition (Rust's
/// `{}`, C's `%d`) and more values follow, that string with each
/// placeholder filled in order.
fn render(values: &[Value], schema: &LanguageSchema) -> String {
    let words = words(schema);
    let holes = &schema.placeholders;
    // Position and width of the first placeholder in a template piece.
    let first_hole = |s: &str| holes.iter().filter_map(|h| s.find(h.as_str()).map(|at| (at, h.len()))).min();
    if values.len() > 1 {
        if let Value::String(template) = &values[0] {
            if first_hole(template).is_some() {
                let mut out = String::new();
                let mut rest = values[1..].iter();
                let mut s = template.as_str();
                while let Some((at, width)) = first_hole(s) {
                    out.push_str(&s[..at]);
                    match rest.next() {
                        Some(v) => out.push_str(&v.render(&words)),
                        None => out.push_str(&s[at..at + width]),
                    }
                    s = &s[at + width..];
                }
                out.push_str(s);
                return out;
            }
        }
    }
    values.iter().map(|v| v.render(&words)).collect::<Vec<_>>().join(" ")
}

fn arity(name: &str, values: &[Value], n: usize) -> Result<(), String> {
    if values.len() != n {
        Err(format!("{}() expects {} argument{}, got {}", name, n, if n == 1 { "" } else { "s" }, values.len()))
    } else {
        Ok(())
    }
}

fn builtin_apply(builtin: Builtin, name: &str, values: &[Value], schema: &LanguageSchema) -> Result<Value, String> {
    use Builtin::*;
    let words = words(schema);
    match builtin {
        Emit => {
            arity(name, values, 1)?;
            match &values[0] {
                Value::String(s) => {
                    print!("{}", s);
                    Ok(Value::Null)
                }
                _ => Err(format!("{}() requires a string argument", name)),
            }
        }
        PrintLine => {
            println!("{}", render(values, schema));
            Ok(Value::Null)
        }
        Write => {
            print!("{}", render(values, schema));
            Ok(Value::Null)
        }
        Range => Err(format!("{}() spells a range, which belongs in a for loop", name)),
        Real => {
            if values.is_empty() || values.len() > 2 {
                return Err(format!("{}() expects 1 or 2 arguments, got {}", name, values.len()));
            }
            let precision = match values.get(1) {
                None => DEFAULT_REAL_PRECISION,
                Some(Value::Number(n)) => n.to_u64().ok_or_else(|| "Precision must be a positive integer".to_string())? as usize,
                Some(_) => return Err("Precision argument must be an integer".to_string()),
            };
            match &values[0] {
                Value::Number(n) => Ok(Value::Real { numerator: n.clone(), denominator: BigInt::from(1), precision }),
                Value::Rational { numerator, denominator } | Value::Real { numerator, denominator, .. } => {
                    Ok(Value::Real { numerator: numerator.clone(), denominator: denominator.clone(), precision })
                }
                _ => Err(format!("{}() requires a number, rational, or real argument", name)),
            }
        }
        Precision => {
            arity(name, values, 1)?;
            match &values[0] {
                Value::Real { precision, .. } => Ok(Value::Number(BigInt::from(*precision))),
                _ => Err(format!("{}() requires a real argument", name)),
            }
        }
        ToString => {
            arity(name, values, 1)?;
            Ok(Value::String(values[0].render(&words)))
        }
        ToInt => {
            arity(name, values, 1)?;
            match &values[0] {
                Value::Number(n) => Ok(Value::Number(n.clone())),
                Value::Rational { numerator, denominator } | Value::Real { numerator, denominator, .. } => {
                    Ok(Value::Number(numerator / denominator))
                }
                _ => Err(format!("{}() requires a number argument", name)),
            }
        }
        ToReal => {
            arity(name, values, 1)?;
            match &values[0] {
                Value::Number(n) => {
                    Ok(Value::Real { numerator: n.clone(), denominator: BigInt::from(1), precision: DEFAULT_REAL_PRECISION })
                }
                Value::Rational { numerator, denominator } => Ok(Value::Real {
                    numerator: numerator.clone(),
                    denominator: denominator.clone(),
                    precision: DEFAULT_REAL_PRECISION,
                }),
                v @ Value::Real { .. } => Ok(v.clone()),
                _ => Err(format!("{}() requires a number argument", name)),
            }
        }
        Len => {
            arity(name, values, 1)?;
            match &values[0] {
                Value::String(s) => Ok(Value::Number(BigInt::from(s.chars().count()))),
                Value::Array(items) => Ok(Value::Number(BigInt::from(items.len()))),
                _ => Err(format!("{}() requires a string or array argument", name)),
            }
        }
        CharAt => {
            arity(name, values, 2)?;
            match (&values[0], &values[1]) {
                (Value::String(s), Value::Number(i)) => match i.to_usize().and_then(|i| s.chars().nth(i)) {
                    Some(ch) => Ok(Value::String(ch.to_string())),
                    None => Err(format!("{} index out of bounds", name)),
                },
                (Value::String(_), _) => Err(format!("{}() second argument must be an integer", name)),
                _ => Err(format!("{}() first argument must be a string", name)),
            }
        }
        Ord => {
            arity(name, values, 1)?;
            match &values[0] {
                Value::String(s) => match s.chars().next() {
                    Some(ch) => Ok(Value::Number(BigInt::from(ch as u32))),
                    None => Err(format!("{}() requires a non-empty string", name)),
                },
                _ => Err(format!("{}() requires a string argument", name)),
            }
        }
        Chr => {
            arity(name, values, 1)?;
            match &values[0] {
                Value::Number(n) => {
                    let code = n.to_u32().ok_or_else(|| {
                        format!("{}() argument must be a non-negative integer within valid Unicode range", name)
                    })?;
                    let ch = char::from_u32(code)
                        .ok_or_else(|| format!("{}() argument {} is not a valid Unicode code point", name, code))?;
                    Ok(Value::String(ch.to_string()))
                }
                _ => Err(format!("{}() requires an integer argument", name)),
            }
        }
        Error => {
            arity(name, values, 1)?;
            match &values[0] {
                Value::String(s) => Err(s.clone()),
                _ => Err(format!("{}() argument must be a string", name)),
            }
        }
        Kind => {
            arity(name, values, 1)?;
            values[0].kind().map(Value::Kind).ok_or_else(|| format!("{}(): unknown value type", name))
        }
        // Numerator and denominator of any number: an integer is over 1, a
        // real is the fraction it carries. int and frac are library code over these.
        Num => {
            arity(name, values, 1)?;
            match &values[0] {
                Value::Number(n) => Ok(Value::Number(n.clone())),
                Value::Rational { numerator, .. } | Value::Real { numerator, .. } => Ok(Value::Number(numerator.clone())),
                _ => Err(format!("{}() requires a number argument", name)),
            }
        }
        Den => {
            arity(name, values, 1)?;
            match &values[0] {
                Value::Number(_) => Ok(Value::Number(BigInt::from(1))),
                Value::Rational { denominator, .. } | Value::Real { denominator, .. } => Ok(Value::Number(denominator.clone())),
                _ => Err(format!("{}() requires a number argument", name)),
            }
        }
        Extern => {
            let target = match values.first() {
                Some(Value::String(s)) => s.as_str(),
                Some(_) => return Err(format!("First argument to {} must be a string (function name)", name)),
                None => return Err(format!("{} requires at least one argument (function name)", name)),
            };
            let rest = &values[1..];
            match target {
                // The built-in capabilities of docs/LUMEN_LANGUAGE_EXTERN_SYSTEM.md:
                // each takes one value; the printing ones hand it back, and
                // value_type answers with a number code.
                "print_native" | "debug_info" | "value_type" => {
                    if rest.len() != 1 {
                        return Err(format!("{} expects 1 argument, got {}", target, rest.len()));
                    }
                    let v = &rest[0];
                    match target {
                        "print_native" => println!("{}", v.render(&words)),
                        "debug_info" => eprintln!("[DEBUG] {}", v.render(&words)),
                        _ => {
                            let code = match v {
                                Value::Number(_) | Value::Rational { .. } | Value::Real { .. } => 0,
                                Value::Bool(_) => 1,
                                Value::String(_) => 2,
                                _ => return Err("Unknown value type".to_string()),
                            };
                            return Ok(Value::Number(BigInt::from(code)));
                        }
                    }
                    Ok(v.clone())
                }
                other => Err(format!("Unknown external function: {}", other)),
            }
        }
        Get => {
            arity(name, values, 2)?;
            binary(Op::Index, &values[0], &values[1], schema)
        }
        Push | Put | Pop | Word | Eval | Depth | Gather => {
            unreachable!("name-taking builtins are handled before their arguments are evaluated")
        }
    }
}

// ---------------- Operate ----------------

fn operate(op: Op, operands: &[Instruction], env: &mut Environment, schema: &LanguageSchema) -> Outcome {
    match op {
        Op::ArrayLiteral => {
            let mut items = Vec::with_capacity(operands.len());
            for operand in operands {
                items.push(eval!(operand, env, schema));
            }
            return Ok((Value::Array(items), Flow::Normal));
        }
        Op::Not | Op::Negate => {
            if operands.len() != 1 {
                return Err("Unary operator requires 1 operand".to_string());
            }
            let v = eval!(&operands[0], env, schema);
            let result = match op {
                Op::Not => Value::Bool(!v.to_bool()),
                // Derived: -x is 0 - x, so a real keeps its precision.
                _ => match numeric::to_num(&v) {
                    Some(n) => numeric::arith(Op::Sub, &numeric::to_num(&Value::Number(BigInt::from(0))).expect("a number"), &n)?,
                    None => return Err("Cannot negate non-numeric value".to_string()),
                },
            };
            return Ok((result, Flow::Normal));
        }
        _ => {}
    }

    if operands.len() != 2 {
        return Err("Binary operator requires 2 operands".to_string());
    }
    let left = eval!(&operands[0], env, schema);

    // Short-circuit logic.
    match op {
        Op::And if !left.to_bool() => return Ok((Value::Bool(false), Flow::Normal)),
        Op::Or if left.to_bool() => return Ok((Value::Bool(true), Flow::Normal)),
        _ => {}
    }

    let right = eval!(&operands[1], env, schema);
    let result = binary(op, &left, &right, schema)?;
    Ok((result, Flow::Normal))
}

/// Numbers of any kind are equal when their exact values are; everything
/// else compares by kind and content.
fn equal(left: &Value, right: &Value) -> bool {
    match (numeric::to_num(left), numeric::to_num(right)) {
        (Some(a), Some(b)) => numeric::compare(&a, &b) == Ordering::Equal,
        _ => left == right,
    }
}

fn binary(op: Op, left: &Value, right: &Value, schema: &LanguageSchema) -> Result<Value, String> {
    let words = words(schema);
    match op {
        Op::And => Ok(Value::Bool(left.to_bool() && right.to_bool())),
        Op::Or => Ok(Value::Bool(left.to_bool() || right.to_bool())),
        Op::Eq => Ok(Value::Bool(equal(left, right))),
        Op::Ne => Ok(Value::Bool(!equal(left, right))),
        Op::Concat => Ok(Value::String(format!("{}{}", left.render(&words), right.render(&words)))),
        Op::Range => Err("A range belongs in a for loop".to_string()),
        Op::Index => {
            let idx = array_index(right)?;
            match left {
                Value::Array(items) => items
                    .get(idx)
                    .cloned()
                    .ok_or_else(|| format!("Array index {} out of bounds (length: {})", idx, items.len())),
                Value::String(s) if schema.structure.index_strings => s
                    .chars()
                    .nth(idx)
                    .map(|c| Value::String(c.to_string()))
                    .ok_or_else(|| format!("String index {} out of bounds (length: {})", idx, s.chars().count())),
                _ => Err("Cannot index non-array value".to_string()),
            }
        }
        Op::Add if matches!(left, Value::String(_)) || matches!(right, Value::String(_)) => {
            Ok(Value::String(format!("{}{}", left.render(&words), right.render(&words))))
        }
        Op::Add | Op::Sub | Op::Mul | Op::Div | Op::DivReal | Op::Quot | Op::Rem | Op::Pow => {
            match (numeric::to_num(left), numeric::to_num(right)) {
                (Some(a), Some(b)) => numeric::arith(op, &a, &b),
                _ if matches!(op, Op::Div | Op::DivReal | Op::Quot | Op::Rem | Op::Pow) => {
                    Err(format!("{} requires numeric operands", op_name(op)))
                }
                // Legacy coercion for the closed operations on booleans and null.
                _ => {
                    let a = left.to_number()?;
                    let b = right.to_number()?;
                    Ok(Value::Number(match op {
                        Op::Add => a + b,
                        Op::Sub => a - b,
                        _ => a * b,
                    }))
                }
            }
        }
        Op::Lt | Op::Le | Op::Gt | Op::Ge => {
            // Derived from less-than alone: a > b is b < a, a <= b is not
            // b < a, a >= b is not a < b.
            let less = |x: &Value, y: &Value| -> Result<bool, String> {
                Ok(match (numeric::to_num(x), numeric::to_num(y)) {
                    (Some(a), Some(b)) => numeric::compare(&a, &b) == Ordering::Less,
                    _ => x.to_number()? < y.to_number()?,
                })
            };
            Ok(Value::Bool(match op {
                Op::Lt => less(left, right)?,
                Op::Gt => less(right, left)?,
                Op::Le => !less(right, left)?,
                _ => !less(left, right)?,
            }))
        }
        Op::Pipe | Op::Not | Op::Negate | Op::ArrayLiteral => {
            Err(format!("{:?} is not a binary operation", op))
        }
    }
}

fn op_name(op: Op) -> &'static str {
    match op {
        Op::Div | Op::DivReal => "Division",
        Op::Quot => "Integer quotient",
        Op::Rem => "Modulo",
        Op::Pow => "Exponentiation",
        _ => "Operation",
    }
}

fn array_index(value: &Value) -> Result<usize, String> {
    match value {
        Value::Number(n) => n.to_usize().ok_or_else(|| "Array index out of bounds".to_string()),
        _ => Err("Array index must be a number".to_string()),
    }
}
