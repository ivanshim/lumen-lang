// Stage 3: Reduce - Token stream → Instruction tree
//
// Parse tokens into Instruction tree using 7 primitives.
// All semantics come from:
// - Schema (operator precedence, statement patterns)
// - Value types (what data exists)
// - Environment (where data lives)
//
// Parser uses Pratt parsing for expressions + top-down for statements.

use super::ingest::Token;
use super::primitives::Instruction;
use super::eval::Value;
use crate::schema::LanguageSchema;

/// Parser: stateful token consumer
struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
    #[allow(dead_code)]
    schema: &'a LanguageSchema,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token], schema: &'a LanguageSchema) -> Self {
        Parser {
            tokens,
            pos: 0,
            schema,
        }
    }

    fn peek(&self) -> Token {
        self.tokens.get(self.pos).cloned().unwrap_or_else(|| Token {
            lexeme: "EOF".to_string(),
            span: (0, 0),
            line: 0,
            col: 0,
        })
    }

    fn advance(&mut self) -> Token {
        let token = self.peek();
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        token
    }

    fn is_at_end(&self) -> bool {
        self.peek().lexeme == "EOF"
    }

    fn skip_whitespace(&mut self) {
        while self.peek().lexeme == " " || self.peek().lexeme == "\t" || self.peek().lexeme == "\n" {
            self.advance();
        }
    }

    /// Parse a program (sequence of statements)
    fn parse_program(&mut self) -> Result<Instruction, String> {
        let mut stmts = Vec::new();

        while !self.is_at_end() {
            self.skip_whitespace();
            if self.is_at_end() {
                break;
            }

            let stmt = self.parse_statement()?;
            stmts.push(stmt);
            self.skip_whitespace();
        }

        Ok(Instruction::sequence(stmts))
    }

    /// Parse a statement
    fn parse_statement(&mut self) -> Result<Instruction, String> {
        let keyword = &self.peek().lexeme.clone();

        match keyword.as_str() {
            "let" => self.parse_let(),
            "if" => self.parse_if(),
            "while" => self.parse_while(),
            "return" => self.parse_return(),
            "break" => {
                self.advance();
                Ok(Instruction::break_stmt())
            }
            "continue" => {
                self.advance();
                Ok(Instruction::continue_stmt())
            }
            "fn" => self.parse_function_def(),
            _ => self.parse_assignment_or_expression(),
        }
    }

    /// Parse: let name = expr
    fn parse_let(&mut self) -> Result<Instruction, String> {
        self.advance();  // consume 'let'
        self.skip_whitespace();

        let name = self.parse_identifier()?;
        self.skip_whitespace();

        // Skip optional type annotation ": type"
        if self.peek().lexeme == ":" {
            self.advance();
            self.skip_whitespace();
            // Skip the type name
            let _ = self.parse_identifier();
            self.skip_whitespace();
        }

        if self.peek().lexeme != "=" {
            return Err("Expected '=' in let binding".to_string());
        }
        self.advance();
        self.skip_whitespace();

        let value = self.parse_expression()?;
        Ok(Instruction::assign(name, value))
    }

    /// Parse: if condition { block } [else { block }]
    fn parse_if(&mut self) -> Result<Instruction, String> {
        self.advance();  // consume 'if'
        self.skip_whitespace();

        let condition = self.parse_expression()?;
        self.skip_whitespace();

        let then_block = self.parse_block()?;
        self.skip_whitespace();

        let else_block = if self.peek().lexeme == "else" {
            self.advance();
            self.skip_whitespace();
            Some(self.parse_block()?)
        } else {
            None
        };

        Ok(Instruction::branch(condition, then_block, else_block))
    }

    /// Parse: while condition { block }
    fn parse_while(&mut self) -> Result<Instruction, String> {
        self.advance();  // consume 'while'
        self.skip_whitespace();

        let condition = self.parse_expression()?;
        self.skip_whitespace();

        let body = self.parse_block()?;

        Ok(Instruction::loop_stmt(condition, body))
    }

    /// Parse: return [expr]
    fn parse_return(&mut self) -> Result<Instruction, String> {
        self.advance();  // consume 'return'
        self.skip_whitespace();

        if self.peek().lexeme == "\n" || self.is_at_end() || self.peek().lexeme == "}" {
            Ok(Instruction::return_stmt(None))
        } else {
            let expr = self.parse_expression()?;
            Ok(Instruction::return_stmt(Some(expr)))
        }
    }

    /// Parse: fn name(params) { block }
    fn parse_function_def(&mut self) -> Result<Instruction, String> {
        self.advance();  // consume 'fn'
        self.skip_whitespace();

        let name = self.parse_identifier()?;
        self.skip_whitespace();

        if self.peek().lexeme != "(" {
            return Err("Expected '(' after function name".to_string());
        }
        self.advance();
        self.skip_whitespace();

        // Parse parameters
        let mut params = Vec::new();
        while self.peek().lexeme != ")" {
            params.push(self.parse_identifier()?);
            self.skip_whitespace();
            if self.peek().lexeme == "," {
                self.advance();
                self.skip_whitespace();
            }
        }

        self.advance();  // consume ')'
        self.skip_whitespace();

        let body = self.parse_block()?;

        Ok(Instruction::FunctionDef {
            name,
            params,
            body: Box::new(body),
        })
    }

    /// Parse a block: { statements }
    fn parse_block(&mut self) -> Result<Instruction, String> {
        if self.peek().lexeme != "{" {
            return Err("Expected '{'".to_string());
        }
        self.advance();
        self.skip_whitespace();

        let mut stmts = Vec::new();
        while self.peek().lexeme != "}" && !self.is_at_end() {
            let stmt = self.parse_statement()?;
            stmts.push(stmt);
            self.skip_whitespace();
        }

        if self.peek().lexeme != "}" {
            return Err("Expected '}'".to_string());
        }
        self.advance();

        Ok(Instruction::sequence(stmts))
    }

    /// Parse assignment or expression statement
    fn parse_assignment_or_expression(&mut self) -> Result<Instruction, String> {
        let expr = self.parse_expression()?;
        self.skip_whitespace();

        if self.peek().lexeme == "=" {
            self.advance();
            self.skip_whitespace();
            let value = self.parse_expression()?;

            // Extract variable name from expression
            if let Instruction::Variable(name) = expr {
                return Ok(Instruction::assign(name, value));
            } else {
                return Err("Invalid assignment target".to_string());
            }
        }

        Ok(expr)
    }

    /// Parse expression (Pratt parsing for operators)
    fn parse_expression(&mut self) -> Result<Instruction, String> {
        self.parse_comparison()
    }

    /// Parse comparison operators
    fn parse_comparison(&mut self) -> Result<Instruction, String> {
        let mut left = self.parse_additive()?;
        self.skip_whitespace();

        loop {
            let op = match self.peek().lexeme.as_str() {
                "==" | "!=" | "<" | ">" | "<=" | ">=" => self.peek().lexeme.clone(),
                _ => break,
            };
            self.advance();
            self.skip_whitespace();
            let right = self.parse_additive()?;
            self.skip_whitespace();
            left = Instruction::binary(op, left, right);
        }

        Ok(left)
    }

    /// Parse additive operators
    fn parse_additive(&mut self) -> Result<Instruction, String> {
        let mut left = self.parse_multiplicative()?;
        self.skip_whitespace();

        loop {
            let op = match self.peek().lexeme.as_str() {
                "+" | "-" => self.peek().lexeme.clone(),
                _ => break,
            };
            self.advance();
            self.skip_whitespace();
            let right = self.parse_multiplicative()?;
            self.skip_whitespace();
            left = Instruction::binary(op, left, right);
        }

        Ok(left)
    }

    /// Parse multiplicative operators
    fn parse_multiplicative(&mut self) -> Result<Instruction, String> {
        let mut left = self.parse_unary()?;
        self.skip_whitespace();

        loop {
            let op = match self.peek().lexeme.as_str() {
                "*" | "/" | "%" => self.peek().lexeme.clone(),
                _ => break,
            };
            self.advance();
            self.skip_whitespace();
            let right = self.parse_unary()?;
            self.skip_whitespace();
            left = Instruction::binary(op, left, right);
        }

        Ok(left)
    }

    /// Parse unary operators
    fn parse_unary(&mut self) -> Result<Instruction, String> {
        let op = match self.peek().lexeme.as_str() {
            "-" | "not" => self.peek().lexeme.clone(),
            _ => return self.parse_primary(),
        };

        self.advance();
        self.skip_whitespace();
        let operand = self.parse_unary()?;
        Ok(Instruction::unary(op, operand))
    }

    /// Parse primary expression
    fn parse_primary(&mut self) -> Result<Instruction, String> {
        let lexeme = &self.peek().lexeme.clone();

        // Numbers
        if lexeme.chars().next().map_or(false, |c| c.is_ascii_digit()) {
            let num_str = self.consume_number()?;
            let num = num_str.parse::<f64>()
                .map_err(|_| format!("Invalid number: {}", num_str))?;
            return Ok(Instruction::literal(Value::Number(num)));
        }

        // Strings
        if lexeme == "\"" {
            let string_val = self.consume_string()?;
            return Ok(Instruction::literal(Value::String(string_val)));
        }

        // Booleans
        if lexeme == "true" || lexeme == "false" {
            let val = lexeme == "true";
            self.advance();
            return Ok(Instruction::literal(Value::Bool(val)));
        }

        // None
        if lexeme == "none" {
            self.advance();
            return Ok(Instruction::literal(Value::Null));
        }

        // Parenthesized expression
        if lexeme == "(" {
            self.advance();
            self.skip_whitespace();
            let expr = self.parse_expression()?;
            self.skip_whitespace();
            if self.peek().lexeme != ")" {
                return Err("Expected ')'".to_string());
            }
            self.advance();
            return Ok(expr);
        }

        // Identifiers (variables or function calls)
        if lexeme.chars().next().map_or(false, |c| c.is_alphabetic() || c == '_') {
            let name = self.parse_identifier()?;
            self.skip_whitespace();

            if self.peek().lexeme == "(" {
                self.advance();
                self.skip_whitespace();

                let mut args = Vec::new();
                while self.peek().lexeme != ")" {
                    args.push(self.parse_expression()?);
                    self.skip_whitespace();
                    if self.peek().lexeme == "," {
                        self.advance();
                        self.skip_whitespace();
                    }
                }

                self.advance();  // consume ')'
                return Ok(Instruction::invoke(name, args));
            }

            return Ok(Instruction::variable(name));
        }

        Err(format!("Unexpected token: {}", lexeme))
    }

    /// Parse identifier (handling multi-char identifiers from character tokens)
    fn parse_identifier(&mut self) -> Result<String, String> {
        if !self.peek().lexeme.chars().next().map_or(false, |c| c.is_alphabetic() || c == '_') {
            return Err(format!("Expected identifier, got: {}", self.peek().lexeme));
        }

        let mut name = self.peek().lexeme.clone();
        self.advance();

        loop {
            if self.peek().lexeme.len() == 1 {
                let ch = self.peek().lexeme.as_bytes()[0] as char;
                if ch.is_alphanumeric() || ch == '_' {
                    name.push_str(&self.peek().lexeme);
                    self.advance();
                    continue;
                }
            }
            break;
        }

        Ok(name)
    }

    /// Consume a number (handling multi-char numbers)
    fn consume_number(&mut self) -> Result<String, String> {
        let mut num_str = self.peek().lexeme.clone();
        self.advance();

        loop {
            let token = self.peek();
            let ch = token.lexeme.as_str();
            if ch.len() == 1 {
                let b = ch.as_bytes()[0] as char;
                if b.is_ascii_digit() || b == '.' {
                    num_str.push_str(ch);
                    self.advance();
                    continue;
                }
            }
            break;
        }

        Ok(num_str)
    }

    /// Consume a string (handling escape sequences)
    fn consume_string(&mut self) -> Result<String, String> {
        self.advance();  // consume opening quote
        let mut string_val = String::new();

        while self.peek().lexeme != "\"" && !self.is_at_end() {
            if self.peek().lexeme == "\\" {
                self.advance();
                let token = self.peek();
                let next = token.lexeme.as_str();
                match next {
                    "\"" => { string_val.push('"'); self.advance(); }
                    "\\" => { string_val.push('\\'); self.advance(); }
                    "n" => { string_val.push('\n'); self.advance(); }
                    "t" => { string_val.push('\t'); self.advance(); }
                    _ => {
                        string_val.push('\\');
                        string_val.push_str(next);
                        self.advance();
                    }
                }
            } else {
                let token = self.peek();
                string_val.push_str(&token.lexeme);
                self.advance();
            }
        }

        if self.peek().lexeme != "\"" {
            return Err("Unterminated string".to_string());
        }
        self.advance();

        Ok(string_val)
    }
}

/// Parse tokens to instruction tree
pub fn parse(tokens: Vec<Token>, schema: &LanguageSchema) -> Result<Instruction, String> {
    let mut parser = Parser::new(&tokens, schema);
    parser.parse_program()
}
