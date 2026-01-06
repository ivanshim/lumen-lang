// Stage 4: Execute - Faithful execution of instructions
//
// Apply the 7 primitives with clear, deterministic semantics.
// No language-specific behavior here - just mechanics.

use super::primitives::{Instruction, TransferKind, OperateKind};
use super::eval::Value;
use super::env::Environment;
use crate::schema::LanguageSchema;

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
            // For now, just built-in functions
            match function.as_str() {
                "print" | "print_native" => {
                    for val in &arg_vals {
                        println!("{}", val);
                    }
                    Ok((Value::Null, ControlFlow::Normal))
                }
                _ => Err(format!("Unknown function: {}", function)),
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

        // Function definition: store in environment (simplified)
        Instruction::FunctionDef {
            name,
            params: _,
            body: _,
        } => {
            // Simplified: just mark as defined
            // Full implementation would store in function registry
            env.set(
                name.clone(),
                Value::Function {
                    params: vec![],
                    body_ref: name.clone(),
                },
            );
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
                "-" => Value::Number(-val.to_number()?),
                "not" => Value::Bool(!val.to_bool()),
                _ => return Err(format!("Unknown unary operator: {}", op)),
            };

            Ok((result, ControlFlow::Normal))
        }

        OperateKind::Binary(op) => {
            if operands.len() != 2 {
                return Err("Binary operator requires 2 operands".to_string());
            }

            let (left, left_flow) = execute(&operands[0], env, schema)?;
            if left_flow != ControlFlow::Normal {
                return Ok((left, left_flow));
            }

            // Short-circuit evaluation for logical operators
            match op.as_str() {
                "and" => {
                    if !left.to_bool() {
                        return Ok((Value::Bool(false), ControlFlow::Normal));
                    }
                }
                "or" => {
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
                        Value::Number(left.to_number()? + right.to_number()?)
                    }
                }
                "-" => Value::Number(left.to_number()? - right.to_number()?),
                "*" => Value::Number(left.to_number()? * right.to_number()?),
                "/" => {
                    let r = right.to_number()?;
                    if r == 0.0 {
                        return Err("Division by zero".to_string());
                    }
                    Value::Number(left.to_number()? / r)
                }
                "%" => {
                    let l = left.to_number()? as i64;
                    let r = right.to_number()? as i64;
                    if r == 0 {
                        return Err("Division by zero".to_string());
                    }
                    Value::Number((l % r) as f64)
                }
                "==" => Value::Bool(left == right),
                "!=" => Value::Bool(left != right),
                "<" => Value::Bool(left.to_number()? < right.to_number()?),
                ">" => Value::Bool(left.to_number()? > right.to_number()?),
                "<=" => Value::Bool(left.to_number()? <= right.to_number()?),
                ">=" => Value::Bool(left.to_number()? >= right.to_number()?),
                "and" => Value::Bool(left.to_bool() && right.to_bool()),
                "or" => Value::Bool(left.to_bool() || right.to_bool()),
                _ => return Err(format!("Unknown binary operator: {}", op)),
            };

            Ok((result, ControlFlow::Normal))
        }
    }
}
