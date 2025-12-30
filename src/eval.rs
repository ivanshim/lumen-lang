use crate::ast::*;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Signal {
    None,
    Break,
    Continue,
}

pub fn eval(program: &[Stmt]) -> Result<(), String> {
    let mut env: HashMap<String, Value> = HashMap::new();

    let sig = exec_block(program, &mut env, false)?;
    if sig != Signal::None {
        return Err("SyntaxError: 'break' or 'continue' outside of while".into());
    }

    Ok(())
}

fn exec_block(
    stmts: &[Stmt],
    env: &mut HashMap<String, Value>,
    in_loop: bool,
) -> Result<Signal, String> {
    for stmt in stmts {
        let sig = exec_stmt(stmt, env, in_loop)?;
        if sig != Signal::None {
            return Ok(sig);
        }
    }
    Ok(Signal::None)
}

fn exec_stmt(
    stmt: &Stmt,
    env: &mut HashMap<String, Value>,
    in_loop: bool,
) -> Result<Signal, String> {
    match stmt {
        Stmt::Assign { name, value } => {
            let v = eval_expr(value, env)?;
            env.insert(name.clone(), v);
            Ok(Signal::None)
        }

        Stmt::Print { expr } => {
            let v = eval_expr(expr, env)?;
            println!("{}", display(&v));
            Ok(Signal::None)
        }

        Stmt::If {
            cond,
            then_block,
            else_block,
        } => {
            let c = eval_expr(cond, env)?;
            if truthy(&c) {
                exec_block(then_block, env, in_loop)
            } else if let Some(block) = else_block {
                exec_block(block, env, in_loop)
            } else {
                Ok(Signal::None)
            }
        }

        Stmt::While { cond, body } => {
            loop {
                let c = eval_expr(cond, env)?;
                if !truthy(&c) {
                    break;
                }

                match exec_block(body, env, true)? {
                    Signal::None => {}
                    Signal::Continue => continue,
                    Signal::Break => break,
                }
            }
            Ok(Signal::None)
        }

        Stmt::Break => {
            if !in_loop {
                return Err("SyntaxError: 'break' outside of while".into());
            }
            Ok(Signal::Break)
        }

        Stmt::Continue => {
            if !in_loop {
                return Err("SyntaxError: 'continue' outside of while".into());
            }
            Ok(Signal::Continue)
        }
    }
}

fn eval_expr(expr: &Expr, env: &HashMap<String, Value>) -> Result<Value, String> {
    match expr {
        Expr::Literal(v) => Ok(v.clone()),

        Expr::Var(name) => Ok(env.get(name).cloned().unwrap_or(Value::None)),

        Expr::Unary { op, expr } => {
            let v = eval_expr(expr, env)?;
            match op {
                UnOp::Neg => Ok(Value::Int(-to_int(&v)?)),
                UnOp::Not => Ok(Value::Bool(!truthy(&v))),
            }
        }

        Expr::Binary { left, op, right } => {
            // short-circuit boolean logic
            if matches!(op, BinOp::And) {
                let l = eval_expr(left, env)?;
                if !truthy(&l) {
                    return Ok(Value::Bool(false));
                }
                let r = eval_expr(right, env)?;
                return Ok(Value::Bool(truthy(&r)));
            }

            if matches!(op, BinOp::Or) {
                let l = eval_expr(left, env)?;
                if truthy(&l) {
                    return Ok(Value::Bool(true));
                }
                let r = eval_expr(right, env)?;
                return Ok(Value::Bool(truthy(&r)));
            }

            let l = eval_expr(left, env)?;
            let r = eval_expr(right, env)?;

            match op {
                BinOp::Add => Ok(Value::Int(to_int(&l)? + to_int(&r)?)),
                BinOp::Sub => Ok(Value::Int(to_int(&l)? - to_int(&r)?)),
                BinOp::Mul => Ok(Value::Int(to_int(&l)? * to_int(&r)?)),
                BinOp::Div => {
                    let denom = to_int(&r)?;
                    if denom == 0 {
                        return Err("ZeroDivisionError".into());
                    }
                    Ok(Value::Int(to_int(&l)? / denom))
                }
                BinOp::And | BinOp::Or => unreachable!(),
            }
        }

        Expr::Compare { left, op, right } => {
            let l = eval_expr(left, env)?;
            let r = eval_expr(right, env)?;

            let b = match op {
                CmpOp::Eq => eq(&l, &r),
                CmpOp::Ne => !eq(&l, &r),
                CmpOp::Lt => to_int(&l)? < to_int(&r)?,
                CmpOp::Le => to_int(&l)? <= to_int(&r)?,
                CmpOp::Gt => to_int(&l)? > to_int(&r)?,
                CmpOp::Ge => to_int(&l)? >= to_int(&r)?,
            };

            Ok(Value::Bool(b))
        }
    }
}

// ---------- helpers ----------

fn to_int(v: &Value) -> Result<i64, String> {
    match v {
        Value::Int(n) => Ok(*n),
        _ => Err("TypeError: expected int".into()),
    }
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::None => false,
        Value::Bool(b) => *b,
        Value::Int(n) => *n != 0,
    }
}

fn eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::None, Value::None) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Int(x), Value::Int(y)) => x == y,
        _ => false,
    }
}

fn display(v: &Value) -> String {
    match v {
        Value::None => "None".into(),
        Value::Bool(true) => "true".into(),
        Value::Bool(false) => "false".into(),
        Value::Int(n) => n.to_string(),
    }
}
