use std::collections::HashMap;
use crate::ast::{Expr, Stmt, BinOp, CmpOp};

pub fn eval_program(stmts: &[Stmt]) {
    let mut env: HashMap<String, f64> = HashMap::new();
    eval_block(stmts, &mut env);
}

fn eval_block(stmts: &[Stmt], env: &mut HashMap<String, f64>) {
    for stmt in stmts {
        match stmt {
            Stmt::Assign(name, expr) => {
                let val = eval_expr(expr, env);
                env.insert(name.clone(), val);
            }
            Stmt::Print(expr) => {
                let val = eval_expr(expr, env);
                println!("{}", val);
            }
            Stmt::While(cond, body) => {
                while eval_cond(cond, env) {
                    eval_block(body, env);
                }
            }
            Stmt::If(cond, then_block, else_block) => {
                if eval_cond(cond, env) {
                    eval_block(then_block, env);
                } else {
                    eval_block(else_block, env);
                }
            }
        }
    }
}

fn eval_cond(expr: &Expr, env: &mut HashMap<String, f64>) -> bool {
    match expr {
        Expr::Compare(left, op, right) => {
            let l = eval_expr(left, env);
            let r = eval_expr(right, env);
            match op {
                CmpOp::Eq => l == r,
                CmpOp::Lt => l < r,
                CmpOp::Gt => l > r,
            }
        }
        _ => eval_expr(expr, env) != 0.0,
    }
}

fn eval_expr(expr: &Expr, env: &mut HashMap<String, f64>) -> f64 {
    match expr {
        Expr::Number(n) => *n,
        Expr::Var(name) => *env.get(name).unwrap_or(&0.0),
        Expr::BinOp(left, op, right) => {
            let l = eval_expr(left, env);
            let r = eval_expr(right, env);
            match op {
                BinOp::Add => l + r,
                BinOp::Sub => l - r,
            }
        }
        Expr::Compare(left, op, right) => {
            if eval_cond(expr, env) { 1.0 } else { 0.0 }
        }
    }
}
