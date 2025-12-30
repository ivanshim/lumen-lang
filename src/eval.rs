use crate::ast::*;
use std::collections::HashMap;

pub fn eval(program: &[Stmt]) {
    let mut env = HashMap::new();
    eval_block(program, &mut env);
}

fn eval_block(block: &[Stmt], env: &mut HashMap<String, f64>) {
    for stmt in block {
        eval_stmt(stmt, env);
    }
}

fn eval_stmt(stmt: &Stmt, env: &mut HashMap<String, f64>) {
    match stmt {
        Stmt::Assign { name, value } => {
            let v = eval_expr(value, env);
            env.insert(name.clone(), v);
        }

        Stmt::Print { expr } => {
            let v = eval_expr(expr, env);
            println!("{}", v as i64);
        }

        Stmt::While { cond, body } => {
            while eval_cond(cond, env) {
                eval_block(body, env);
            }
        }

        Stmt::If {
            cond,
            then_block,
            else_block,
        } => {
            if eval_cond(cond, env) {
                eval_block(then_block, env);
            } else if let Some(else_block) = else_block {
                eval_block(else_block, env);
            }
        }
    }
}

fn eval_cond(expr: &Expr, env: &HashMap<String, f64>) -> bool {
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

fn eval_expr(expr: &Expr, env: &HashMap<String, f64>) -> f64 {
    match expr {
        Expr::Number(n) => *n,
        Expr::Var(name) => *env.get(name).expect("Undefined variable"),
        Expr::Binary(left, op, right) => {
            let l = eval_expr(left, env);
            let r = eval_expr(right, env);
            match op {
                BinOp::Add => l + r,
                BinOp::Sub => l - r,
            }
        }
        Expr::Compare(_, _, _) => {
            if eval_cond(expr, env) {
                1.0
            } else {
                0.0
            }
        }
    }
}
