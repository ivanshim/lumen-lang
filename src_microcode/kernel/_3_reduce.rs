// Stage 3: Reduce - Token stream → Instruction tree
//
// Parse tokens into Instruction tree using 7 primitives.
// All semantics come from:
// - Schema (operator precedence, statement patterns)
// - Value types (what data exists)
// - Environment (where data lives)
//
// Parser uses Pratt parsing for expressions + top-down for statements.

use super::eval::Value;
use super::_1_ingest::Token;
use super::primitives::Instruction;
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
        while self.peek().lexeme == " " || self.peek().lexeme == "\t" || self.peek().lexeme == "\n"
        {
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

            // Skip optional semicolon after statement
            if self.peek().lexeme == ";" {
                self.advance();
                self.skip_whitespace();
            }
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
            "for" => self.parse_for(),
            "until" => self.parse_until(),
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

    /// Parse: let [mut] name [: type] = expr
    fn parse_let(&mut self) -> Result<Instruction, String> {
        self.advance(); // consume 'let'
        self.skip_whitespace();

        // Skip optional "mut" keyword
        if self.peek().lexeme == "mut" {
            self.advance();
            self.skip_whitespace();
        }

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
        self.advance(); // consume 'if'
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
        self.advance(); // consume 'while'
        self.skip_whitespace();

        let condition = self.parse_expression()?;
        self.skip_whitespace();

        let body = self.parse_block()?;

        Ok(Instruction::loop_stmt(condition, body))
    }

    /// Parse: for var in iterable { block }
    fn parse_for(&mut self) -> Result<Instruction, String> {
        self.advance(); // consume 'for'
        self.skip_whitespace();

        // Parse loop variable name (simple identifier, stop at keywords)
        if !self
            .peek()
            .lexeme
            .chars()
            .next()
            .map_or(false, |c| c.is_alphabetic() || c == '_')
        {
            return Err(format!("Expected identifier, got: {}", self.peek().lexeme));
        }
        let var = self.peek().lexeme.clone();
        self.advance();
        self.skip_whitespace();

        // Expect 'in' keyword
        if self.peek().lexeme != "in" {
            return Err(format!("Expected 'in' after for loop variable, got: {}", self.peek().lexeme));
        }
        self.advance(); // consume 'in'
        self.skip_whitespace();

        // Parse iterable expression
        let iterable = self.parse_expression()?;
        self.skip_whitespace();

        // Parse block
        let body = self.parse_block()?;

        Ok(Instruction::for_loop(var, iterable, body))
    }

    /// Parse: until condition { block }
    fn parse_until(&mut self) -> Result<Instruction, String> {
        self.advance(); // consume 'until'
        self.skip_whitespace();

        let condition = self.parse_expression()?;
        self.skip_whitespace();

        let body = self.parse_block()?;

        Ok(Instruction::until_loop(condition, body))
    }

    /// Parse: return [expr]
    fn parse_return(&mut self) -> Result<Instruction, String> {
        self.advance(); // consume 'return'
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
        self.advance(); // consume 'fn'
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

        self.advance(); // consume ')'
        self.skip_whitespace();

        let body = self.parse_block()?;

        // Check if function is marked as memoizable by the language schema
        // The kernel is language-agnostic: only memoize if explicitly permitted
        let memoizable = self.schema.memoizable_functions.contains(&name);

        Ok(Instruction::FunctionDef {
            name,
            params,
            body: Box::new(body),
            memoizable,
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

            // Skip optional semicolon or newline after statement
            if self.peek().lexeme == ";" || self.peek().lexeme == "\n" {
                self.advance();
                self.skip_whitespace();
            }
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

            // Handle two cases:
            // 1. Simple assignment: name = value
            if let Instruction::Variable(name) = expr {
                return Ok(Instruction::assign(name, value));
            }

            // 2. Indexed assignment: arr[i] = value
            // Check if expr is an Operate::Binary with "[]" operator
            if let Instruction::Operate { kind: super::primitives::OperateKind::Binary(op), operands } = expr {
                if op == "[]" && operands.len() == 2 {
                    // Extract array name from the left operand
                    if let Instruction::Variable(name) = &operands[0] {
                        let index = operands[1].clone();
                        return Ok(Instruction::indexed_assign(name.clone(), index, value));
                    }
                }
            }

            return Err("Invalid assignment target".to_string());
        }

        Ok(expr)
    }

    /// Parse expression (lowest precedence)
    fn parse_expression(&mut self) -> Result<Instruction, String> {
        self.parse_pipe()
    }

    /// Parse pipe operator (lowest precedence: 0.5)
    fn parse_pipe(&mut self) -> Result<Instruction, String> {
        let mut left = self.parse_logical_or()?;
        self.skip_whitespace();

        while self.peek().lexeme == "|>" {
            self.advance();
            self.skip_whitespace();
            let right = self.parse_logical_or()?;
            self.skip_whitespace();
            left = Instruction::binary("|>".to_string(), left, right);
        }

        Ok(left)
    }

    /// Parse logical OR (lowest precedence after assignment)
    fn parse_logical_or(&mut self) -> Result<Instruction, String> {
        let mut left = self.parse_logical_and()?;
        self.skip_whitespace();

        loop {
            match self.peek().lexeme.as_str() {
                "or" | "||" => {
                    let op = self.peek().lexeme.clone();
                    self.advance();
                    self.skip_whitespace();
                    let right = self.parse_logical_and()?;
                    self.skip_whitespace();
                    left = Instruction::binary(op, left, right);
                }
                _ => break,
            }
        }

        Ok(left)
    }

    /// Parse logical AND
    fn parse_logical_and(&mut self) -> Result<Instruction, String> {
        let mut left = self.parse_comparison()?;
        self.skip_whitespace();

        loop {
            match self.peek().lexeme.as_str() {
                "and" | "&&" => {
                    let op = self.peek().lexeme.clone();
                    self.advance();
                    self.skip_whitespace();
                    let right = self.parse_comparison()?;
                    self.skip_whitespace();
                    left = Instruction::binary(op, left, right);
                }
                _ => break,
            }
        }

        Ok(left)
    }

    /// Parse comparison operators
    fn parse_comparison(&mut self) -> Result<Instruction, String> {
        let mut left = self.parse_range()?;
        self.skip_whitespace();

        loop {
            let op = match self.peek().lexeme.as_str() {
                "==" | "!=" | "<" | ">" | "<=" | ">=" => self.peek().lexeme.clone(),
                _ => break,
            };
            self.advance();
            self.skip_whitespace();
            let right = self.parse_range()?;
            self.skip_whitespace();
            left = Instruction::binary(op, left, right);
        }

        Ok(left)
    }

    /// Parse range operator (..)
    fn parse_range(&mut self) -> Result<Instruction, String> {
        let mut left = self.parse_additive()?;
        self.skip_whitespace();

        while self.peek().lexeme == ".." {
            self.advance();
            self.skip_whitespace();
            let right = self.parse_additive()?;
            self.skip_whitespace();
            left = Instruction::binary("..".to_string(), left, right);
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
        let mut left = self.parse_exponentiation()?;
        self.skip_whitespace();

        loop {
            let op = match self.peek().lexeme.as_str() {
                "*" | "/" | "%" | "//" | "." => self.peek().lexeme.clone(),
                _ => break,
            };
            self.advance();
            self.skip_whitespace();
            let right = self.parse_exponentiation()?;
            self.skip_whitespace();
            left = Instruction::binary(op, left, right);
        }

        Ok(left)
    }

    /// Parse exponentiation operator (right-associative)
    fn parse_exponentiation(&mut self) -> Result<Instruction, String> {
        let left = self.parse_unary()?;
        self.skip_whitespace();

        if self.peek().lexeme == "**" {
            self.advance();
            self.skip_whitespace();
            let right = self.parse_exponentiation()?; // Right-associative!
            return Ok(Instruction::binary("**".to_string(), left, right));
        }

        Ok(left)
    }

    /// Parse unary operators
    fn parse_unary(&mut self) -> Result<Instruction, String> {
        let op = match self.peek().lexeme.as_str() {
            "-" | "not" | "!" => self.peek().lexeme.clone(),
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

        // Numbers (integer or float)
        if lexeme.chars().next().map_or(false, |c| c.is_ascii_digit()) {
            let num_str = self.consume_number()?;

            // Check if it's a float (contains decimal point)
            if num_str.contains('.') {
                let (numerator, denominator) = Self::parse_float(&num_str)?;
                // Float literals are Real values (not Rational)
                // Precision is determined by significant figures
                let precision = Self::calculate_precision(&num_str);
                return Ok(Instruction::literal(Value::Real { numerator, denominator, precision }));
            } else {
                // Parse as integer
                let num = num_str
                    .parse::<num_bigint::BigInt>()
                    .map_err(|_| format!("Invalid number: {}", num_str))?;
                return Ok(Instruction::literal(Value::Number(num)));
            }
        }

        // Strings - double-quoted
        if lexeme == "\"" {
            let string_val = self.consume_string('"')?;
            return Ok(Instruction::literal(Value::String(string_val)));
        }

        // Strings - single-quoted
        if lexeme == "'" {
            let string_val = self.consume_string('\'')?;
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

        // Array literal
        if lexeme == "[" {
            self.advance();
            self.skip_whitespace();
            let mut elements = Vec::new();

            while self.peek().lexeme != "]" {
                elements.push(self.parse_expression()?);
                self.skip_whitespace();
                if self.peek().lexeme == "," {
                    self.advance();
                    self.skip_whitespace();
                }
            }

            if self.peek().lexeme != "]" {
                return Err("Expected ']'".to_string());
            }
            self.advance();

            // Return an instruction that constructs an array from the elements
            return Ok(Instruction::construct_array(elements));
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
        if lexeme
            .chars()
            .next()
            .map_or(false, |c| c.is_alphabetic() || c == '_')
        {
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

                self.advance(); // consume ')'
                let mut expr = Instruction::invoke(name, args);

                // Handle postfix array indexing on function call results: func()[i]
                while self.peek().lexeme == "[" {
                    self.advance(); // consume '['
                    self.skip_whitespace();
                    let index_expr = self.parse_expression()?;
                    self.skip_whitespace();
                    if self.peek().lexeme != "]" {
                        return Err("Expected ']' after array index".to_string());
                    }
                    self.advance(); // consume ']'
                    self.skip_whitespace();

                    expr = Instruction::binary("[]".to_string(), expr, index_expr);
                }

                return Ok(expr);
            }

            let mut expr = Instruction::variable(name);
            self.skip_whitespace();

            // Handle postfix array indexing: var[i]
            while self.peek().lexeme == "[" {
                self.advance(); // consume '['
                self.skip_whitespace();
                let index_expr = self.parse_expression()?;
                self.skip_whitespace();
                if self.peek().lexeme != "]" {
                    return Err("Expected ']' after array index".to_string());
                }
                self.advance(); // consume ']'
                self.skip_whitespace();

                expr = Instruction::binary("[]".to_string(), expr, index_expr);
            }

            return Ok(expr);
        }

        Err(format!("Unexpected token: {}", lexeme))
    }

    /// Parse identifier (handling multi-char identifiers from character tokens)
    /// Also consumes multi-char keyword tokens that are part of the identifier
    fn parse_identifier(&mut self) -> Result<String, String> {
        if !self
            .peek()
            .lexeme
            .chars()
            .next()
            .map_or(false, |c| c.is_alphabetic() || c == '_')
        {
            return Err(format!("Expected identifier, got: {}", self.peek().lexeme));
        }

        let mut name = self.peek().lexeme.clone();
        self.advance();

        loop {
            let next_lexeme = &self.peek().lexeme;

            // Check if next token is single-char alphanumeric/underscore
            if next_lexeme.len() == 1 {
                let ch = next_lexeme.as_bytes()[0] as char;
                if ch.is_alphanumeric() || ch == '_' {
                    name.push_str(next_lexeme);
                    self.advance();
                    continue;
                }
            } else {
                // Check if multi-char token is all alphabetic/underscore
                // (which means it could be a keyword that's part of identifier)
                if next_lexeme.chars().all(|c| c.is_alphabetic() || c == '_') {
                    name.push_str(next_lexeme);
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

    /// Calculate precision (significant figures) from a float literal string
    /// E.g., "1.5" -> 15, "3.14" -> 15, "0.05" -> 15 (minimum 15 significant figures)
    fn calculate_precision(s: &str) -> usize {
        // Remove decimal point and minus sign
        let without_dot: String = s.chars().filter(|c| *c != '.' && *c != '-').collect();

        // Count leading zeros to skip them
        let leading_zeros = without_dot.chars().take_while(|c| *c == '0').count();

        // Significant figures = total digits minus leading zeros
        let significant_count = without_dot.len().saturating_sub(leading_zeros);

        // Use at least 15 significant figures as default minimum
        std::cmp::max(significant_count.max(1), 15)
    }

    /// Parse a float string to (numerator, denominator) rational representation
    /// E.g., "1.5" -> (3, 2), "3.14" -> (314, 100)
    fn parse_float(num_str: &str) -> Result<(num_bigint::BigInt, num_bigint::BigInt), String> {
        use num_bigint::BigInt;

        if let Some(dot_pos) = num_str.find('.') {
            let before_dot = &num_str[..dot_pos];
            let after_dot = &num_str[dot_pos + 1..];

            // Count decimal places to determine denominator
            let decimal_places = after_dot.len();
            let denominator = BigInt::from(10).pow(decimal_places as u32);

            // Parse integer and fractional parts
            let integer_part: BigInt = if before_dot.is_empty() || before_dot == "-" {
                BigInt::from(0)
            } else {
                before_dot.parse::<BigInt>()
                    .map_err(|_| format!("Failed to parse number: {}", num_str))?
            };

            let fractional_part: BigInt = after_dot.parse::<BigInt>()
                .map_err(|_| format!("Failed to parse number: {}", num_str))?;

            // Combine integer and fractional parts: (integer * 10^decimal_places) + fractional
            let is_negative = before_dot.starts_with('-');
            let numerator = if is_negative {
                integer_part * &denominator - fractional_part
            } else {
                integer_part * &denominator + fractional_part
            };

            Ok((numerator, denominator))
        } else {
            Err(format!("parse_float called on non-float: {}", num_str))
        }
    }

    /// Consume a string (handling escape sequences)
    /// quote_char: '"' for double-quoted strings, '\'' for single-quoted strings
    fn consume_string(&mut self, quote_char: char) -> Result<String, String> {
        let quote_str = quote_char.to_string();
        self.advance(); // consume opening quote
        let mut string_val = String::new();

        while self.peek().lexeme != quote_str && !self.is_at_end() {
            if self.peek().lexeme == "\\" {
                self.advance();
                let token = self.peek();
                let next = token.lexeme.as_str();

                if quote_char == '\'' {
                    // Single-quoted strings: only \' and \\ escapes
                    match next {
                        "'" => {
                            string_val.push('\'');
                            self.advance();
                        }
                        "\\" => {
                            string_val.push('\\');
                            self.advance();
                        }
                        _ => {
                            string_val.push('\\');
                            string_val.push_str(next);
                            self.advance();
                        }
                    }
                } else {
                    // Double-quoted strings: \", \\, \n, \t escapes
                    match next {
                        "\"" => {
                            string_val.push('"');
                            self.advance();
                        }
                        "\\" => {
                            string_val.push('\\');
                            self.advance();
                        }
                        "n" => {
                            string_val.push('\n');
                            self.advance();
                        }
                        "t" => {
                            string_val.push('\t');
                            self.advance();
                        }
                        _ => {
                            string_val.push('\\');
                            string_val.push_str(next);
                            self.advance();
                        }
                    }
                }
            } else {
                let token = self.peek();
                string_val.push_str(&token.lexeme);
                self.advance();
            }
        }

        if self.peek().lexeme != quote_str {
            return Err(format!("Unterminated {} string", quote_char));
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
