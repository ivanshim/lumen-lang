use crate::ast::*;
use std::str::Lines;

pub struct Parser<'a> {
    lines: Lines<'a>,
}

impl<'a> Parser<'a> {
    pub fn new(src: &'a str) -> Self {
        Self { lines: src.lines() }
    }

    pub fn parse(&mut self) -> Vec<Stmt> {
        let mut stmts = Vec::new();

        while let Some(line) = self.lines.next() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if line.starts_with("while ") {
                let cond = self.parse_condition(&line[6..line.len() - 1]);
                let mut body = Vec::new();

                while let Some(next) = self.lines.next() {
                    if !next.starts_with("    ") {
                        break;
                    }
                    body.push(self.parse_line(next.trim()));
                }

                stmts.push(Stmt::While { cond, body });
            } else {
                stmts.push(self.parse_line(line));
            }
        }

        stmts
    }

    fn parse_line(&self, line: &str) -> Stmt {
        if line.starts_with("print(") {
            let inner = &line[6..line.len() - 1];
            Stmt::Print {
                expr: self.parse_expr(inner),
            }
        } else if line.contains('=') {
            let parts: Vec<_> = line.split('=').map(str::trim).collect();
            Stmt::Assign {
                name: parts[0].to_string(),
                value: self.parse_expr(parts[1]),
            }
        } else {
            panic!("Unknown statement: {}", line);
        }
    }

    fn parse_expr(&self, src: &str) -> Expr {
        if let Ok(n) = src.parse::<f64>() {
            Expr::Number(n)
        } else if src.contains('+') {
            let parts: Vec<_> = src.split('+').map(str::trim).collect();
            Expr::Binary {
                left: Box::new(self.parse_expr(parts[0])),
                op: BinOp::Add,
                right: Box::new(self.parse_expr(parts[1])),
            }
        } else if src.contains('-') {
            let parts: Vec<_> = src.split('-').map(str::trim).collect();
            Expr::Binary {
                left: Box::new(self.parse_expr(parts[0])),
                op: BinOp::Sub,
                right: Box::new(self.parse_expr(parts[1])),
            }
        } else {
            Expr::Var(src.to_string())
        }
    }

    fn parse_condition(&self, src: &str) -> Expr {
        if src.contains('<') {
            let parts: Vec<_> = src.split('<').map(str::trim).collect();
            Expr::Compare {
                left: Box::new(self.parse_expr(parts[0])),
                op: CmpOp::Lt,
                right: Box::new(self.parse_expr(parts[1])),
            }
        } else if src.contains('>') {
            let parts: Vec<_> = src.split('>').map(str::trim).collect();
            Expr::Compare {
                left: Box::new(self.parse_expr(parts[0])),
                op: CmpOp::Gt,
                right: Box::new(self.parse_expr(parts[1])),
            }
        } else {
            panic!("Invalid condition: {}", src);
        }
    }
}
