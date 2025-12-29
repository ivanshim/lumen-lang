use crate::ast::{Expr, Stmt, BinOp, CmpOp};

pub fn parse_program(src: &str) -> Vec<Stmt> {
    let mut lines: Vec<&str> = src.lines().collect();
    parse_block(&mut lines, 0)
}

fn parse_block(lines: &mut Vec<&str>, indent: usize) -> Vec<Stmt> {
    let mut stmts = Vec::new();
    while !lines.is_empty() {
        let line = lines[0];
        let trimmed = line.trim_start();
        let current_indent = line.len() - trimmed.len();
        if current_indent < indent {
            break;
        }
        if current_indent > indent {
            // skip deeper indentation (should be handled by recursion)
        }
        // consume the line
        lines.remove(0);
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with("while ") {
            // parse while condition followed by colon
            let rest = trimmed.trim_start_matches("while ").trim();
            if let Some((cond_str, _)) = rest.split_once(':') {
                let body = parse_block(lines, indent + 4);
                stmts.push(Stmt::While(parse_cond(cond_str), body));
            }
        } else if trimmed.starts_with("if ") {
            let rest = trimmed.trim_start_matches("if ").trim();
            if let Some((cond_str, _)) = rest.split_once(':') {
                let then_block = parse_block(lines, indent + 4);
                // check for else
                let mut else_block = Vec::new();
                if !lines.is_empty() {
                    let next_line = lines[0];
                    let next_trimmed = next_line.trim_start();
                    let next_indent = next_line.len() - next_trimmed.len();
                    if next_indent == indent && next_trimmed.starts_with("else:") {
                        // consume else line
                        lines.remove(0);
                        else_block = parse_block(lines, indent + 4);
                    }
                }
                stmts.push(Stmt::If(parse_cond(cond_str), then_block, else_block));
            }
        } else if trimmed.starts_with("print(") && trimmed.ends_with(')') {
            let inner = &trimmed["print(".len()..trimmed.len() - 1];
            stmts.push(Stmt::Print(parse_expr(inner.trim())));
        } else if let Some((var, expr)) = trimmed.split_once('=') {
            let var = var.trim();
            let expr = parse_expr(expr.trim());
            stmts.push(Stmt::Assign(var.to_string(), expr));
        }
    }
    stmts
}

fn parse_cond(s: &str) -> Expr {
    // handle ==, <, >
    if let Some((left, right)) = s.split_once("==") {
        return Expr::Compare(
            Box::new(parse_expr(left.trim())),
            CmpOp::Eq,
            Box::new(parse_expr(right.trim())),
        );
    }
    if let Some((left, right)) = s.split_once("<") {
        return Expr::Compare(
            Box::new(parse_expr(left.trim())),
            CmpOp::Lt,
            Box::new(parse_expr(right.trim())),
        );
    }
    if let Some((left, right)) = s.split_once(">") {
        return Expr::Compare(
            Box::new(parse_expr(left.trim())),
            CmpOp::Gt,
            Box::new(parse_expr(right.trim())),
        );
    }
    parse_expr(s.trim())
}

fn parse_expr(s: &str) -> Expr {
    // parse addition or subtraction, right-associative
    // search for last '+' or '-' not within parentheses (not supported)
    if let Some(pos) = s.rfind('+') {
        let (left, right) = s.split_at(pos);
        return Expr::BinOp(
            Box::new(parse_expr(left.trim())),
            BinOp::Add,
            Box::new(parse_expr(right[1..].trim())),
        );
    }
    if let Some(pos) = s.rfind('-') {
        let (left, right) = s.split_at(pos);
        return Expr::BinOp(
            Box::new(parse_expr(left.trim())),
            BinOp::Sub,
            Box::new(parse_expr(right[1..].trim())),
        );
    }
    // numeric
    if let Ok(num) = s.parse::<f64>() {
        return Expr::Number(num);
    }
    // variable
    Expr::Var(s.trim().to_string())
}
