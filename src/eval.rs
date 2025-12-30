use crate::ast::*;
use std::collections::HashMap;

pub struct Env {
    vars: HashMap<String, f64>,
}

impl Env {
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
        }
    }
}

pub fn eval(stmts: &[Stmt], env: &mut Env) {
    for stmt in stmts {
        eval_stmt(stmt, env);
    }
}

fn eval_stmt(stmt: &Stmt, env: &mut Env) {
    match stmt {
        Stmt::Assign { name, value } => {
            let v = eval_expr(value, env);
            env.vars.insert(name.clone(), v);
        }
        Stmt::Print { expr } => {
            let v = eval_expr(expr, env);
            println!("{}", v);
        }
        Stmt::While { cond, body } => {
            while eval_cond(cond, env) {
                for stmt in body {
                    eval_stmt(stmt, env);
                }
            }
        }
        Stmt::If { .. } => {
            unimplemented!("if/else not in v0.0.1")
        }
    }
}

fn eval_expr(expr: &Expr, env: &Env) -> f64 {
    match expr {
        Expr::Number(n) => *n,
        Expr::Var(name) => *env.vars.get(name).unwrap_or(&0.0),
        Expr::Binary { left, op, right } => {
            let l = eval_expr(left, env);
            let r = eval_expr(right, env);
            match op {
                BinOp::Add => l + r,
                BinOp::Sub => l - r,
            }
        }
        Expr::Compare { .. } => {
            panic!("Comparison used as value")
        }
    }
}

fn eval_cond(expr: &Expr, env: &Env) -> bool {
    match expr {
        Expr::Compare { left, op, right } => {
            let l = eval_expr(left, env);
            let r = eval_expr(right, env);
            match op {
                CmpOp::Eq => l == r,
                CmpOp::Ne => l != r,
                CmpOp::Lt => l < r,
                CmpOp::Gt => l > r,
            }
        }
        _ => panic!("Invalid condition"),
    }
}
