// Stage 4: Execute — instruction tree → values.
//
// The seven primitives are interpreted here with fixed mechanics. Which
// surface names reach the built-ins, and which bindings are system values,
// is read from the schema; the operations themselves are the kernel's.

use std::cmp::Ordering;

use num_bigint::BigInt;
use num_traits::ToPrimitive;

use super::env::Environment;
use super::instruction::{Instruction, Target, TransferKind};
use super::numeric;
use super::value::{LiteralWords, Value};
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
    if let Some(builtin) = schema.functions.get(function).copied() {
        return builtin_call(builtin, function, args, env, schema);
    }

    let mut values = Vec::with_capacity(args.len());
    for arg in args {
        values.push(eval!(arg, env, schema));
    }

    let def = match env.lookup(function) {
        Some(Value::Function(def)) => def.clone(),
        Some(_) => return Err(format!("'{}' is not a function", function)),
        None => return Err(format!("Unknown function: {}", function)),
    };
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
        execute(&def.body, env, schema)
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
    // push(array, value) mutates the named array in place, so its first
    // argument is a binding name rather than a value.
    if builtin == Builtin::Push {
        if args.len() != 2 {
            return Err(format!("{}() expects 2 arguments, got {}", name, args.len()));
        }
        let target = match &args[0] {
            Instruction::Variable(n) => n.clone(),
            _ => return Err(format!("First argument to {}() must be an array variable name", name)),
        };
        let value = eval!(&args[1], env, schema);
        return match env.lookup_mut(&target) {
            Some(Value::Array(items)) => {
                items.push(value);
                Ok((Value::Null, Flow::Normal))
            }
            Some(_) => Err(format!("Variable '{}' is not an array", target)),
            None => Err(format!("Undefined variable '{}'", target)),
        };
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
/// first value is a string holding `{}` placeholders and more values
/// follow, that string with each placeholder filled in order.
fn render(values: &[Value], schema: &LanguageSchema) -> String {
    let words = words(schema);
    if values.len() > 1 {
        if let Value::String(template) = &values[0] {
            if template.contains("{}") {
                let mut out = String::new();
                let mut rest = values[1..].iter();
                let mut s = template.as_str();
                while let Some(pos) = s.find("{}") {
                    out.push_str(&s[..pos]);
                    match rest.next() {
                        Some(v) => out.push_str(&v.render(&words)),
                        None => out.push_str("{}"),
                    }
                    s = &s[pos + 2..];
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
        Range => {
            arity(name, values, 2)?;
            match (&values[0], &values[1]) {
                (Value::Number(start), Value::Number(end)) => Ok(Value::Range { start: start.clone(), end: end.clone() }),
                _ => Err(format!("{}() requires two integer arguments", name)),
            }
        }
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
        IntToString => {
            arity(name, values, 1)?;
            match &values[0] {
                Value::Number(n) => Ok(Value::String(n.to_string())),
                _ => Err(format!("{}() requires an integer argument", name)),
            }
        }
        RealToString => {
            arity(name, values, 1)?;
            match &values[0] {
                Value::Real { numerator, denominator, precision } => {
                    Ok(Value::String(Value::real_to_decimal(numerator, denominator, *precision)))
                }
                _ => Err(format!("{}() requires a real argument", name)),
            }
        }
        RationalToString => {
            arity(name, values, 1)?;
            match &values[0] {
                v @ Value::Rational { .. } => Ok(Value::String(v.to_string())),
                _ => Err(format!("{}() requires a rational argument", name)),
            }
        }
        BoolToString => {
            arity(name, values, 1)?;
            match &values[0] {
                v @ Value::Bool(_) => Ok(Value::String(v.render(&words))),
                _ => Err(format!("{}() requires a boolean argument", name)),
            }
        }
        ArrayToString => {
            arity(name, values, 1)?;
            match &values[0] {
                v @ Value::Array(_) => Ok(Value::String(v.render(&words))),
                _ => Err(format!("{}() requires an array argument", name)),
            }
        }
        NullToString => {
            arity(name, values, 1)?;
            match &values[0] {
                v @ Value::Null => Ok(Value::String(v.render(&words))),
                _ => Err(format!("{}() requires a null argument", name)),
            }
        }
        KindToString => {
            arity(name, values, 1)?;
            match &values[0] {
                Value::Kind(k) => Ok(Value::String(k.name().to_string())),
                _ => Err(format!("{}() requires a kind argument", name)),
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
        Num => {
            arity(name, values, 1)?;
            match &values[0] {
                Value::Rational { numerator, .. } => Ok(Value::Number(numerator.clone())),
                _ => Err(format!("{}() requires a rational argument", name)),
            }
        }
        Den => {
            arity(name, values, 1)?;
            match &values[0] {
                Value::Rational { denominator, .. } => Ok(Value::Number(denominator.clone())),
                _ => Err(format!("{}() requires a rational argument", name)),
            }
        }
        Int => {
            arity(name, values, 1)?;
            match &values[0] {
                Value::Real { numerator, denominator, .. } => Ok(Value::Number(numerator / denominator)),
                _ => Err(format!("{}() requires a real argument", name)),
            }
        }
        Frac => {
            arity(name, values, 1)?;
            match &values[0] {
                Value::Real { numerator, denominator, precision } => {
                    let int_part = numerator / denominator;
                    Ok(Value::Real {
                        numerator: numerator - &int_part * denominator,
                        denominator: denominator.clone(),
                        precision: *precision,
                    })
                }
                _ => Err(format!("{}() requires a real argument", name)),
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
                "print_native" => {
                    for v in rest {
                        println!("{}", v.render(&words));
                    }
                    Ok(Value::Null)
                }
                "value_type" => {
                    let v = rest.first().ok_or_else(|| "value_type requires an argument".to_string())?;
                    let kind = match v {
                        Value::Number(_) => "number",
                        Value::Rational { .. } => "rational",
                        Value::Real { .. } => "real",
                        Value::String(_) => "string",
                        Value::Bool(_) => "bool",
                        Value::Null => "null",
                        Value::Range { .. } => "range",
                        Value::Array(_) => "array",
                        Value::Function(_) => "function",
                        Value::Kind(_) => "kind",
                    };
                    Ok(Value::String(kind.to_string()))
                }
                "debug_info" => {
                    let v = rest.first().ok_or_else(|| "debug_info requires an argument".to_string())?;
                    println!("[DEBUG] {}", v.render(&words));
                    Ok(Value::Null)
                }
                other => Err(format!("Unknown external function: {}", other)),
            }
        }
        Push => unreachable!("push is handled before its arguments are evaluated"),
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
        Op::Not | Op::Negate | Op::RangeStart | Op::RangeEnd => {
            if operands.len() != 1 {
                return Err("Unary operator requires 1 operand".to_string());
            }
            let v = eval!(&operands[0], env, schema);
            let result = match op {
                Op::Not => Value::Bool(!v.to_bool()),
                Op::Negate => numeric::negate(&v)?,
                Op::RangeStart | Op::RangeEnd => match v {
                    Value::Range { start, end } => Value::Number(if op == Op::RangeStart { start } else { end }),
                    other => return Err(format!("For loop requires a range, got {}", other)),
                },
                _ => unreachable!(),
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

fn binary(op: Op, left: &Value, right: &Value, schema: &LanguageSchema) -> Result<Value, String> {
    let words = words(schema);
    match op {
        Op::And => Ok(Value::Bool(left.to_bool() && right.to_bool())),
        Op::Or => Ok(Value::Bool(left.to_bool() || right.to_bool())),
        Op::Eq => Ok(Value::Bool(left == right)),
        Op::Ne => Ok(Value::Bool(left != right)),
        Op::Concat => Ok(Value::String(format!("{}{}", left.render(&words), right.render(&words)))),
        Op::Range => Ok(Value::Range { start: left.to_number()?, end: right.to_number()? }),
        Op::Index => {
            let items = match left {
                Value::Array(items) => items,
                _ => return Err("Cannot index non-array value".to_string()),
            };
            let idx = array_index(right)?;
            items.get(idx).cloned().ok_or_else(|| format!("Array index {} out of bounds (length: {})", idx, items.len()))
        }
        Op::Add if matches!(left, Value::String(_)) || matches!(right, Value::String(_)) => {
            Ok(Value::String(format!("{}{}", left.render(&words), right.render(&words))))
        }
        Op::Add | Op::Sub | Op::Mul | Op::Div | Op::Quot | Op::Rem | Op::Pow => {
            match (numeric::to_num(left), numeric::to_num(right)) {
                (Some(a), Some(b)) => numeric::arith(op, &a, &b),
                _ if matches!(op, Op::Div | Op::Quot | Op::Rem | Op::Pow) => {
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
            let ordering = match (numeric::to_num(left), numeric::to_num(right)) {
                (Some(a), Some(b)) => numeric::compare(&a, &b),
                _ => left.to_number()?.cmp(&right.to_number()?),
            };
            Ok(Value::Bool(match op {
                Op::Lt => ordering == Ordering::Less,
                Op::Le => ordering != Ordering::Greater,
                Op::Gt => ordering == Ordering::Greater,
                _ => ordering != Ordering::Less,
            }))
        }
        Op::Pipe | Op::Not | Op::Negate | Op::ArrayLiteral | Op::RangeStart | Op::RangeEnd => {
            Err(format!("{:?} is not a binary operation", op))
        }
    }
}

fn op_name(op: Op) -> &'static str {
    match op {
        Op::Div => "Division",
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
