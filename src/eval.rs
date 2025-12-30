use crate::ast::*;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Signal {
    None,
    Break,
    Continue,
}

type Env = HashMap<String, Value>;
type ExprEvaluator = fn(&Expr, &mut Env, &EvalRegistry) -> Result<Value, String>;
type StmtEvaluator = fn(&Stmt, &mut Env, &EvalRegistry, bool) -> Result<Signal, String>;

struct EvalRegistry {
    expr_eval: HashMap<ExprTag, ExprEvaluator>,
    stmt_eval: HashMap<StmtTag, StmtEvaluator>,
}

impl EvalRegistry {
    fn new() -> Self {
        Self {
            expr_eval: HashMap::new(),
            stmt_eval: HashMap::new(),
        }
    }

    fn register_expr(&mut self, tag: ExprTag, handler: ExprEvaluator) {
        self.expr_eval.insert(tag, handler);
    }

    fn register_stmt(&mut self, tag: StmtTag, handler: StmtEvaluator) {
        self.stmt_eval.insert(tag, handler);
    }
}

pub fn eval(program: &[Stmt]) -> Result<(), String> {
    let mut env: Env = HashMap::new();
    let registry = build_registry();

    let sig = exec_block(program, &mut env, &registry, false)?;
    if sig != Signal::None {
        return Err("SyntaxError: 'break' or 'continue' outside of while".into());
    }

    Ok(())
}

fn build_registry() -> EvalRegistry {
    let mut registry = EvalRegistry::new();
    register_expr_features(&mut registry);
    register_statement_features(&mut registry);
    register_control_flow_features(&mut registry);
    registry
}

fn exec_block(
    stmts: &[Stmt],
    env: &mut Env,
    registry: &EvalRegistry,
    in_loop: bool,
) -> Result<Signal, String> {
    for stmt in stmts {
        let sig = exec_stmt(stmt, env, registry, in_loop)?;
        if sig != Signal::None {
            return Ok(sig);
        }
    }
    Ok(Signal::None)
}

fn exec_stmt(
    stmt: &Stmt,
    env: &mut Env,
    registry: &EvalRegistry,
    in_loop: bool,
) -> Result<Signal, String> {
    let tag = stmt.tag();
    let handler = registry.stmt_eval.get(&tag).ok_or_else(|| {
        format!(
            "RuntimeError: no evaluator registered for statement tag {:?}",
            tag
        )
    })?;

    handler(stmt, env, registry, in_loop)
}

fn eval_expr(expr: &Expr, env: &mut Env, registry: &EvalRegistry) -> Result<Value, String> {
    let tag = expr.tag();
    let handler = registry.expr_eval.get(&tag).ok_or_else(|| {
        format!(
            "RuntimeError: no evaluator registered for expression tag {:?}",
            tag
        )
    })?;

    handler(expr, env, registry)
}

fn register_expr_features(registry: &mut EvalRegistry) {
    registry.register_expr(ExprTag::Literal, eval_literal);
    registry.register_expr(ExprTag::Var, eval_var);
    registry.register_expr(ExprTag::Unary, eval_unary);
    registry.register_expr(ExprTag::Binary, eval_binary);
    registry.register_expr(ExprTag::Compare, eval_compare);
}

fn register_statement_features(registry: &mut EvalRegistry) {
    registry.register_stmt(StmtTag::Assign, exec_assign);
    registry.register_stmt(StmtTag::Print, exec_print);
    registry.register_stmt(StmtTag::Break, exec_break);
    registry.register_stmt(StmtTag::Continue, exec_continue);
}

fn register_control_flow_features(registry: &mut EvalRegistry) {
    registry.register_stmt(StmtTag::If, exec_if);
    registry.register_stmt(StmtTag::While, exec_while);
}

fn eval_literal(expr: &Expr, _env: &mut Env, _registry: &EvalRegistry) -> Result<Value, String> {
    match expr {
        Expr::Literal(v) => Ok(v.clone()),
        _ => unreachable!("Literal tag assigned to non-literal expression"),
    }
}

fn eval_var(expr: &Expr, env: &mut Env, _registry: &EvalRegistry) -> Result<Value, String> {
    match expr {
        Expr::Var(name) => Ok(env.get(name).cloned().unwrap_or(Value::None)),
        _ => unreachable!("Var tag assigned to non-var expression"),
    }
}

fn eval_unary(expr: &Expr, env: &mut Env, registry: &EvalRegistry) -> Result<Value, String> {
    if let Expr::Unary { op, expr } = expr {
        let v = eval_expr(expr, env, registry)?;
        return match op {
            UnOp::Neg => Ok(Value::Int(-to_int(&v)?)),
            UnOp::Not => Ok(Value::Bool(!truthy(&v))),
        };
    }

    unreachable!("Unary tag assigned to non-unary expression")
}

fn eval_binary(expr: &Expr, env: &mut Env, registry: &EvalRegistry) -> Result<Value, String> {
    if let Expr::Binary { left, op, right } = expr {
        if matches!(op, BinOp::And) {
            let l = eval_expr(left, env, registry)?;
            if !truthy(&l) {
                return Ok(Value::Bool(false));
            }
            let r = eval_expr(right, env, registry)?;
            return Ok(Value::Bool(truthy(&r)));
        }

        if matches!(op, BinOp::Or) {
            let l = eval_expr(left, env, registry)?;
            if truthy(&l) {
                return Ok(Value::Bool(true));
            }
            let r = eval_expr(right, env, registry)?;
            return Ok(Value::Bool(truthy(&r)));
        }

        let l = eval_expr(left, env, registry)?;
        let r = eval_expr(right, env, registry)?;

        return match op {
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
        };
    }

    unreachable!("Binary tag assigned to non-binary expression")
}

fn eval_compare(expr: &Expr, env: &mut Env, registry: &EvalRegistry) -> Result<Value, String> {
    if let Expr::Compare { left, op, right } = expr {
        let l = eval_expr(left, env, registry)?;
        let r = eval_expr(right, env, registry)?;

        let b = match op {
            CmpOp::Eq => eq(&l, &r),
            CmpOp::Ne => !eq(&l, &r),
            CmpOp::Lt => to_int(&l)? < to_int(&r)?,
            CmpOp::Le => to_int(&l)? <= to_int(&r)?,
            CmpOp::Gt => to_int(&l)? > to_int(&r)?,
            CmpOp::Ge => to_int(&l)? >= to_int(&r)?,
        };

        return Ok(Value::Bool(b));
    }

    unreachable!("Compare tag assigned to non-compare expression")
}

fn exec_assign(
    stmt: &Stmt,
    env: &mut Env,
    registry: &EvalRegistry,
    _in_loop: bool,
) -> Result<Signal, String> {
    if let Stmt::Assign { name, value } = stmt {
        let v = eval_expr(value, env, registry)?;
        env.insert(name.clone(), v);
        return Ok(Signal::None);
    }

    unreachable!("Assign tag assigned to non-assign statement")
}

fn exec_print(
    stmt: &Stmt,
    env: &mut Env,
    registry: &EvalRegistry,
    _in_loop: bool,
) -> Result<Signal, String> {
    if let Stmt::Print { expr } = stmt {
        let v = eval_expr(expr, env, registry)?;
        println!("{}", display(&v));
        return Ok(Signal::None);
    }

    unreachable!("Print tag assigned to non-print statement")
}

fn exec_if(
    stmt: &Stmt,
    env: &mut Env,
    registry: &EvalRegistry,
    in_loop: bool,
) -> Result<Signal, String> {
    if let Stmt::If {
        cond,
        then_block,
        else_block,
    } = stmt
    {
        let c = eval_expr(cond, env, registry)?;
        if truthy(&c) {
            return exec_block(then_block, env, registry, in_loop);
        } else if let Some(block) = else_block {
            return exec_block(block, env, registry, in_loop);
        }
        return Ok(Signal::None);
    }

    unreachable!("If tag assigned to non-if statement")
}

fn exec_while(
    stmt: &Stmt,
    env: &mut Env,
    registry: &EvalRegistry,
    _in_loop: bool,
) -> Result<Signal, String> {
    if let Stmt::While { cond, body } = stmt {
        loop {
            let c = eval_expr(cond, env, registry)?;
            if !truthy(&c) {
                break;
            }

            match exec_block(body, env, registry, true)? {
                Signal::None => {}
                Signal::Continue => continue,
                Signal::Break => break,
            }
        }
        return Ok(Signal::None);
    }

    unreachable!("While tag assigned to non-while statement")
}

fn exec_break(
    stmt: &Stmt,
    _env: &mut Env,
    _registry: &EvalRegistry,
    in_loop: bool,
) -> Result<Signal, String> {
    if matches!(stmt, Stmt::Break) {
        if !in_loop {
            return Err("SyntaxError: 'break' outside of while".into());
        }
        return Ok(Signal::Break);
    }

    unreachable!("Break tag assigned to non-break statement")
}

fn exec_continue(
    stmt: &Stmt,
    _env: &mut Env,
    _registry: &EvalRegistry,
    in_loop: bool,
) -> Result<Signal, String> {
    if matches!(stmt, Stmt::Continue) {
        if !in_loop {
            return Err("SyntaxError: 'continue' outside of while".into());
        }
        return Ok(Signal::Continue);
    }

    unreachable!("Continue tag assigned to non-continue statement")
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
