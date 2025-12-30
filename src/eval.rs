use crate::ast::*;
use std::collections::HashMap;

#[derive(Clone)]
enum Func {
    User { params: Vec<String>, body: Vec<Stmt> },
    Builtin(fn(Vec<Value>) -> Result<Value, String>),
}

#[derive(Clone)]
struct Env {
    vars: HashMap<String, Value>,
    funcs: HashMap<String, Func>,
}

impl Env {
    fn new() -> Self {
        let mut env = Self {
            vars: HashMap::new(),
            funcs: HashMap::new(),
        };

        // builtins
        env.funcs.insert(
            "print".into(),
            Func::Builtin(|args| {
                if args.len() != 1 {
                    return Err("print() expects 1 argument".into());
                }
                println!("{}", display(&args[0]));
                Ok(Value::None)
            }),
        );

        env
    }
}

pub fn eval(program: &[Stmt]) -> Result<(), String> {
    let mut env = Env::new();
    exec_block(program, &mut env).map(|_| ())
}

fn exec_block(stmts: &[Stmt], env: &mut Env) -> Result<Option<Value>, String> {
    for s in stmts {
        if let Some(ret) = exec_stmt(s, env)? {
            return Ok(Some(ret));
        }
    }
    Ok(None)
}

fn exec_stmt(stmt: &Stmt, env: &mut Env) -> Result<Option<Value>, String> {
    match stmt {
        Stmt::Assign { name, value } => {
            let v = eval_expr(value, env)?;
            env.vars.insert(name.clone(), v);
            Ok(None)
        }

        Stmt::ExprStmt(expr) => {
            let _ = eval_expr(expr, env)?;
            Ok(None)
        }

        Stmt::Print { expr } => {
            let v = eval_expr(expr, env)?;
            println!("{}", display(&v));
            Ok(None)
        }

        Stmt::If { branches, else_block } => {
            for (cond, block) in branches {
                if truthy(&eval_expr(cond, env)?) {
                    return exec_block(block, env);
                }
            }
            if let Some(block) = else_block {
                return exec_block(block, env);
            }
            Ok(None)
        }

        Stmt::While { cond, body } => {
            while truthy(&eval_expr(cond, env)?) {
                if let Some(ret) = exec_block(body, env)? {
                    return Ok(Some(ret));
                }
            }
            Ok(None)
        }

        Stmt::ForRange { name, start, end, body } => {
            let s = to_number(&eval_expr(start, env)?)? as i64;
            let e = to_number(&eval_expr(end, env)?)? as i64;
            for i in s..e {
                env.vars.insert(name.clone(), Value::Number(i as f64));
                if let Some(ret) = exec_block(body, env)? {
                    return Ok(Some(ret));
                }
            }
            Ok(None)
        }

        Stmt::FnDef { name, params, body } => {
            env.funcs.insert(
                name.clone(),
                Func::User {
                    params: params.clone(),
                    body: body.clone(),
                },
            );
            Ok(None)
        }

        Stmt::Return(expr) => {
            let v = match expr {
                Some(e) => eval_expr(e, env)?,
                None => Value::None,
            };
            Ok(Some(v))
        }
    }
}

fn eval_expr(expr: &Expr, env: &mut Env) -> Result<Value, String> {
    match expr {
        Expr::Literal(v) => Ok(v.clone()),
        Expr::Var(name) => Ok(env.vars.get(name).cloned().unwrap_or(Value::None)),

        Expr::Unary { op, expr } => {
            let v = eval_expr(expr, env)?;
            match op {
                UnOp::Neg => Ok(Value::Number(-to_number(&v)?)),
                UnOp::Not => Ok(Value::Bool(!truthy(&v))),
            }
        }

        Expr::Binary { left, op, right } => {
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
                BinOp::Add => match (&l, &r) {
                    (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a + b)),
                    (Value::Str(a), Value::Str(b)) => Ok(Value::Str(format!("{a}{b}"))),
                    _ => Err("TypeError: '+' supports number+number or string+string".into()),
                },
                BinOp::Sub => Ok(Value::Number(to_number(&l)? - to_number(&r)?)),
                BinOp::Mul => Ok(Value::Number(to_number(&l)? * to_number(&r)?)),
                BinOp::Div => Ok(Value::Number(to_number(&l)? / to_number(&r)?)),
                BinOp::And | BinOp::Or => unreachable!(),
            }
        }

        Expr::Compare { left, op, right } => {
            let l = eval_expr(left, env)?;
            let r = eval_expr(right, env)?;
            let b = match op {
                CmpOp::Eq => eq(&l, &r),
                CmpOp::Ne => !eq(&l, &r),
                CmpOp::Lt => to_number(&l)? < to_number(&r)?,
                CmpOp::Le => to_number(&l)? <= to_number(&r)?,
                CmpOp::Gt => to_number(&l)? > to_number(&r)?,
                CmpOp::Ge => to_number(&l)? >= to_number(&r)?,
            };
            Ok(Value::Bool(b))
        }

        Expr::Call { callee, args } => {
            let callee = eval_expr(callee, env)?;
            let fname = match callee {
                Value::Str(s) => s,
                Value::None => return Err("TypeError: cannot call None".into()),
                _ => {
                    // allow calling bare identifiers: foo(...)
                    // if callee was Var("foo"), we resolved to None above, so we need a special case:
                    // easiest: disallow non-string callees and detect Var earlier.
                    return Err("TypeError: call expects a function name".into());
                }
            };

            let f = env
                .funcs
                .get(&fname)
                .cloned()
                .ok_or_else(|| format!("NameError: undefined function '{fname}'"))?;

            let mut argv = Vec::new();
            for a in args {
                argv.push(eval_expr(a, env)?);
            }

            call_func(&f, argv, env)
        }

        Expr::Index { base, index } => {
            let b = eval_expr(base, env)?;
            let i = eval_expr(index, env)?;
            let idx = to_number(&i)? as i64;
            match b {
                Value::List(items) => {
                    let u = usize::try_from(idx).map_err(|_| "IndexError".to_string())?;
                    items.get(u).cloned().ok_or_else(|| "IndexError".into())
                }
                _ => Err("TypeError: indexing requires a list".into()),
            }
        }

        Expr::List(items) => {
            let mut out = Vec::new();
            for it in items {
                out.push(eval_expr(it, env)?);
            }
            Ok(Value::List(out))
        }
    }
}

// --- function calling ---

fn call_func(f: &Func, args: Vec<Value>, env: &mut Env) -> Result<Value, String> {
    match f {
        Func::Builtin(fun) => fun(args),
        Func::User { params, body } => {
            if args.len() != params.len() {
                return Err(format!(
                    "TypeError: expected {} args, got {}",
                    params.len(),
                    args.len()
                ));
            }

            // new local environment: shallow copy funcs, fresh vars
            let mut local = Env {
                vars: HashMap::new(),
                funcs: env.funcs.clone(),
            };

            for (p, a) in params.iter().zip(args.iter()) {
                local.vars.insert(p.clone(), a.clone());
            }

            if let Some(v) = exec_block(body, &mut local)? {
                Ok(v)
            } else {
                Ok(Value::None)
            }
        }
    }
}

// --- helpers ---

fn to_number(v: &Value) -> Result<f64, String> {
    match v {
        Value::Number(n) => Ok(*n),
        _ => Err("TypeError: expected number".into()),
    }
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::None => false,
        Value::Bool(b) => *b,
        Value::Number(n) => *n != 0.0,
        Value::Str(s) => !s.is_empty(),
        Value::List(xs) => !xs.is_empty(),
    }
}

fn eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::None, Value::None) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Number(x), Value::Number(y)) => x == y,
        (Value::Str(x), Value::Str(y)) => x == y,
        _ => false,
    }
}

fn display(v: &Value) -> String {
    match v {
        Value::None => "None".into(),
        Value::Bool(b) => (if *b { "true" } else { "false" }).into(),
        Value::Number(n) => {
            if (n.fract() - 0.0).abs() < 1e-12 {
                format!("{}", *n as i64)
            } else {
                format!("{}", n)
            }
        }
        Value::Str(s) => s.clone(),
        Value::List(xs) => {
            let inner: Vec<String> = xs.iter().map(display).collect();
            format!("[{}]", inner.join(", "))
        }
    }
}
