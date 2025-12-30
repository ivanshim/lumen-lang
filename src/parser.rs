use crate::ast::*;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    // structure
    Newline,
    Indent,
    Dedent,
    Eof,

    // literals / identifiers
    Ident(String),
    Number(f64),
    Str(String),

    // punctuation
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Colon,

    // operators
    Assign,      // =
    Plus,        // +
    Minus,       // -
    Star,        // *
    Slash,       // /
    EqEq,        // ==
    NotEq,       // !=
    Lt,          // <
    Le,          // <=
    Gt,          // >
    Ge,          // >=
    DotDot,      // ..
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
    // Returns (token, line, col)
    let mut out: Vec<(Tok, usize, usize)> = Vec::new();

    let mut indent_stack: Vec<usize> = vec![0];
    let mut line_no = 0;

    for raw_line in src.lines() {
        line_no += 1;
        let mut col = 1;

        // Strip trailing whitespace
        let line = raw_line.trim_end_matches(|c: char| c == ' ' || c == '\t');

        // Skip blank lines
        if line.trim().is_empty() {
            continue;
        }

        // Count leading spaces
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

        // Lex the non-indented slice
        let mut i = leading;
        let chars: Vec<char> = raw_line.chars().collect();

        // helper closures
        let mut push = |t: Tok, c: usize| out.push((t, line_no, c));

        while i < chars.len() {
            let ch = chars[i];
            col = i + 1;

            // whitespace inside line
            if ch == ' ' || ch == '\t' {
                i += 1;
                continue;
            }

            // comment
            if ch == '#' {
                break;
            }

            // strings
            if ch == '"' {
                i += 1;
                let start_col = col;
                let mut s = String::new();
                while i < chars.len() && chars[i] != '"' {
                    s.push(chars[i]);
                    i += 1;
                }
                if i >= chars.len() || chars[i] != '"' {
                    return Err(ParseError {
                        msg: "Unterminated string literal".into(),
                        line: line_no,
                        col: start_col,
                    });
                }
                i += 1;
                push(Tok::Str(s), start_col);
                continue;
            }

            // numbers
            if ch.is_ascii_digit() {
                let start = i;
                let start_col = col;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
                let n = s.parse::<f64>().map_err(|_| ParseError {
                    msg: format!("Invalid number: {s}"),
                    line: line_no,
                    col: start_col,
                })?;
                push(Tok::Number(n), start_col);
                continue;
            }

            // identifiers / keywords
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

            // two-char operators
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
                    ('.', '.') => {
                        push(Tok::DotDot, col);
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
                '[' => push(Tok::LBracket, col),
                ']' => push(Tok::RBracket, col),
                ',' => push(Tok::Comma, col),
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

    // close indentation
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
        // keyword dispatch by Ident(...)
        if let Some(word) = self.peek_ident() {
            match word.as_str() {
                "print" => return self.parse_print(),
                "if" => return self.parse_if(),
                "while" => return self.parse_while(),
                "for" => return self.parse_for(),
                "fn" => return self.parse_fn(),
                "return" => return self.parse_return(),
                _ => {}
            }
        }

        // assignment: name '=' expr
        if let (Tok::Ident(name), _, _) = self.peek().clone() {
            if self.peek_n(1).map(|t| &t.0) == Some(&Tok::Assign) {
                self.advance(); // name
                self.advance(); // =
                let value = self.parse_expr()?;
                self.consume_newline();
                return Ok(Stmt::Assign { name, value });
            }
        }

        // expression statement
        let expr = self.parse_expr()?;
        self.consume_newline();
        Ok(Stmt::ExprStmt(expr))
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

        let mut branches = vec![(cond, then_block)];
        let mut else_block: Option<Vec<Stmt>> = None;

        while let Some(word) = self.peek_ident() {
            if word == "elif" {
                self.advance();
                let c = self.parse_expr()?;
                self.expect(Tok::Colon, "Expected ':' after elif condition")?;
                let b = self.parse_block()?;
                branches.push((c, b));
            } else {
                break;
            }
        }

        if let Some(word) = self.peek_ident() {
            if word == "else" {
                self.advance();
                self.expect(Tok::Colon, "Expected ':' after else")?;
                else_block = Some(self.parse_block()?);
            }
        }

        Ok(Stmt::If { branches, else_block })
    }

    fn parse_while(&mut self) -> Result<Stmt, ParseError> {
        self.expect_ident("while")?;
        let cond = self.parse_expr()?;
        self.expect(Tok::Colon, "Expected ':' after while condition")?;
        let body = self.parse_block()?;
        Ok(Stmt::While { cond, body })
    }

    fn parse_for(&mut self) -> Result<Stmt, ParseError> {
        self.expect_ident("for")?;
        let name = self.expect_any_ident("Expected loop variable after 'for'")?;
        self.expect_ident("in")?;

        // range: expr '..' expr
        let start = self.parse_expr()?;
        self.expect(Tok::DotDot, "Expected '..' in range")?;
        let end = self.parse_expr()?;

        self.expect(Tok::Colon, "Expected ':' after for range")?;
        let body = self.parse_block()?;
        Ok(Stmt::ForRange { name, start, end, body })
    }

    fn parse_fn(&mut self) -> Result<Stmt, ParseError> {
        self.expect_ident("fn")?;
        let name = self.expect_any_ident("Expected function name")?;
        self.expect(Tok::LParen, "Expected '(' after function name")?;

        let mut params = Vec::new();
        if !self.check(&Tok::RParen) {
            loop {
                let p = self.expect_any_ident("Expected parameter name")?;
                params.push(p);
                if self.check(&Tok::Comma) {
                    self.advance();
                    continue;
                }
                break;
            }
        }

        self.expect(Tok::RParen, "Expected ')' after parameter list")?;
        self.expect(Tok::Colon, "Expected ':' after function header")?;
        let body = self.parse_block()?;
        Ok(Stmt::FnDef { name, params, body })
    }

    fn parse_return(&mut self) -> Result<Stmt, ParseError> {
        self.expect_ident("return")?;
        if self.check(&Tok::Newline) {
            self.advance();
            return Ok(Stmt::Return(None));
        }
        if self.check(&Tok::Dedent) || self.check(&Tok::Eof) {
            return Ok(Stmt::Return(None));
        }
        let expr = self.parse_expr()?;
        self.consume_newline();
        Ok(Stmt::Return(Some(expr)))
    }

    // ---------- Expressions (precedence climbing) ----------

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
        let mut expr = self.parse_cmp()?;
        while self.peek_ident_is("and") {
            self.advance();
            let right = self.parse_cmp()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinOp::And,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_cmp(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_add()?;
        loop {
            let op = match self.peek().0 {
                Tok::EqEq => Some(CmpOp::Eq),
                Tok::NotEq => Some(CmpOp::Ne),
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
        if self.peek().0 == Tok::Minus {
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
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;
        loop {
            // call
            if self.check(&Tok::LParen) {
                self.advance();
                let mut args = Vec::new();
                if !self.check(&Tok::RParen) {
                    loop {
                        args.push(self.parse_expr()?);
                        if self.check(&Tok::Comma) {
                            self.advance();
                            continue;
                        }
                        break;
                    }
                }
                self.expect(Tok::RParen, "Expected ')' after arguments")?;
                expr = Expr::Call {
                    callee: Box::new(expr),
                    args,
                };
                continue;
            }

            // index
            if self.check(&Tok::LBracket) {
                self.advance();
                let idx = self.parse_expr()?;
                self.expect(Tok::RBracket, "Expected ']'")?;
                expr = Expr::Index {
                    base: Box::new(expr),
                    index: Box::new(idx),
                };
                continue;
            }

            break;
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let (tok, line, col) = self.peek().clone();

        match tok {
            Tok::Number(n) => {
                self.advance();
                Ok(Expr::Literal(Value::Number(n)))
            }
            Tok::Str(s) => {
                self.advance();
                Ok(Expr::Literal(Value::Str(s)))
            }
            Tok::Ident(name) => {
                // keywords as literals
                if name == "true" {
                    self.advance();
                    return Ok(Expr::Literal(Value::Bool(true)));
                }
                if name == "false" {
                    self.advance();
                    return Ok(Expr::Literal(Value::Bool(false)));
                }
                if name == "None" {
                    self.advance();
                    return Ok(Expr::Literal(Value::None));
                }

                self.advance();
                Ok(Expr::Var(name))
            }
            Tok::LParen => {
                self.advance();
                let e = self.parse_expr()?;
                self.expect(Tok::RParen, "Expected ')'")?;
                Ok(e)
            }
            Tok::LBracket => {
                self.advance();
                let mut items = Vec::new();
                if !self.check(&Tok::RBracket) {
                    loop {
                        items.push(self.parse_expr()?);
                        if self.check(&Tok::Comma) {
                            self.advance();
                            continue;
                        }
                        break;
                    }
                }
                self.expect(Tok::RBracket, "Expected ']' after list")?;
                Ok(Expr::List(items))
            }
            _ => Err(ParseError {
                msg: format!("Unexpected token: {:?}.", tok),
                line,
                col,
            }),
        }
    }

    // ---------- helpers ----------

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

    fn peek_ident(&self) -> Option<String> {
        if let Tok::Ident(s) = &self.peek().0 {
            Some(s.clone())
        } else {
            None
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

    fn expect_any_ident(&mut self, msg: &str) -> Result<String, ParseError> {
        if let Tok::Ident(s) = &self.peek().0 {
            let s = s.clone();
            self.advance();
            Ok(s)
        } else {
            let (_, line, col) = self.peek().clone();
            Err(ParseError {
                msg: msg.into(),
                line,
                col,
            })
        }
    }
}
