// Stage 4: Execute - Faithful execution of instructions
//
// Apply the 7 primitives with clear, deterministic semantics.
// No language-specific behavior here - just mechanics.

use super::primitives::{Instruction, TransferKind, OperateKind};
use super::eval::{Value, KindValue};
use super::env::Environment;
use crate::schema::LanguageSchema;
use num_bigint::BigInt;
use num_traits::cast::ToPrimitive;
use num_traits::Signed;
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
            // Special handling for push: push(arr, value)
            // First argument should be a Variable (not evaluated), second is the value
            if function == "push" {
                if args.len() != 2 {
                    return Err(format!("push() expects 2 arguments, got {}", args.len()));
                }

                // Extract array variable name from first argument
                let arr_name = match &args[0] {
                    Instruction::Variable(name) => name.clone(),
                    _ => return Err("First argument to push() must be an array variable name".to_string()),
                };

                // Evaluate the value to push
                let (val, flow) = execute(&args[1], env, _schema)?;
                if flow != ControlFlow::Normal {
                    return Ok((val, flow));
                }

                // Push to array
                env.push_to_array(&arr_name, val.clone())?;
                return Ok((Value::Null, ControlFlow::Normal));
            }

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
                "real" => {
                    // real(x) or real(x, y): convert to real with configurable precision
                    // Default precision is 15 significant digits
                    if arg_vals.is_empty() || arg_vals.len() > 2 {
                        return Err(format!("real() expects 1 or 2 arguments, got {}", arg_vals.len()));
                    }

                    let precision = if arg_vals.len() == 2 {
                        match &arg_vals[1] {
                            Value::Number(n) => {
                                n.to_u64()
                                    .ok_or_else(|| "Precision must be a positive integer".to_string())? as usize
                            }
                            _ => return Err("Precision argument must be an integer".to_string()),
                        }
                    } else {
                        15 // Default precision
                    };

                    match &arg_vals[0] {
                        Value::Number(n) => {
                            // Integer → Real
                            Ok((Value::Real {
                                numerator: n.clone(),
                                denominator: BigInt::from(1),
                                precision,
                            }, ControlFlow::Normal))
                        }
                        Value::Rational { numerator, denominator } => {
                            // Rational → Real
                            Ok((Value::Real {
                                numerator: numerator.clone(),
                                denominator: denominator.clone(),
                                precision,
                            }, ControlFlow::Normal))
                        }
                        Value::Real { numerator, denominator, .. } => {
                            // Real → Real (with new precision)
                            Ok((Value::Real {
                                numerator: numerator.clone(),
                                denominator: denominator.clone(),
                                precision,
                            }, ControlFlow::Normal))
                        }
                        _ => Err("real() requires a number, rational, or real argument".to_string()),
                    }
                }
                "int_to_string" => {
                    // int_to_string(x): convert integer to string (mechanical primitive)
                    // Assumes input is INTEGER. No type branching.
                    if arg_vals.len() != 1 {
                        return Err(format!("int_to_string() expects 1 argument, got {}", arg_vals.len()));
                    }
                    match &arg_vals[0] {
                        Value::Number(n) => Ok((Value::String(n.to_string()), ControlFlow::Normal)),
                        _ => Err("int_to_string() requires an integer argument".to_string()),
                    }
                }
                "real_to_string" => {
                    // real_to_string(x): convert real to string (mechanical primitive)
                    // Assumes input is REAL. No type branching.
                    if arg_vals.len() != 1 {
                        return Err(format!("real_to_string() expects 1 argument, got {}", arg_vals.len()));
                    }
                    match &arg_vals[0] {
                        Value::Real { numerator, denominator, precision } => {
                            // Format real as decimal with precision
                            let int_part = numerator / denominator;
                            let remainder = numerator.clone() - (&int_part * denominator);
                            if remainder == BigInt::from(0) {
                                Ok((Value::String(int_part.to_string()), ControlFlow::Normal))
                            } else {
                                let mut decimal_str = String::new();
                                let digit_count = int_part.to_string().len();
                                let target_digits = *precision;
                                let mut rem = remainder.abs();
                                let mut frac_digits = if digit_count >= target_digits {
                                    0
                                } else {
                                    target_digits - digit_count
                                };
                                let denom = denominator.clone();
                                while frac_digits > 0 && rem > BigInt::from(0) {
                                    rem = rem * BigInt::from(10);
                                    let digit = &rem / &denom;
                                    decimal_str.push_str(&digit.to_string());
                                    rem = &rem - (&digit * &denom);
                                    frac_digits -= 1;
                                }
                                Ok((Value::String(format!("{}.{}", int_part, decimal_str)), ControlFlow::Normal))
                            }
                        }
                        _ => Err("real_to_string() requires a real argument".to_string()),
                    }
                }
                "rational_to_string" => {
                    // rational_to_string(x): convert rational to string (mechanical primitive)
                    // Assumes input is RATIONAL. No type branching.
                    if arg_vals.len() != 1 {
                        return Err(format!("rational_to_string() expects 1 argument, got {}", arg_vals.len()));
                    }
                    match &arg_vals[0] {
                        Value::Rational { numerator, denominator } => {
                            let string = if denominator == &BigInt::from(1) {
                                numerator.to_string()
                            } else {
                                format!("{}/{}", numerator, denominator)
                            };
                            Ok((Value::String(string), ControlFlow::Normal))
                        }
                        _ => Err("rational_to_string() requires a rational argument".to_string()),
                    }
                }
                "bool_to_string" => {
                    // bool_to_string(x): convert boolean to string (mechanical primitive)
                    // Assumes input is BOOLEAN. No type branching.
                    if arg_vals.len() != 1 {
                        return Err(format!("bool_to_string() expects 1 argument, got {}", arg_vals.len()));
                    }
                    match &arg_vals[0] {
                        Value::Bool(b) => {
                            let string = if *b { "true" } else { "false" };
                            Ok((Value::String(string.to_string()), ControlFlow::Normal))
                        }
                        _ => Err("bool_to_string() requires a boolean argument".to_string()),
                    }
                }
                "array_to_string" => {
                    // array_to_string(x): convert array to string (mechanical primitive)
                    // Assumes input is ARRAY. No type branching.
                    if arg_vals.len() != 1 {
                        return Err(format!("array_to_string() expects 1 argument, got {}", arg_vals.len()));
                    }
                    match &arg_vals[0] {
                        Value::Array(elements) => {
                            let elements_str = elements
                                .iter()
                                .map(|e| format!("{}", e))
                                .collect::<Vec<_>>()
                                .join(", ");
                            Ok((Value::String(format!("[{}]", elements_str)), ControlFlow::Normal))
                        }
                        _ => Err("array_to_string() requires an array argument".to_string()),
                    }
                }
                "none_to_string" => {
                    // none_to_string(x): convert none to string (mechanical primitive)
                    // Assumes input is NONE. No type branching.
                    if arg_vals.len() != 1 {
                        return Err(format!("none_to_string() expects 1 argument, got {}", arg_vals.len()));
                    }
                    match &arg_vals[0] {
                        Value::Null => Ok((Value::String("none".to_string()), ControlFlow::Normal)),
                        _ => Err("none_to_string() requires a none argument".to_string()),
                    }
                }
                "kind_to_string" => {
                    // kind_to_string(x): convert kind meta-value to string (mechanical primitive)
                    // Assumes input is KIND. No type branching.
                    if arg_vals.len() != 1 {
                        return Err(format!("kind_to_string() expects 1 argument, got {}", arg_vals.len()));
                    }
                    match &arg_vals[0] {
                        Value::Kind(k) => {
                            let string = match k {
                                KindValue::INTEGER => "INTEGER",
                                KindValue::RATIONAL => "RATIONAL",
                                KindValue::REAL => "REAL",
                                KindValue::STRING => "STRING",
                                KindValue::BOOLEAN => "BOOLEAN",
                                KindValue::ARRAY => "ARRAY",
                                KindValue::NONE => "NONE",
                            };
                            Ok((Value::String(string.to_string()), ControlFlow::Normal))
                        }
                        _ => Err("kind_to_string() requires a kind argument".to_string()),
                    }
                }
                "len" => {
                    // len(x): return length of string or array
                    // For strings, counts UTF-8 characters (not bytes)
                    if arg_vals.len() != 1 {
                        return Err(format!("len() expects 1 argument, got {}", arg_vals.len()));
                    }
                    match &arg_vals[0] {
                        Value::String(s) => {
                            let len = s.chars().count();
                            Ok((Value::Number(BigInt::from(len)), ControlFlow::Normal))
                        }
                        Value::Array(arr) => {
                            let len = arr.len();
                            Ok((Value::Number(BigInt::from(len)), ControlFlow::Normal))
                        }
                        _ => Err("len() requires a string or array argument".to_string()),
                    }
                }
                "char_at" => {
                    // char_at(string, index): return character at index
                    // Characters are UTF-8 characters (not bytes)
                    // Returns null if index is out of bounds or negative
                    if arg_vals.len() != 2 {
                        return Err(format!("char_at() expects 2 arguments, got {}", arg_vals.len()));
                    }
                    match (&arg_vals[0], &arg_vals[1]) {
                        (Value::String(s), Value::Number(idx)) => {
                            // Convert index to usize
                            match idx.to_usize() {
                                Some(i) => {
                                    // Get character at index
                                    match s.chars().nth(i) {
                                        Some(ch) => Ok((Value::String(ch.to_string()), ControlFlow::Normal)),
                                        None => Ok((Value::Null, ControlFlow::Normal)), // Out of bounds
                                    }
                                }
                                None => Ok((Value::Null, ControlFlow::Normal)), // Negative or too large
                            }
                        }
                        (Value::String(_), _) => Err("char_at() second argument must be an integer".to_string()),
                        _ => Err("char_at() first argument must be a string".to_string()),
                    }
                }
                "ord" => {
                    // ord(s): return decimal integer value of first character
                    // Returns the UTF-8 code point of the first character
                    if arg_vals.len() != 1 {
                        return Err(format!("ord() expects 1 argument, got {}", arg_vals.len()));
                    }
                    match &arg_vals[0] {
                        Value::String(s) => {
                            // Check if string is empty
                            if s.is_empty() {
                                return Err("ord() requires a non-empty string".to_string());
                            }
                            // Get first character and convert to Unicode code point
                            let first_char = s.chars().next().unwrap();
                            let code_point = first_char as u32;
                            Ok((Value::Number(BigInt::from(code_point)), ControlFlow::Normal))
                        }
                        _ => Err("ord() requires a string argument".to_string()),
                    }
                }
                "chr" => {
                    // chr(n): return single-character string for decimal integer
                    // Returns a string containing the character for the given Unicode code point
                    if arg_vals.len() != 1 {
                        return Err(format!("chr() expects 1 argument, got {}", arg_vals.len()));
                    }
                    match &arg_vals[0] {
                        Value::Number(n) => {
                            // Convert to u32 for char conversion
                            let code_point = n.to_u32()
                                .ok_or_else(|| "chr() argument must be a non-negative integer within valid Unicode range".to_string())?;
                            // Convert to char (validates Unicode code point)
                            let character = char::from_u32(code_point)
                                .ok_or_else(|| format!("chr() argument {} is not a valid Unicode code point", code_point))?;
                            Ok((Value::String(character.to_string()), ControlFlow::Normal))
                        }
                        _ => Err("chr() requires an integer argument".to_string()),
                    }
                }
                "kind" => {
                    // kind(x): return kind meta-value representing value category
                    // Returns one of the predefined kind constants: INTEGER, RATIONAL, REAL, ARRAY, STRING, BOOLEAN, NONE
                    if arg_vals.len() != 1 {
                        return Err(format!("kind() expects 1 argument, got {}", arg_vals.len()));
                    }
                    let kind_val = match &arg_vals[0] {
                        Value::Number(_) => KindValue::INTEGER,
                        Value::Rational { .. } => KindValue::RATIONAL,
                        Value::Real { .. } => KindValue::REAL,
                        Value::Array(_) => KindValue::ARRAY,
                        Value::String(_) => KindValue::STRING,
                        Value::Bool(_) => KindValue::BOOLEAN,
                        Value::Null => KindValue::NONE,
                        Value::Kind(_) => KindValue::NONE, // KIND-of-KIND returns NONE as placeholder
                        _ => return Err("kind(): unknown value type".to_string()),
                    };
                    Ok((Value::Kind(kind_val), ControlFlow::Normal))
                }
                "num" => {
                    // num(x): extract numerator from rational
                    // Valid only for RATIONAL values, returns numerator as INTEGER
                    if arg_vals.len() != 1 {
                        return Err(format!("num() expects 1 argument, got {}", arg_vals.len()));
                    }
                    match &arg_vals[0] {
                        Value::Rational { numerator, .. } => {
                            Ok((Value::Number(numerator.clone()), ControlFlow::Normal))
                        }
                        _ => Err("num() requires a rational argument".to_string()),
                    }
                }
                "den" => {
                    // den(x): extract denominator from rational
                    // Valid only for RATIONAL values, returns denominator as INTEGER
                    if arg_vals.len() != 1 {
                        return Err(format!("den() expects 1 argument, got {}", arg_vals.len()));
                    }
                    match &arg_vals[0] {
                        Value::Rational { denominator, .. } => {
                            Ok((Value::Number(denominator.clone()), ControlFlow::Normal))
                        }
                        _ => Err("den() requires a rational argument".to_string()),
                    }
                }
                "int" => {
                    // int(x): extract integer part from real
                    // Valid only for REAL values, returns integer part as INTEGER
                    if arg_vals.len() != 1 {
                        return Err(format!("int() expects 1 argument, got {}", arg_vals.len()));
                    }
                    match &arg_vals[0] {
                        Value::Real { numerator, denominator, .. } => {
                            // Integer part: truncate toward zero (integer division)
                            let int_part = numerator / denominator;
                            Ok((Value::Number(int_part), ControlFlow::Normal))
                        }
                        _ => Err("int() requires a real argument".to_string()),
                    }
                }
                "frac" => {
                    // frac(x): extract fractional part from real
                    // Valid only for REAL values, returns fractional part as REAL
                    if arg_vals.len() != 1 {
                        return Err(format!("frac() expects 1 argument, got {}", arg_vals.len()));
                    }
                    match &arg_vals[0] {
                        Value::Real { numerator, denominator, precision } => {
                            // Fractional part: x - int(x)
                            // frac(x) = (numerator - (numerator / denominator) * denominator) / denominator
                            let int_part = numerator / denominator;
                            let frac_numerator = numerator - (&int_part * denominator);
                            Ok((Value::Real {
                                numerator: frac_numerator,
                                denominator: denominator.clone(),
                                precision: *precision,
                            }, ControlFlow::Normal))
                        }
                        _ => Err("frac() requires a real argument".to_string()),
                    }
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
                                Value::Real { .. } => "real",
                                Value::String(_) => "string",
                                Value::Bool(_) => "bool",
                                Value::Null => "null",
                                Value::Range { .. } => "range",
                                Value::Array(_) => "array",
                                Value::Function { .. } => "function",
                                Value::Symbol(_) => "symbol",
                                Value::Kind(_) => "kind",
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
                "__construct_array" => {
                    // Construct an array from the evaluated arguments
                    Ok((Value::Array(arg_vals), ControlFlow::Normal))
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

        // Indexed assignment: arr[index] = value
        Instruction::IndexedAssign { name, index, value } => {
            // Evaluate index
            let (index_val, flow) = execute(index, env, _schema)?;
            if flow != ControlFlow::Normal {
                return Ok((index_val, flow));
            }

            // Evaluate value
            let (val, flow) = execute(value, env, _schema)?;
            if flow != ControlFlow::Normal {
                return Ok((val, flow));
            }

            // Convert index to usize
            let idx = match &index_val {
                Value::Number(n) => {
                    n.to_usize()
                        .ok_or_else(|| "Array index out of bounds".to_string())?
                }
                _ => return Err("Array index must be a number".to_string()),
            };

            // Mutate the array
            env.mutate_array(name, idx, val.clone())?;
            Ok((val, ControlFlow::Normal))
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
                        Value::Real { numerator, denominator, precision } => {
                            Value::Real { numerator: -numerator, denominator, precision }
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
                "." => {
                    // Period operator: string concatenation with automatic coercion
                    // Coerce both operands to strings using str()
                    let left_str = format!("{}", left);
                    let right_str = format!("{}", right);
                    Value::String(format!("{}{}", left_str, right_str))
                }
                "+" => {
                    if let (Value::String(_), _) | (_, Value::String(_)) = (&left, &right) {
                        Value::String(format!("{}{}", left, right))
                    } else {
                        // Check if either operand is real or rational
                        match (&left, &right) {
                            // Real + Real = Real
                            (Value::Real { numerator: l_num, denominator: l_denom, precision: l_prec },
                             Value::Real { numerator: r_num, denominator: r_denom, precision: r_prec }) => {
                                // (a/b) + (c/d) = (ad + bc) / bd, preserve left precision
                                let num = l_num * r_denom + r_num * l_denom;
                                let denom = l_denom * r_denom;
                                reduce_real(num, denom, *l_prec)
                            }
                            // Real + Rational = Real
                            (Value::Real { numerator: l_num, denominator: l_denom, precision: l_prec },
                             Value::Rational { numerator: r_num, denominator: r_denom }) => {
                                let num = l_num * r_denom + r_num * l_denom;
                                let denom = l_denom * r_denom;
                                reduce_real(num, denom, *l_prec)
                            }
                            // Real + Number = Real
                            (Value::Real { numerator: l_num, denominator: l_denom, precision: l_prec },
                             Value::Number(r_num)) => {
                                let num = l_num + r_num * l_denom;
                                reduce_real(num, l_denom.clone(), *l_prec)
                            }
                            // Rational + Real = Real
                            (Value::Rational { numerator: l_num, denominator: l_denom },
                             Value::Real { numerator: r_num, denominator: r_denom, precision: r_prec }) => {
                                let num = l_num * r_denom + r_num * l_denom;
                                let denom = l_denom * r_denom;
                                reduce_real(num, denom, *r_prec)
                            }
                            // Number + Real = Real
                            (Value::Number(l_num),
                             Value::Real { numerator: r_num, denominator: r_denom, precision: r_prec }) => {
                                let num = l_num * r_denom + r_num;
                                reduce_real(num, r_denom.clone(), *r_prec)
                            }
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
                        // Real - Real = Real
                        (Value::Real { numerator: l_num, denominator: l_denom, precision: l_prec },
                         Value::Real { numerator: r_num, denominator: r_denom, precision: r_prec }) => {
                            // (a/b) - (c/d) = (ad - bc) / bd, preserve left precision
                            let num = l_num * r_denom - r_num * l_denom;
                            let denom = l_denom * r_denom;
                            reduce_real(num, denom, *l_prec)
                        }
                        // Real - Rational = Real
                        (Value::Real { numerator: l_num, denominator: l_denom, precision: l_prec },
                         Value::Rational { numerator: r_num, denominator: r_denom }) => {
                            // (a/b) - (c/d) = (ad - bc) / bd
                            let num = l_num * r_denom - r_num * l_denom;
                            let denom = l_denom * r_denom;
                            reduce_real(num, denom, *l_prec)
                        }
                        // Real - Number = Real
                        (Value::Real { numerator: l_num, denominator: l_denom, precision: l_prec },
                         Value::Number(r_num)) => {
                            // (a/b) - c = (a - bc) / b
                            let num = l_num - r_num * l_denom;
                            reduce_real(num, l_denom.clone(), *l_prec)
                        }
                        // Rational - Real = Real
                        (Value::Rational { numerator: l_num, denominator: l_denom },
                         Value::Real { numerator: r_num, denominator: r_denom, precision: r_prec }) => {
                            // (a/b) - (c/d) = (ad - bc) / bd, preserve right precision
                            let num = l_num * r_denom - r_num * l_denom;
                            let denom = l_denom * r_denom;
                            reduce_real(num, denom, *r_prec)
                        }
                        // Number - Real = Real
                        (Value::Number(l_num),
                         Value::Real { numerator: r_num, denominator: r_denom, precision: r_prec }) => {
                            // a - (c/d) = (ad - c) / d, preserve right precision
                            let num = l_num * r_denom - r_num;
                            reduce_real(num, r_denom.clone(), *r_prec)
                        }
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
                        // Real * Real = Real
                        (Value::Real { numerator: l_num, denominator: l_denom, precision: l_prec },
                         Value::Real { numerator: r_num, denominator: r_denom, precision: r_prec }) => {
                            // (a/b) * (c/d) = (ac) / (bd), preserve left precision
                            let num = l_num * r_num;
                            let denom = l_denom * r_denom;
                            reduce_real(num, denom, *l_prec)
                        }
                        // Real * Rational = Real
                        (Value::Real { numerator: l_num, denominator: l_denom, precision: l_prec },
                         Value::Rational { numerator: r_num, denominator: r_denom }) => {
                            let num = l_num * r_num;
                            let denom = l_denom * r_denom;
                            reduce_real(num, denom, *l_prec)
                        }
                        // Real * Number = Real
                        (Value::Real { numerator: l_num, denominator: l_denom, precision: l_prec },
                         Value::Number(r_num)) => {
                            let num = l_num * r_num;
                            reduce_real(num, l_denom.clone(), *l_prec)
                        }
                        // Rational * Real = Real
                        (Value::Rational { numerator: l_num, denominator: l_denom },
                         Value::Real { numerator: r_num, denominator: r_denom, precision: r_prec }) => {
                            let num = l_num * r_num;
                            let denom = l_denom * r_denom;
                            reduce_real(num, denom, *r_prec)
                        }
                        // Number * Real = Real
                        (Value::Number(l_num),
                         Value::Real { numerator: r_num, denominator: r_denom, precision: r_prec }) => {
                            let num = l_num * r_num;
                            reduce_real(num, r_denom.clone(), *r_prec)
                        }
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
                        // Real / Real = Real
                        (Value::Real { numerator: l_num, denominator: l_denom, precision: l_prec },
                         Value::Real { numerator: r_num, denominator: r_denom, precision: r_prec }) => {
                            // (a/b) / (c/d) = (ad) / (bc), preserve left precision
                            if r_num == &BigInt::from(0) {
                                return Err("Division by zero".to_string());
                            }
                            let num = l_num * r_denom;
                            let denom = l_denom * r_num;
                            reduce_real(num, denom, *l_prec)
                        }
                        // Real / Rational = Real
                        (Value::Real { numerator: l_num, denominator: l_denom, precision: l_prec },
                         Value::Rational { numerator: r_num, denominator: r_denom }) => {
                            if r_num == &BigInt::from(0) {
                                return Err("Division by zero".to_string());
                            }
                            let num = l_num * r_denom;
                            let denom = l_denom * r_num;
                            reduce_real(num, denom, *l_prec)
                        }
                        // Real / Number = Real
                        (Value::Real { numerator: l_num, denominator: l_denom, precision: l_prec },
                         Value::Number(r_num)) => {
                            if r_num == &BigInt::from(0) {
                                return Err("Division by zero".to_string());
                            }
                            let denom = l_denom * r_num;
                            reduce_real(l_num.clone(), denom, *l_prec)
                        }
                        // Rational / Real = Real
                        (Value::Rational { numerator: l_num, denominator: l_denom },
                         Value::Real { numerator: r_num, denominator: r_denom, precision: r_prec }) => {
                            if r_num == &BigInt::from(0) {
                                return Err("Division by zero".to_string());
                            }
                            let num = l_num * r_denom;
                            let denom = l_denom * r_num;
                            reduce_real(num, denom, *r_prec)
                        }
                        // Number / Real = Real
                        (Value::Number(l_num),
                         Value::Real { numerator: r_num, denominator: r_denom, precision: r_prec }) => {
                            if r_num == &BigInt::from(0) {
                                return Err("Division by zero".to_string());
                            }
                            let num = l_num * r_denom;
                            reduce_real(num, r_num.clone(), *r_prec)
                        }
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
                    // For modulo, extract integer parts from rationals
                    let l_int = match &left {
                        Value::Number(n) => n.clone(),
                        Value::Rational { numerator, denominator } => numerator / denominator,
                        _ => return Err("Modulo requires numeric operands".to_string()),
                    };
                    let r_int = match &right {
                        Value::Number(n) => n.clone(),
                        Value::Rational { numerator, denominator } => numerator / denominator,
                        _ => return Err("Modulo requires numeric operands".to_string()),
                    };
                    if r_int == BigInt::from(0) {
                        return Err("Modulo by zero".to_string());
                    }
                    Value::Number(l_int % r_int)
                }
                "//" => {
                    // Integer quotient: a // b returns quotient truncating toward zero
                    // Identity: a == b * (a // b) + (a % b)
                    match (&left, &right) {
                        // Integer // Integer = Integer
                        (Value::Number(l), Value::Number(r)) => {
                            if *r == BigInt::from(0) {
                                return Err("Division by zero".to_string());
                            }
                            Value::Number(l / r)  // Truncates toward zero in Rust
                        }
                        // Integer // Rational = Rational
                        (Value::Number(l), Value::Rational { numerator: r_num, denominator: r_denom }) => {
                            if r_num == &BigInt::from(0) {
                                return Err("Division by zero".to_string());
                            }
                            // l // (r_num/r_denom) = (l * r_denom) // r_num
                            let quot = (l * r_denom) / r_num;
                            reduce_rational(quot, BigInt::from(1))
                        }
                        // Rational // Integer = Rational
                        (Value::Rational { numerator: l_num, denominator: l_denom }, Value::Number(r)) => {
                            if *r == BigInt::from(0) {
                                return Err("Division by zero".to_string());
                            }
                            // (l_num/l_denom) // r = l_num // (r * l_denom)
                            let quot = l_num / (r * l_denom);
                            reduce_rational(quot, BigInt::from(1))
                        }
                        // Rational // Rational = Rational
                        (Value::Rational { numerator: l_num, denominator: l_denom },
                         Value::Rational { numerator: r_num, denominator: r_denom }) => {
                            if r_num == &BigInt::from(0) {
                                return Err("Division by zero".to_string());
                            }
                            // (l_num/l_denom) // (r_num/r_denom) = (l_num * r_denom) // (r_num * l_denom)
                            let quot = (l_num * r_denom) / (r_num * l_denom);
                            reduce_rational(quot, BigInt::from(1))
                        }
                        // Real // ... = Real
                        (Value::Real { numerator: l_num, denominator: l_denom, precision: l_prec }, _) => {
                            let (r_num, r_denom) = match &right {
                                Value::Number(n) => (n.clone(), BigInt::from(1)),
                                Value::Rational { numerator: n, denominator: d } => (n.clone(), d.clone()),
                                Value::Real { numerator: n, denominator: d, .. } => (n.clone(), d.clone()),
                                _ => return Err("Integer quotient requires numeric operands".to_string()),
                            };
                            if r_num == BigInt::from(0) {
                                return Err("Division by zero".to_string());
                            }
                            let quot = (l_num * &r_denom) / (&r_num * l_denom);
                            reduce_real(quot, BigInt::from(1), *l_prec)
                        }
                        // ... // Real = Real (symmetric)
                        (_, Value::Real { numerator: r_num, denominator: r_denom, precision: r_prec }) => {
                            if r_num == &BigInt::from(0) {
                                return Err("Division by zero".to_string());
                            }
                            let (l_num, l_denom) = match &left {
                                Value::Number(n) => (n.clone(), BigInt::from(1)),
                                Value::Rational { numerator: n, denominator: d } => (n.clone(), d.clone()),
                                _ => return Err("Integer quotient requires numeric operands".to_string()),
                            };
                            let quot = (&l_num * r_denom) / (&l_denom * r_num);
                            reduce_real(quot, BigInt::from(1), *r_prec)
                        }
                        _ => return Err("Integer quotient requires numeric operands".to_string()),
                    }
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
                "[]" => {
                    // Array indexing: left is array, right is index
                    let arr = match left {
                        Value::Array(ref elements) => elements,
                        _ => return Err("Cannot index non-array value".to_string()),
                    };

                    // Convert index to usize
                    let idx = match &right {
                        Value::Number(n) => {
                            n.to_usize()
                                .ok_or_else(|| "Array index out of bounds".to_string())?
                        }
                        _ => return Err("Array index must be a number".to_string()),
                    };

                    // Bounds check
                    if idx >= arr.len() {
                        return Err(format!("Array index {} out of bounds (length: {})", idx, arr.len()));
                    }

                    arr[idx].clone()
                }
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

/// Reduce a real to canonical form (like reduce_rational) but preserve precision
fn reduce_real(numerator: BigInt, denominator: BigInt, precision: usize) -> Value {
    // Handle zero numerator
    if numerator == BigInt::from(0) {
        return Value::Real {
            numerator: BigInt::from(0),
            denominator: BigInt::from(1),
            precision,
        };
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

    Value::Real {
        numerator: reduced_num,
        denominator: reduced_denom,
        precision,
    }
}
