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

            // Handle three cases:
            // 1. MEMOIZATION assignment (system control)
            if let Instruction::Variable(name) = &expr {
                if name == "MEMOIZATION" {
                    // Extract boolean value from either Variable or Literal
                    match &value {
                        Instruction::Variable(bool_str) => {
                            match bool_str.as_str() {
                                "true" => return Ok(Instruction::SetMemoization { enabled: true }),
                                "false" => return Ok(Instruction::SetMemoization { enabled: false }),
                                _ => return Err(format!("MEMOIZATION must be set to 'true' or 'false', got: {}", bool_str)),
                            }
                        }
                        Instruction::Literal(val) => {
                            if let crate::kernel::eval::Value::Bool(b) = val {
                                return Ok(Instruction::SetMemoization { enabled: *b });
                            }
                            return Err("MEMOIZATION must be set to a boolean literal (true or false)".to_string());
                        }
                        _ => {
                            return Err("MEMOIZATION must be set to a boolean literal (true or false)".to_string());
                        }
                    }
                }
            }

            // 2. Simple assignment: name = value
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

        // Numbers (integer or float or base-N)
        if lexeme.chars().next().map_or(false, |c| c.is_ascii_digit()) {
            let num_str = self.consume_number()?;

            // Check if it's a base-N literal (contains '@')
            if num_str.contains('@') {
                let (numerator, denominator) = Self::parse_base_n_literal(&num_str)?;
                // Base-N literals with fractional part are Real
                if denominator != num_bigint::BigInt::from(1) {
                    let precision = Self::calculate_precision(&num_str);
                    return Ok(Instruction::literal(Value::Real { numerator, denominator, precision }));
                } else {
                    // Base-N integer literal
                    return Ok(Instruction::literal(Value::Number(numerator)));
                }
            }

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

        // Null
        if lexeme == "null" {
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

    /// Consume a number (handling multi-char numbers and base-N literals)
    /// For base-N literals: <base>@<digits>[.<fraction>][^<exponent>]
    fn consume_number(&mut self) -> Result<String, String> {
        let mut num_str = self.peek().lexeme.clone();
        self.advance();

        loop {
            let token = self.peek();
            let ch = token.lexeme.as_str();
            if ch.len() == 1 {
                let b = ch.as_bytes()[0] as char;
                // Consume digits, '.', '@', and '^' for base-N literals
                if b.is_ascii_digit() || b == '.' || b == '@' || b == '^' {
                    num_str.push_str(ch);
                    self.advance();
                    continue;
                }
                // For base-N literals, also consume letters (a-z, A-Z) after '@'
                if num_str.contains('@') && (b.is_ascii_lowercase() || b.is_ascii_uppercase()) {
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

    /// Parse a base-N numeric literal: <base>@<digits>[.<fraction>][^<exponent>]
    /// Examples: 16@FF, 2@1011, 36@1234.wxyz, 10@123.45^6
    /// Returns (numerator, denominator) where denominator is 1 for integers
    fn parse_base_n_literal(num_str: &str) -> Result<(num_bigint::BigInt, num_bigint::BigInt), String> {
        use num_bigint::BigInt;
        use num_traits::cast::ToPrimitive;

        // Find the '@' separator
        let at_pos = num_str.find('@')
            .ok_or_else(|| format!("Invalid base-N literal: missing '@' in '{}'", num_str))?;

        // Parse base (always in decimal)
        let base_str = &num_str[..at_pos];
        let base: u32 = base_str.parse()
            .map_err(|_| format!("Invalid base in literal '{}': base must be decimal integer", num_str))?;

        // Validate base range [2, 36]
        if base < 2 || base > 36 {
            return Err(format!("Invalid base {}: must be between 2 and 36", base));
        }

        // Parse the rest: <digits>[.<fraction>][^<exponent>]
        let rest = &num_str[at_pos + 1..];

        if rest.is_empty() {
            return Err(format!("Invalid base-N literal '{}': missing digits after '@'", num_str));
        }

        // Split by '^' for exponent
        let (mantissa_str, exp_str) = if let Some(exp_pos) = rest.find('^') {
            let mantissa = &rest[..exp_pos];
            let exp = &rest[exp_pos + 1..];
            if exp.is_empty() {
                return Err(format!("Invalid base-N literal '{}': missing digits after '^'", num_str));
            }
            (mantissa, Some(exp))
        } else {
            (rest, None)
        };

        // Split mantissa by '.' for fractional part
        let (int_str, frac_str) = if let Some(dot_pos) = mantissa_str.find('.') {
            let int_part = &mantissa_str[..dot_pos];
            let frac_part = &mantissa_str[dot_pos + 1..];
            if frac_part.is_empty() {
                return Err(format!("Invalid base-N literal '{}': missing digits after '.'", num_str));
            }
            (int_part, Some(frac_part))
        } else {
            (mantissa_str, None)
        };

        if int_str.is_empty() {
            return Err(format!("Invalid base-N literal '{}': missing digits before '.' or '^'", num_str));
        }

        // Parse integer part
        let int_value = Self::parse_digits_in_base(int_str, base)
            .map_err(|e| format!("Invalid base-N literal '{}': {}", num_str, e))?;

        // Parse fractional part if present
        let (numerator, denominator) = if let Some(frac) = frac_str {
            let frac_value = Self::parse_digits_in_base(frac, base)
                .map_err(|e| format!("Invalid base-N literal '{}': {}", num_str, e))?;

            // fractional value = frac_value / base^frac_digits
            let frac_digits = frac.len() as u32;
            let frac_denominator = BigInt::from(base).pow(frac_digits);

            // Combined: int_value + frac_value/frac_denominator
            // = (int_value * frac_denominator + frac_value) / frac_denominator
            let combined_numerator = int_value * &frac_denominator + frac_value;
            (combined_numerator, frac_denominator)
        } else {
            // Integer literal (no fraction)
            (int_value, BigInt::from(1))
        };

        // Apply exponent if present
        let (final_numerator, final_denominator) = if let Some(exp) = exp_str {
            let exp_value = Self::parse_digits_in_base(exp, base)
                .map_err(|e| format!("Invalid base-N literal '{}': exponent {}", num_str, e))?;

            // Convert exponent to u32
            let exp_u32 = exp_value.to_u32()
                .ok_or_else(|| format!("Invalid base-N literal '{}': exponent too large", num_str))?;

            // Multiply by base^exponent
            let multiplier = BigInt::from(base).pow(exp_u32);
            (numerator * multiplier, denominator)
        } else {
            (numerator, denominator)
        };

        Ok((final_numerator, final_denominator))
    }

    /// Parse a string of digits in the given base
    /// Digits: 0-9 for values 0-9, a-z/A-Z for values 10-35
    fn parse_digits_in_base(digits: &str, base: u32) -> Result<num_bigint::BigInt, String> {
        use num_bigint::BigInt;
        let mut result = BigInt::from(0);
        let base_bigint = BigInt::from(base);

        for ch in digits.chars() {
            let digit_value = match ch {
                '0'..='9' => (ch as u32) - ('0' as u32),
                'a'..='z' => (ch as u32) - ('a' as u32) + 10,
                'A'..='Z' => (ch as u32) - ('A' as u32) + 10,
                _ => return Err(format!("invalid digit '{}' for base {}", ch, base)),
            };

            if digit_value >= base {
                return Err(format!("digit '{}' (value {}) is not valid in base {}", ch, digit_value, base));
            }

            result = result * &base_bigint + BigInt::from(digit_value);
        }

        Ok(result)
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
