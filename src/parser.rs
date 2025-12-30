use crate::ast::*;
use std::iter::Peekable;
use std::str::Lines;

pub fn parse(source: &str) -> Vec<Stmt> {
    let mut lines = source.lines().peekable();
    parse_block(&mut lines)
}

fn parse_block(lines: &mut Peekable<Lines>) -> Vec<Stmt> {
    let mut stmts = Vec::new();

    while let Some(&line) = lines.peek() {
        if line.trim().is_empty() {
            lines.next();
            continue;
        }

        if !line.starts_with("    ") && !stmts.is_empty() {
            break;
        }

        let line = lines.next().unwrap().trim_start();
        stmts.push(parse_stmt(line, lines));
    }

    stmts
}

fn parse_stmt(line: &str, lines: &mut Peekable<Lines>) -> Stmt {
    if line.starts_with("while ") {
        let cond = parse_expr(line.strip_prefix("while ").unwrap().trim_end_matches(':'));
        let body = parse_block(lines);
        Stmt::While { cond, body }
    } else if line.starts_with("if ") {
        let cond = parse_expr(line.strip_prefix("if ").unwrap().trim_end_matches(':'));
        let then_block = parse_block(lines);

        let else_block = if let Some(&next) = lines.peek() {
            if next.trim_start().starts_with("else:") {
                lines.next();
                Some(parse_block(lines))
            } else {
                None
            }
        } else {
            None
        };

        Stmt::If {
            cond,
            then_block,
            else_block,
        }
    } else if line.starts_with("print(") {
        let expr = line
            .strip_prefix("print(")
            .unwrap()
            .strip_suffix(")")
            .unwrap();
        Stmt::Print {
            expr: parse_expr(expr),
        }
    } else if line.contains('=') {
        let (name, expr) = line.split_once('=').unwrap();
        Stmt::Assign {
            name: name.trim().to_string(),
            value: parse_expr(expr.trim()),
        }
    } else {
        panic!("Unknown statement: {}", line);
    }
}

fn parse_expr(input: &str) -> Expr {
    if let Some((l, r)) = input.split_once("==") {
        Expr::Compare(
            Box::new(parse_expr(l.trim())),
            CmpOp::Eq,
            Box::new(parse_expr(r.trim())),
        )
    } else if let Some((l, r)) = input.split_once('<') {
        Expr::Compare(
            Box::new(parse_expr(l.trim())),
            CmpOp::Lt,
            Box::new(parse_expr(r.trim())),
        )
    } else if let Some((l, r)) = input.split_once('>') {
        Expr::Compare(
            Box::new(parse_expr(l.trim())),
            CmpOp::Gt,
            Box::new(parse_expr(r.trim())),
        )
    } else if let Some((l, r)) = input.split_once('+') {
        Expr::Binary(
            Box::new(parse_expr(l.trim())),
            BinOp::Add,
            Box::new(parse_expr(r.trim())),
        )
    } else if let Some((l, r)) = input.split_once('-') {
        Expr::Binary(
            Box::new(parse_expr(l.trim())),
            BinOp::Sub,
            Box::new(parse_expr(r.trim())),
        )
    } else if let Ok(n) = input.parse::<f64>() {
        Expr::Number(n)
    } else {
        Expr::Var(input.to_string())
    }
}
