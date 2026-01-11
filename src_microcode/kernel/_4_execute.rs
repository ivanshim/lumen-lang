// Stage 4: Execute - Faithful execution of instructions
//
// Apply the 7 primitives with clear, deterministic semantics.
// No language-specific behavior here - just mechanics.

use super::primitives::{Instruction, TransferKind, OperateKind};
use super::eval::Value;
use super::env::Environment;
use crate::schema::LanguageSchema;
use num_bigint::BigInt;
use num_traits::cast::ToPrimitive;
use num_integer::gcd;

/// Execution state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlFlow {
    Normal,
    Return,
    Break,
    Continue,
}

/// Execute instruction tree
pub fn execute(
    instr: &Instruction,
    env: &mut Environment,
    _schema: &LanguageSchema,
) -> Result<(Value, ControlFlow), String> {
    match instr {
        // 1. Sequence: execute in order, return last value
        Instruction::Sequence(instrs) => {
            let mut result = Value::Null;
            for inst in instrs {
                let (val, flow) = execute(inst, env, _schema)?;
                result = val;
                if flow != ControlFlow::Normal {
                    return Ok((result, flow));
                }
            }
            Ok((result, ControlFlow::Normal))
        }

        // 2. Scope: push scope, execute, pop scope
        Instruction::Scope(inst) => {
            env.push_scope();
            let result = execute(inst, env, _schema);
            env.pop_scope();
            result
        }

        // 3. Branch: if condition then else
        Instruction::Branch {
            condition,
            then_instr,
            else_instr,
        } => {
            let (cond_val, flow) = execute(condition, env, _schema)?;
            if flow != ControlFlow::Normal {
                return Ok((cond_val, flow));
            }

            if cond_val.to_bool() {
                execute(then_instr, env, _schema)
            } else if let Some(else_inst) = else_instr {
                execute(else_inst, env, _schema)
            } else {
                Ok((Value::Null, ControlFlow::Normal))
            }
        }

        // 4. Assign: bind name in current scope
        Instruction::Assign { name, value } => {
            // ARGS is a system-provided immutable semantic value
            if name == "ARGS" {
                return Err("Cannot reassign ARGS (system-provided immutable value)".to_string());
            }
            let (val, flow) = execute(value, env, _schema)?;
            if flow != ControlFlow::Normal {
                return Ok((val.clone(), flow));
            }
            env.set(name.clone(), val.clone());
            Ok((val, ControlFlow::Normal))
        }

        // 5. Invoke: call external function
        Instruction::Invoke { function, args } => {
            let mut arg_vals = Vec::new();
            for arg in args {
                let (val, flow) = execute(arg, env, _schema)?;
                if flow != ControlFlow::Normal {
                    return Ok((val, flow));
                }
                arg_vals.push(val);
            }

            // External function dispatch
            match function.as_str() {
                "emit" => {
                    // emit(string) - kernel primitive for output
                    // Accepts a string only, no implicit conversion
                    if arg_vals.len() != 1 {
                        return Err(format!("emit() expects 1 argument, got {}", arg_vals.len()));
                    }
                    match &arg_vals[0] {
                        Value::String(s) => {
                            print!("{}", s);
                            Ok((Value::Null, ControlFlow::Normal))
                        }
                        _ => Err("emit() requires a string argument".to_string()),
                    }
                }
                "int" => {
                    // int(x): convert string to integer
                    if arg_vals.len() != 1 {
                        return Err(format!("int() expects 1 argument, got {}", arg_vals.len()));
                    }
                    match &arg_vals[0] {
                        Value::String(s) => {
                            // Parse string as BigInt
                            match s.trim().parse::<num_bigint::BigInt>() {
                                Ok(bigint) => Ok((Value::Number(bigint), ControlFlow::Normal)),
                                Err(_) => Err(format!("int(): cannot parse '{}' as integer", s)),
                            }
                        }
                        _ => Err("int() requires a string argument".to_string()),
                    }
                }
                "str" => {
                    // str(x): convert any value to string representation
                    if arg_vals.len() != 1 {
                        return Err(format!("str() expects 1 argument, got {}", arg_vals.len()));
                    }
                    let str_val = format!("{}", &arg_vals[0]);
                    Ok((Value::String(str_val), ControlFlow::Normal))
                }
                "extern" => {
                    // extern(function_name, arg1, arg2, ...)
                    if arg_vals.is_empty() {
                        return Err("extern requires at least one argument (function name)".to_string());
                    }
                    let func_name = match &arg_vals[0] {
                        Value::String(s) => s.clone(),
                        _ => return Err("First argument to extern must be a string (function name)".to_string()),
                    };
                    let extern_args = arg_vals[1..].to_vec();

                    // Dispatch to the requested function
                    match func_name.as_str() {
                        "print_native" => {
                            for val in &extern_args {
                                println!("{}", val);
                            }
                            Ok((Value::Null, ControlFlow::Normal))
                        }
                        "value_type" => {
                            // Return the type of the first argument
                            if extern_args.is_empty() {
                                return Err("value_type requires an argument".to_string());
                            }
                            let type_str = match &extern_args[0] {
                                Value::Number(_) => "number",
                                Value::Rational { .. } => "rational",
                                Value::String(_) => "string",
                                Value::Bool(_) => "bool",
                                Value::Null => "null",
                                Value::Range { .. } => "range",
                                Value::Function { .. } => "function",
                            };
                            Ok((Value::String(type_str.to_string()), ControlFlow::Normal))
                        }
                        "debug_info" => {
                            // Print debug info about the value
                            if extern_args.is_empty() {
                                return Err("debug_info requires an argument".to_string());
                            }
                            println!("[DEBUG] {}", extern_args[0]);
                            Ok((Value::Null, ControlFlow::Normal))
                        }
                        _ => Err(format!("Unknown external function: {}", func_name)),
                    }
                }
                _ => {
                    // Check if it's a user-defined function
                    if let Ok(_func_val) = env.get(function) {
                        // Look up the function metadata
                        if let Some(metadata) = env.functions.get(function).cloned() {
                            let params = metadata.params.clone();
                            let body_instr = metadata.body.clone();
                            let memoizable = metadata.memoizable;

                            // Check parameter count
                            if params.len() != arg_vals.len() {
                                return Err(format!(
                                    "Function {} expects {} arguments, got {}",
                                    function,
                                    params.len(),
                                    arg_vals.len()
                                ));
                            }

                            // Semantic memoization: Check cache ONLY if function is marked memoizable
                            if memoizable {
                                if let Some(cached_result) = env.get_cached(function, &arg_vals) {
                                    // Cache hit: return cached result without executing
                                    return Ok((cached_result, ControlFlow::Normal));
                                }
                            }

                            // Execute function (either not memoizable or cache miss)
                            env.push_scope();

                            // Bind parameters
                            for (param, arg) in params.iter().zip(arg_vals.iter()) {
                                env.set(param.clone(), arg.clone());
                            }

                            // Execute function body
                            let (result, flow) = execute(&body_instr, env, _schema)?;

                            // Pop scope
                            env.pop_scope();

                            // Cache result ONLY if function is memoizable
                            if memoizable {
                                env.cache_result(function, &arg_vals, result.clone());
                            }

                            // Handle return value
                            match flow {
                                ControlFlow::Return => Ok((result, ControlFlow::Normal)),
                                ControlFlow::Normal => Ok((result, ControlFlow::Normal)),
                                _ => Ok((result, flow)),
                            }
                        } else {
                            Err(format!("Function body not found for: {}", function))
                        }
                    } else {
                        Err(format!("Unknown function: {}", function))
                    }
                }
            }
        }

        // 6. Operate: apply operator
        Instruction::Operate { kind, operands } => {
            execute_operator(kind, operands, env, _schema)
        }

        // 7. Transfer: control flow (return/break/continue)
        Instruction::Transfer { kind, value } => {
            let val = if let Some(v) = value {
                let (v_val, flow) = execute(v, env, _schema)?;
                if flow != ControlFlow::Normal {
                    return Ok((v_val, flow));
                }
                v_val
            } else {
                Value::Null
            };

            let flow = match kind {
                TransferKind::Return => ControlFlow::Return,
                TransferKind::Break => ControlFlow::Break,
                TransferKind::Continue => ControlFlow::Continue,
            };

            Ok((val, flow))
        }

        // Loop: while condition { body }
        Instruction::Loop { condition, body } => {
            loop {
                let (cond_val, flow) = execute(condition, env, _schema)?;
                if flow != ControlFlow::Normal {
                    return Ok((cond_val, flow));
                }

                if !cond_val.to_bool() {
                    break;
                }

                let (result, flow) = execute(body, env, _schema)?;
                match flow {
                    ControlFlow::Normal => continue,
                    ControlFlow::Break => return Ok((result, ControlFlow::Normal)),
                    ControlFlow::Continue => continue,
                    ControlFlow::Return => return Ok((result, ControlFlow::Return)),
                }
            }

            Ok((Value::Null, ControlFlow::Normal))
        }

        // ForLoop: for var in iterable { body }
        Instruction::ForLoop { var, iterable, body } => {
            let (range_val, flow) = execute(iterable, env, _schema)?;
            if flow != ControlFlow::Normal {
                return Ok((range_val, flow));
            }

            // Expect a range value
            match range_val {
                Value::Range { start, end } => {
                    let mut current = start;
                    while current < end {
                        env.set(var.clone(), Value::Number(current.clone()));
                        let (result, flow) = execute(body, env, _schema)?;
                        match flow {
                            ControlFlow::Normal => {},
                            ControlFlow::Break => return Ok((result, ControlFlow::Normal)),
                            ControlFlow::Continue => {},
                            ControlFlow::Return => return Ok((result, ControlFlow::Return)),
                        }
                        current += BigInt::from(1);
                    }
                    Ok((Value::Null, ControlFlow::Normal))
                }
                _ => Err(format!("For loop requires a range, got {}", range_val)),
            }
        }

        // UntilLoop: until condition { body } (do-until: execute body first, then check condition)
        Instruction::UntilLoop { condition, body } => {
            loop {
                let (result, flow) = execute(body, env, _schema)?;
                match flow {
                    ControlFlow::Normal => {},
                    ControlFlow::Break => return Ok((result, ControlFlow::Normal)),
                    ControlFlow::Continue => {},
                    ControlFlow::Return => return Ok((result, ControlFlow::Return)),
                }

                let (cond_val, flow) = execute(condition, env, _schema)?;
                if flow != ControlFlow::Normal {
                    return Ok((cond_val, flow));
                }

                if cond_val.to_bool() {
                    break;
                }
            }

            Ok((Value::Null, ControlFlow::Normal))
        }

        // Function definition: store in environment
        // Metadata includes memoizable flag from language semantics
        Instruction::FunctionDef {
            name,
            params,
            body,
            memoizable,
        } => {
            // Store function metadata: params, body, and memoizable flag
            // The kernel respects the memoizable flag set by the language layer
            env.set(
                name.clone(),
                Value::Function {
                    params: params.clone(),
                    body_ref: name.clone(),
                },
            );

            use super::env::FunctionMetadata;
            let metadata = FunctionMetadata {
                params: params.clone(),
                body: body.as_ref().clone(),
                memoizable: *memoizable,
            };
            env.functions.insert(name.clone(), metadata);

            Ok((Value::Null, ControlFlow::Normal))
        }

        // Literal: just return the value
        Instruction::Literal(val) => Ok((val.clone(), ControlFlow::Normal)),

        // Variable: look up in environment
        Instruction::Variable(name) => {
            let val = env.get(name)?;
            Ok((val, ControlFlow::Normal))
        }
    }
}

/// Execute operator
fn execute_operator(
    kind: &OperateKind,
    operands: &[Instruction],
    env: &mut Environment,
    schema: &LanguageSchema,
) -> Result<(Value, ControlFlow), String> {
    match kind {
        OperateKind::Unary(op) => {
            if operands.len() != 1 {
                return Err("Unary operator requires 1 operand".to_string());
            }
            let (val, flow) = execute(&operands[0], env, schema)?;
            if flow != ControlFlow::Normal {
                return Ok((val, flow));
            }

            let result = match op.as_str() {
                "-" => {
                    match val {
                        Value::Number(n) => Value::Number(-n),
                        Value::Rational { numerator, denominator } => {
                            Value::Rational { numerator: -numerator, denominator }
                        }
                        _ => return Err("Cannot negate non-numeric value".to_string()),
                    }
                }
                "not" | "!" => Value::Bool(!val.to_bool()),
                _ => return Err(format!("Unknown unary operator: {}", op)),
            };

            Ok((result, ControlFlow::Normal))
        }

        OperateKind::Binary(op) => {
            if operands.len() != 2 {
                return Err("Binary operator requires 2 operands".to_string());
            }

            // Special handling for pipe operator
            if op == "|>" {
                let (left_val, left_flow) = execute(&operands[0], env, schema)?;
                if left_flow != ControlFlow::Normal {
                    return Ok((left_val, left_flow));
                }

                // Right operand should be a function call with the left value prepended as first arg
                match &operands[1] {
                    Instruction::Invoke { function, args } => {
                        let mut new_args = vec![Instruction::Literal(left_val.clone())];
                        new_args.extend(args.clone());
                        let piped_invoke = Instruction::Invoke {
                            function: function.clone(),
                            args: new_args,
                        };
                        return execute(&piped_invoke, env, schema);
                    }
                    _ => {
                        return Err("Pipe operator requires a function call on the right side".to_string());
                    }
                }
            }

            let (left, left_flow) = execute(&operands[0], env, schema)?;
            if left_flow != ControlFlow::Normal {
                return Ok((left, left_flow));
            }

            // Short-circuit evaluation for logical operators
            match op.as_str() {
                "and" | "&&" => {
                    if !left.to_bool() {
                        return Ok((Value::Bool(false), ControlFlow::Normal));
                    }
                }
                "or" | "||" => {
                    if left.to_bool() {
                        return Ok((Value::Bool(true), ControlFlow::Normal));
                    }
                }
                _ => {}
            }

            let (right, right_flow) = execute(&operands[1], env, schema)?;
            if right_flow != ControlFlow::Normal {
                return Ok((right, right_flow));
            }

            let result = match op.as_str() {
                "+" => {
                    if let (Value::String(_), _) | (_, Value::String(_)) = (&left, &right) {
                        Value::String(format!("{}{}", left, right))
                    } else {
                        // Check if either operand is rational
                        match (&left, &right) {
                            (Value::Rational { numerator: l_num, denominator: l_denom },
                             Value::Rational { numerator: r_num, denominator: r_denom }) => {
                                // a/b + c/d = (ad + bc) / bd
                                let num = l_num * r_denom + r_num * l_denom;
                                let denom = l_denom * r_denom;
                                reduce_rational(num, denom)
                            }
                            (Value::Rational { numerator: l_num, denominator: l_denom },
                             Value::Number(r_num)) => {
                                // a/b + c = (a + bc) / b
                                let num = l_num + r_num * l_denom;
                                reduce_rational(num, l_denom.clone())
                            }
                            (Value::Number(l_num),
                             Value::Rational { numerator: r_num, denominator: r_denom }) => {
                                // a + c/d = (ad + c) / d
                                let num = l_num * r_denom + r_num;
                                reduce_rational(num, r_denom.clone())
                            }
                            _ => Value::Number(left.to_number()? + right.to_number()?)
                        }
                    }
                }
                "-" => {
                    match (&left, &right) {
                        (Value::Rational { numerator: l_num, denominator: l_denom },
                         Value::Rational { numerator: r_num, denominator: r_denom }) => {
                            // a/b - c/d = (ad - bc) / bd
                            let num = l_num * r_denom - r_num * l_denom;
                            let denom = l_denom * r_denom;
                            reduce_rational(num, denom)
                        }
                        (Value::Rational { numerator: l_num, denominator: l_denom },
                         Value::Number(r_num)) => {
                            // a/b - c = (a - bc) / b
                            let num = l_num - r_num * l_denom;
                            reduce_rational(num, l_denom.clone())
                        }
                        (Value::Number(l_num),
                         Value::Rational { numerator: r_num, denominator: r_denom }) => {
                            // a - c/d = (ad - c) / d
                            let num = l_num * r_denom - r_num;
                            reduce_rational(num, r_denom.clone())
                        }
                        _ => Value::Number(left.to_number()? - right.to_number()?)
                    }
                }
                "*" => {
                    match (&left, &right) {
                        (Value::Rational { numerator: l_num, denominator: l_denom },
                         Value::Rational { numerator: r_num, denominator: r_denom }) => {
                            // a/b * c/d = (ac) / (bd)
                            let num = l_num * r_num;
                            let denom = l_denom * r_denom;
                            reduce_rational(num, denom)
                        }
                        (Value::Rational { numerator: l_num, denominator: l_denom },
                         Value::Number(r_num)) => {
                            // a/b * c = (ac) / b
                            let num = l_num * r_num;
                            reduce_rational(num, l_denom.clone())
                        }
                        (Value::Number(l_num),
                         Value::Rational { numerator: r_num, denominator: r_denom }) => {
                            // a * c/d = (ac) / d
                            let num = l_num * r_num;
                            reduce_rational(num, r_denom.clone())
                        }
                        _ => Value::Number(left.to_number()? * right.to_number()?)
                    }
                }
                "/" => {
                    match (&left, &right) {
                        (Value::Rational { numerator: l_num, denominator: l_denom },
                         Value::Rational { numerator: r_num, denominator: r_denom }) => {
                            // a/b ÷ c/d = (ad) / (bc)
                            if r_num == &BigInt::from(0) {
                                return Err("Division by zero".to_string());
                            }
                            let num = l_num * r_denom;
                            let denom = l_denom * r_num;
                            reduce_rational(num, denom)
                        }
                        (Value::Rational { numerator: l_num, denominator: l_denom },
                         Value::Number(r_num)) => {
                            // a/b ÷ c = a / (bc)
                            if r_num == &BigInt::from(0) {
                                return Err("Division by zero".to_string());
                            }
                            let denom = l_denom * r_num;
                            reduce_rational(l_num.clone(), denom)
                        }
                        (Value::Number(l_num),
                         Value::Rational { numerator: r_num, denominator: r_denom }) => {
                            // a ÷ c/d = (ad) / c
                            if r_num == &BigInt::from(0) {
                                return Err("Division by zero".to_string());
                            }
                            let num = l_num * r_denom;
                            reduce_rational(num, r_num.clone())
                        }
                        (Value::Number(l_num), Value::Number(r_num)) => {
                            // a ÷ b = a/b (produces rational)
                            if r_num == &BigInt::from(0) {
                                return Err("Division by zero".to_string());
                            }
                            reduce_rational(l_num.clone(), r_num.clone())
                        }
                        _ => return Err("Division requires numeric operands".to_string())
                    }
                }
                "%" => {
                    let l = left.to_number()?;
                    let r = right.to_number()?;
                    if r == BigInt::from(0) {
                        return Err("Modulo by zero".to_string());
                    }
                    Value::Number(l % r)
                }
                "==" => Value::Bool(left == right),
                "!=" => Value::Bool(left != right),
                "<" => {
                    match (&left, &right) {
                        (Value::Rational { numerator: l_num, denominator: l_denom },
                         Value::Rational { numerator: r_num, denominator: r_denom }) => {
                            // a/b < c/d ⟺ ad < bc
                            let left_cross = l_num * r_denom;
                            let right_cross = r_num * l_denom;
                            Value::Bool(left_cross < right_cross)
                        }
                        (Value::Rational { numerator: l_num, denominator: l_denom },
                         Value::Number(r_num)) => {
                            // a/b < c ⟺ a < bc
                            let left_cross = l_num;
                            let right_cross = r_num * l_denom;
                            Value::Bool(left_cross < &right_cross)
                        }
                        (Value::Number(l_num),
                         Value::Rational { numerator: r_num, denominator: r_denom }) => {
                            // a < c/d ⟺ ad < c
                            let left_cross = l_num * r_denom;
                            let right_cross = r_num;
                            Value::Bool(&left_cross < right_cross)
                        }
                        _ => Value::Bool(left.to_number()? < right.to_number()?)
                    }
                }
                ">" => {
                    match (&left, &right) {
                        (Value::Rational { numerator: l_num, denominator: l_denom },
                         Value::Rational { numerator: r_num, denominator: r_denom }) => {
                            // a/b > c/d ⟺ ad > bc
                            let left_cross = l_num * r_denom;
                            let right_cross = r_num * l_denom;
                            Value::Bool(left_cross > right_cross)
                        }
                        (Value::Rational { numerator: l_num, denominator: l_denom },
                         Value::Number(r_num)) => {
                            // a/b > c ⟺ a > bc
                            let left_cross = l_num;
                            let right_cross = r_num * l_denom;
                            Value::Bool(left_cross > &right_cross)
                        }
                        (Value::Number(l_num),
                         Value::Rational { numerator: r_num, denominator: r_denom }) => {
                            // a > c/d ⟺ ad > c
                            let left_cross = l_num * r_denom;
                            let right_cross = r_num;
                            Value::Bool(&left_cross > right_cross)
                        }
                        _ => Value::Bool(left.to_number()? > right.to_number()?)
                    }
                }
                "<=" => {
                    match (&left, &right) {
                        (Value::Rational { numerator: l_num, denominator: l_denom },
                         Value::Rational { numerator: r_num, denominator: r_denom }) => {
                            // a/b <= c/d ⟺ ad <= bc
                            let left_cross = l_num * r_denom;
                            let right_cross = r_num * l_denom;
                            Value::Bool(left_cross <= right_cross)
                        }
                        (Value::Rational { numerator: l_num, denominator: l_denom },
                         Value::Number(r_num)) => {
                            // a/b <= c ⟺ a <= bc
                            let left_cross = l_num;
                            let right_cross = r_num * l_denom;
                            Value::Bool(left_cross <= &right_cross)
                        }
                        (Value::Number(l_num),
                         Value::Rational { numerator: r_num, denominator: r_denom }) => {
                            // a <= c/d ⟺ ad <= c
                            let left_cross = l_num * r_denom;
                            let right_cross = r_num;
                            Value::Bool(&left_cross <= right_cross)
                        }
                        _ => Value::Bool(left.to_number()? <= right.to_number()?)
                    }
                }
                ">=" => {
                    match (&left, &right) {
                        (Value::Rational { numerator: l_num, denominator: l_denom },
                         Value::Rational { numerator: r_num, denominator: r_denom }) => {
                            // a/b >= c/d ⟺ ad >= bc
                            let left_cross = l_num * r_denom;
                            let right_cross = r_num * l_denom;
                            Value::Bool(left_cross >= right_cross)
                        }
                        (Value::Rational { numerator: l_num, denominator: l_denom },
                         Value::Number(r_num)) => {
                            // a/b >= c ⟺ a >= bc
                            let left_cross = l_num;
                            let right_cross = r_num * l_denom;
                            Value::Bool(left_cross >= &right_cross)
                        }
                        (Value::Number(l_num),
                         Value::Rational { numerator: r_num, denominator: r_denom }) => {
                            // a >= c/d ⟺ ad >= c
                            let left_cross = l_num * r_denom;
                            let right_cross = r_num;
                            Value::Bool(&left_cross >= right_cross)
                        }
                        _ => Value::Bool(left.to_number()? >= right.to_number()?)
                    }
                }
                "**" => {
                    let l = left.to_number()?;
                    let r = right.to_number()?;
                    let exp = r.to_u32()
                        .ok_or_else(|| "Exponent too large".to_string())?;
                    Value::Number(l.pow(exp))
                }
                ".." => Value::Range {
                    start: left.to_number()?,
                    end: right.to_number()?,
                },
                "and" | "&&" => Value::Bool(left.to_bool() && right.to_bool()),
                "or" | "||" => Value::Bool(left.to_bool() || right.to_bool()),
                _ => return Err(format!("Unknown binary operator: {}", op)),
            };

            Ok((result, ControlFlow::Normal))
        }
    }
}

/// Reduce a rational to canonical form (GCD reduction) and return as integer if denominator = 1
fn reduce_rational(numerator: BigInt, denominator: BigInt) -> Value {
    // Handle zero numerator
    if numerator == BigInt::from(0) {
        return Value::Number(BigInt::from(0));
    }

    // Ensure denominator is always positive (move sign to numerator)
    let (num, denom) = if denominator < BigInt::from(0) {
        (-numerator, -denominator)
    } else {
        (numerator, denominator)
    };

    // Reduce by GCD
    let g = gcd(num.clone(), denom.clone());
    let reduced_num = &num / &g;
    let reduced_denom = &denom / &g;

    // If denominator = 1, return as integer
    if reduced_denom == BigInt::from(1) {
        Value::Number(reduced_num)
    } else {
        Value::Rational {
            numerator: reduced_num,
            denominator: reduced_denom,
        }
    }
}
