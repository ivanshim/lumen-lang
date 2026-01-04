// Stage 4: Execution
//
// Execute instruction trees using the primitive dispatch system.
// Each primitive is language-agnostic and executes via its own rules.

use super::primitives::{Instruction, Primitive, TransferKind, OperateKind};
use super::eval::Value;
use super::env::Environment;
use crate::schema::LanguageSchema;

/// Control flow signal from instruction execution
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ControlFlow {
    /// Normal execution continues
    Normal,

    /// Break from loop
    Break,

    /// Continue to next iteration
    Continue,

    /// Return from function
    Return,
}

/// Execute an instruction tree
pub fn execute(instruction: &Instruction, env: &mut Environment, schema: &LanguageSchema) -> Result<(Value, ControlFlow), String> {
    match &instruction.primitive {
        Primitive::Sequence(instrs) => {
            let mut last_value = Value::Null;
            for instr in instrs {
                let (value, flow) = execute(instr, env, schema)?;
                last_value = value;
                if flow != ControlFlow::Normal {
                    return Ok((last_value, flow));
                }
            }
            Ok((last_value, ControlFlow::Normal))
        }

        Primitive::Scope(instrs) => {
            env.push_scope();
            let result = execute(&Instruction::sequence(instrs.clone()), env, schema);
            env.pop_scope();
            result
        }

        Primitive::Branch {
            condition,
            then_block,
            else_block,
        } => {
            let (cond_value, _) = execute(condition, env, schema)?;
            if cond_value.to_bool() {
                execute(then_block, env, schema)
            } else if let Some(else_inst) = else_block {
                execute(else_inst, env, schema)
            } else {
                Ok((Value::Null, ControlFlow::Normal))
            }
        }

        Primitive::Assign { name, value } => {
            let (val, _) = execute(value, env, schema)?;
            // Try to update existing variable in any scope; if not found, create new variable
            if env.update(name.clone(), val.clone()).is_err() {
                env.set(name.clone(), val.clone());
            }
            Ok((val, ControlFlow::Normal))
        }

        Primitive::Invoke { selector, args } => {
            let mut eval_args = Vec::new();
            for arg in args {
                let (val, _) = execute(arg, env, schema)?;
                eval_args.push(val);
            }
            // Dispatch to external function handler
            let result = crate::runtime::execute_extern(selector, eval_args, schema)?;
            Ok((result, ControlFlow::Normal))
        }

        Primitive::Operate { kind, operands } => {
            match kind {
                OperateKind::Unary(operator) => {
                    if operands.len() != 1 {
                        return Err(format!("Unary operator {} expects 1 operand, got {}", operator, operands.len()));
                    }
                    let (operand_val, _) = execute(&operands[0], env, schema)?;
                    let result = execute_unary_op(operator, &operand_val)?;
                    Ok((result, ControlFlow::Normal))
                }
                OperateKind::Binary(operator) => {
                    if operands.len() != 2 {
                        return Err(format!("Binary operator {} expects 2 operands, got {}", operator, operands.len()));
                    }
                    let (left_val, _) = execute(&operands[0], env, schema)?;
                    let (right_val, _) = execute(&operands[1], env, schema)?;
                    let result = execute_binary_op(operator, &left_val, &right_val)?;
                    Ok((result, ControlFlow::Normal))
                }
            }
        }

        Primitive::Transfer { kind, value } => {
            let val = if let Some(v) = value {
                let (val, _) = execute(v, env, schema)?;
                val
            } else {
                Value::Null
            };

            match kind {
                TransferKind::Return => Ok((val, ControlFlow::Return)),
                TransferKind::Break => Ok((Value::Null, ControlFlow::Break)),
                TransferKind::Continue => Ok((Value::Null, ControlFlow::Continue)),
            }
        }

        Primitive::Literal(val) => Ok((val.clone(), ControlFlow::Normal)),

        Primitive::Variable(name) => {
            let val = env.get(name)?;
            Ok((val, ControlFlow::Normal))
        }

        Primitive::Loop { condition, block } => {
            let mut last_value = Value::Null;
            loop {
                let (cond_value, _) = execute(condition, env, schema)?;
                if !cond_value.to_bool() {
                    break;
                }

                let (value, flow) = execute(block, env, schema)?;
                last_value = value;

                match flow {
                    ControlFlow::Break => break,
                    ControlFlow::Continue => continue,
                    ControlFlow::Normal => {}
                    _ => return Ok((last_value, flow)),
                }
            }
            Ok((last_value, ControlFlow::Normal))
        }
    }
}

/// Execute a unary operation
fn execute_unary_op(operator: &str, operand: &Value) -> Result<Value, String> {
    match operator {
        // Logical NOT
        "not" | "!" => Ok(Value::Bool(!operand.to_bool())),

        // Negation
        "-" => {
            let n = operand.to_number()?;
            Ok(Value::Number(-n))
        }

        // Reference and dereference (not fully implemented, just pass through)
        "&" | "*" => Ok(operand.clone()),

        _ => Err(format!("Unknown unary operator: {}", operator)),
    }
}

/// Execute a binary operation
fn execute_binary_op(operator: &str, left: &Value, right: &Value) -> Result<Value, String> {
    match operator {
        // Arithmetic
        "+" => {
            let l = left.to_number()?;
            let r = right.to_number()?;
            Ok(Value::Number(l + r))
        }
        "-" => {
            let l = left.to_number()?;
            let r = right.to_number()?;
            Ok(Value::Number(l - r))
        }
        "*" => {
            let l = left.to_number()?;
            let r = right.to_number()?;
            Ok(Value::Number(l * r))
        }
        "/" => {
            let l = left.to_number()?;
            let r = right.to_number()?;
            if r == 0.0 {
                return Err("Division by zero".to_string());
            }
            Ok(Value::Number(l / r))
        }
        "%" => {
            let l = left.to_number()? as i64;
            let r = right.to_number()? as i64;
            if r == 0 {
                return Err("Division by zero".to_string());
            }
            Ok(Value::Number((l % r) as f64))
        }
        "//" => {
            let l = left.to_number()?;
            let r = right.to_number()?;
            if r == 0.0 {
                return Err("Division by zero".to_string());
            }
            Ok(Value::Number((l / r).floor()))
        }
        "**" => {
            let l = left.to_number()?;
            let r = right.to_number()?;
            Ok(Value::Number(l.powf(r)))
        }

        // Bitwise operations (treated as integer operations)
        "|" => {
            let l = left.to_number()? as i64;
            let r = right.to_number()? as i64;
            Ok(Value::Number((l | r) as f64))
        }
        "^" => {
            let l = left.to_number()? as i64;
            let r = right.to_number()? as i64;
            Ok(Value::Number((l ^ r) as f64))
        }
        "&" => {
            let l = left.to_number()? as i64;
            let r = right.to_number()? as i64;
            Ok(Value::Number((l & r) as f64))
        }
        "<<" => {
            let l = left.to_number()? as i64;
            let r = right.to_number()? as i64;
            Ok(Value::Number((l << r) as f64))
        }
        ">>" => {
            let l = left.to_number()? as i64;
            let r = right.to_number()? as i64;
            Ok(Value::Number((l >> r) as f64))
        }

        // Comparison
        "==" => {
            let result = left.equals(right)?;
            Ok(Value::Bool(result))
        }
        "!=" => {
            let result = left.equals(right)?;
            Ok(Value::Bool(!result))
        }
        "<" => {
            let l = left.to_number()?;
            let r = right.to_number()?;
            Ok(Value::Bool(l < r))
        }
        ">" => {
            let l = left.to_number()?;
            let r = right.to_number()?;
            Ok(Value::Bool(l > r))
        }
        "<=" => {
            let l = left.to_number()?;
            let r = right.to_number()?;
            Ok(Value::Bool(l <= r))
        }
        ">=" => {
            let l = left.to_number()?;
            let r = right.to_number()?;
            Ok(Value::Bool(l >= r))
        }

        // Logical
        "and" | "&&" => {
            let l = left.to_bool();
            let r = right.to_bool();
            Ok(Value::Bool(l && r))
        }
        "or" | "||" => {
            let l = left.to_bool();
            let r = right.to_bool();
            Ok(Value::Bool(l || r))
        }

        // Range operator (creates a simple range representation)
        ".." => {
            Ok(Value::String(format!("{}..{}", left, right)))
        }

        // Assignment (should not reach here in normal execution)
        "=" => Ok(right.clone()),

        _ => Err(format!("Unknown binary operator: {}", operator)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal_execution() {
        let mut env = Environment::new();
        let instr = Instruction::literal(Value::Number(42.0), 0, 2);
        let (val, flow) = execute(&instr, &mut env).unwrap();
        assert_eq!(val, Value::Number(42.0));
        assert_eq!(flow, ControlFlow::Normal);
    }

    #[test]
    fn test_unary_op() {
        assert_eq!(
            execute_unary_op("not", &Value::Bool(true)).unwrap(),
            Value::Bool(false)
        );
        assert_eq!(
            execute_unary_op("-", &Value::Number(5.0)).unwrap(),
            Value::Number(-5.0)
        );
    }

    #[test]
    fn test_binary_op() {
        assert_eq!(
            execute_binary_op("+", &Value::Number(3.0), &Value::Number(4.0)).unwrap(),
            Value::Number(7.0)
        );
        assert_eq!(
            execute_binary_op("==", &Value::Number(3.0), &Value::Number(3.0)).unwrap(),
            Value::Bool(true)
        );
    }
}
