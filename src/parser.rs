use crate::ast::*;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Newline,
    Indent,
    Dedent,
    Eof,

    Ident(String),
    Int(i64),

    LParen,
    RParen,
    Colon,

    Assign, // =
    Plus,
    Minus,
    Star,
    Slash,

    EqEq,  // ==
    NotEq, // !=
    Lt,    // <
    Le,    // <=
    Gt,    // >
    Ge,    // >=
}

#[derive(Debug)]
pub struct ParseError {
    msg: String,
    line: usize,
    col: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SyntaxError at {}:{}: {}", self.line, self.col, self.msg)
    }
}

pub fn parse(src: &str) -> Result<Vec<Stmt>, ParseError> {
    let toks = lex(src)?;
    let mut p = Parser::new(toks);
    p.parse_program()
}

fn lex(src: &str) -> Result<Vec<(Tok, usize, usize)>, ParseError> {
    let mut out: Vec<(Tok, usize, usize)> = Vec::new();
    let mut indent_stack: Vec<usize> = vec![0];

    let mut line_no = 0;

    for raw_line in src.lines() {
        line_no += 1;

        // strip trailing whitespace
        let line_trimmed = raw_line.trim_end_matches(|c: char| c == ' ' || c == '\t');

        // skip blank lines
        if line_trimmed.trim().is_empty() {
            continue;
        }

        // count leading spaces
        let leading = raw_line.chars().take_while(|c| *c == ' ').count();
        let current = *indent_stack.last().unwrap();

        if leading > current {
            indent_stack.push(leading);
            out.push((Tok::Indent, line_no, 1));
        } else if leading < current {
            while leading < *indent_stack.last().unwrap() {
                indent_stack.pop();
                out.push((Tok::Dedent, line_no, 1));
            }
            if leading != *indent_stack.last().unwrap() {
                return Err(ParseError {
                    msg: "Inconsistent indentation".into(),
                    line: line_no,
                    col: 1,
                });
            }
        }

        let chars: Vec<char> = raw_line.chars().collect();
        let mut i = leading;

        let mut push = |t: Tok, col: usize| out.push((t, line_no, col));

        while i < chars.len() {
            let ch = chars[i];
            let col = i + 1;

            if ch == ' ' || ch == '\t' {
                i += 1;
                continue;
            }

            // comment
            if ch == '#' {
                break;
            }

            // integer
            if ch.is_ascii_digit() {
                let start = i;
                let start_col = col;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
                let n = s.parse::<i64>().map_err(|_| ParseError {
                    msg: format!("Invalid integer: {s}"),
                    line: line_no,
                    col: start_col,
                })?;
                push(Tok::Int(n), start_col);
                continue;
            }

            // identifier / keyword
            if ch.is_ascii_alphabetic() || ch == '_' {
                let start = i;
                let start_col = col;
                i += 1;
                while i < chars.len()
                    && (chars[i].is_ascii_alphanumeric() || chars[i] == '_')
                {
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
                push(Tok::Ident(s), start_col);
                continue;
            }

            // two-char ops
            if i + 1 < chars.len() {
                let two = (chars[i], chars[i + 1]);
                match two {
                    ('=', '=') => {
                        push(Tok::EqEq, col);
                        i += 2;
                        continue;
                    }
                    ('!', '=') => {
                        push(Tok::NotEq, col);
                        i += 2;
                        continue;
                    }
                    ('<', '=') => {
                        push(Tok::Le, col);
                        i += 2;
                        continue;
                    }
                    ('>', '=') => {
                        push(Tok::Ge, col);
                        i += 2;
                        continue;
                    }
                    _ => {}
                }
            }

            // single char
            match ch {
                '(' => push(Tok::LParen, col),
                ')' => push(Tok::RParen, col),
                ':' => push(Tok::Colon, col),
                '=' => push(Tok::Assign, col),
                '+' => push(Tok::Plus, col),
                '-' => push(Tok::Minus, col),
                '*' => push(Tok::Star, col),
                '/' => push(Tok::Slash, col),
                '<' => push(Tok::Lt, col),
                '>' => push(Tok::Gt, col),
                _ => {
                    return Err(ParseError {
                        msg: format!("Unexpected character: {ch}"),
                        line: line_no,
                        col,
                    })
                }
            }

            i += 1;
        }

        out.push((Tok::Newline, line_no, raw_line.len().max(1)));
    }

    while indent_stack.len() > 1 {
        indent_stack.pop();
        out.push((Tok::Dedent, line_no.max(1), 1));
    }

    out.push((Tok::Eof, line_no.max(1), 1));
    Ok(out)
}

struct Parser {
    toks: Vec<(Tok, usize, usize)>,
    pos: usize,
}

impl Parser {
    fn new(toks: Vec<(Tok, usize, usize)>) -> Self {
        Self { toks, pos: 0 }
    }

    fn parse_program(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let mut out = Vec::new();
        while !self.check(&Tok::Eof) {
            if self.check(&Tok::Newline) {
                self.advance();
                continue;
            }
            out.push(self.parse_stmt()?);
        }
        Ok(out)
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, ParseError> {
        self.expect(Tok::Newline, "Expected newline after ':'")?;
        self.expect(Tok::Indent, "Expected indented block")?;

        let mut body = Vec::new();
        while !self.check(&Tok::Dedent) && !self.check(&Tok::Eof) {
            if self.check(&Tok::Newline) {
                self.advance();
                continue;
            }
            body.push(self.parse_stmt()?);
        }

        self.expect(Tok::Dedent, "Expected end of block")?;
        Ok(body)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, ParseError> {
        if self.peek_ident_is("print") {
            return self.parse_print();
        }
        if self.peek_ident_is("if") {
            return self.parse_if();
        }
        if self.peek_ident_is("while") {
            return self.parse_while();
        }
        if self.peek_ident_is("break") {
            self.advance();
            self.consume_newline();
            return Ok(Stmt::Break);
        }
        if self.peek_ident_is("continue") {
            self.advance();
            self.consume_newline();
            return Ok(Stmt::Continue);
        }

        // assignment: IDENT '=' Expr
        if let (Tok::Ident(name), _, _) = self.peek().clone() {
            if self.peek_n(1).map(|t| &t.0) == Some(&Tok::Assign) {
                self.advance(); // name
                self.advance(); // =
                let value = self.parse_expr()?;
                self.consume_newline();
                return Ok(Stmt::Assign { name, value });
            }
        }

        // no expression statements in v0.1 (keeps language honest & small)
        let (_, line, col) = self.peek().clone();
        Err(ParseError {
            msg: "Expected statement (assignment, print, if, while, break, continue)".into(),
            line,
            col,
        })
    }

    fn parse_print(&mut self) -> Result<Stmt, ParseError> {
        self.expect_ident("print")?;
        self.expect(Tok::LParen, "Expected '(' after print")?;
        let expr = self.parse_expr()?;
        self.expect(Tok::RParen, "Expected ')' after print argument")?;
        self.consume_newline();
        Ok(Stmt::Print { expr })
    }

    fn parse_if(&mut self) -> Result<Stmt, ParseError> {
        self.expect_ident("if")?;
        let cond = self.parse_expr()?;
        self.expect(Tok::Colon, "Expected ':' after if condition")?;
        let then_block = self.parse_block()?;

        let else_block = if self.peek_ident_is("else") {
            self.advance();
            self.expect(Tok::Colon, "Expected ':' after else")?;
            Some(self.parse_block()?)
        } else {
            None
        };

        Ok(Stmt::If {
            cond,
            then_block,
            else_block,
        })
    }

    fn parse_while(&mut self) -> Result<Stmt, ParseError> {
        self.expect_ident("while")?;
        let cond = self.parse_expr()?;
        self.expect(Tok::Colon, "Expected ':' after while condition")?;
        let body = self.parse_block()?;
        Ok(Stmt::While { cond, body })
    }

    // ---------------- expressions (precedence) ----------------

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_and()?;
        while self.peek_ident_is("or") {
            self.advance();
            let right = self.parse_and()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinOp::Or,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_eq()?;
        while self.peek_ident_is("and") {
            self.advance();
            let right = self.parse_eq()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinOp::And,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_eq(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_cmp()?;
        loop {
            let op = match self.peek().0 {
                Tok::EqEq => Some(CmpOp::Eq),
                Tok::NotEq => Some(CmpOp::Ne),
                _ => None,
            };
            if let Some(op) = op {
                self.advance();
                let right = self.parse_cmp()?;
                expr = Expr::Compare {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_cmp(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_add()?;
        loop {
            let op = match self.peek().0 {
                Tok::Lt => Some(CmpOp::Lt),
                Tok::Le => Some(CmpOp::Le),
                Tok::Gt => Some(CmpOp::Gt),
                Tok::Ge => Some(CmpOp::Ge),
                _ => None,
            };
            if let Some(op) = op {
                self.advance();
                let right = self.parse_add()?;
                expr = Expr::Compare {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_add(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_mul()?;
        loop {
            let op = match self.peek().0 {
                Tok::Plus => Some(BinOp::Add),
                Tok::Minus => Some(BinOp::Sub),
                _ => None,
            };
            if let Some(op) = op {
                self.advance();
                let right = self.parse_mul()?;
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_mul(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_unary()?;
        loop {
            let op = match self.peek().0 {
                Tok::Star => Some(BinOp::Mul),
                Tok::Slash => Some(BinOp::Div),
                _ => None,
            };
            if let Some(op) = op {
                self.advance();
                let right = self.parse_unary()?;
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        if self.check(&Tok::Minus) {
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnOp::Neg,
                expr: Box::new(expr),
            });
        }
        if self.peek_ident_is("not") {
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnOp::Not,
                expr: Box::new(expr),
            });
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let (tok, line, col) = self.peek().clone();
        match tok {
            Tok::Int(n) => {
                self.advance();
                Ok(Expr::Literal(Value::Int(n)))
            }
            Tok::Ident(s) => {
                // bool literals
                if s == "true" {
                    self.advance();
                    return Ok(Expr::Literal(Value::Bool(true)));
                }
                if s == "false" {
                    self.advance();
                    return Ok(Expr::Literal(Value::Bool(false)));
                }

                self.advance();
                Ok(Expr::Var(s))
            }
            Tok::LParen => {
                self.advance();
                let e = self.parse_expr()?;
                self.expect(Tok::RParen, "Expected ')'")?;
                Ok(e)
            }
            _ => Err(ParseError {
                msg: format!("Unexpected token: {:?}.", tok),
                line,
                col,
            }),
        }
    }

    // ---------------- helpers ----------------

    fn peek(&self) -> &(Tok, usize, usize) {
        &self.toks[self.pos]
    }

    fn peek_n(&self, n: usize) -> Option<&(Tok, usize, usize)> {
        self.toks.get(self.pos + n)
    }

    fn check(&self, t: &Tok) -> bool {
        &self.peek().0 == t
    }

    fn advance(&mut self) -> (Tok, usize, usize) {
        let out = self.toks[self.pos].clone();
        self.pos += 1;
        out
    }

    fn expect(&mut self, t: Tok, msg: &str) -> Result<(), ParseError> {
        if self.peek().0 == t {
            self.advance();
            Ok(())
        } else {
            let (_, line, col) = self.peek().clone();
            Err(ParseError {
                msg: msg.into(),
                line,
                col,
            })
        }
    }

    fn consume_newline(&mut self) {
        if self.check(&Tok::Newline) {
            self.advance();
        }
    }

    fn peek_ident_is(&self, s: &str) -> bool {
        matches!(&self.peek().0, Tok::Ident(x) if x == s)
    }

    fn expect_ident(&mut self, s: &str) -> Result<(), ParseError> {
        if self.peek_ident_is(s) {
            self.advance();
            Ok(())
        } else {
            let (_, line, col) = self.peek().clone();
            Err(ParseError {
                msg: format!("Expected keyword '{s}'"),
                line,
                col,
            })
        }
    }
}
