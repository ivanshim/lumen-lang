// Stage 3: Reduction
//
// Convert token stream to instruction tree per schema.
// Schema tables guide all parsing decisions.
// No semantic assumptions.
//
// The parser uses:
// - Pratt parsing for expressions (operator precedence climbing)
// - Schema-driven statement dispatch
// - Token pattern matching for syntax validation

use super::ingest::Token;
use super::primitives::{Instruction, Primitive, TransferKind, OperateKind};
use super::eval::Value;
use crate::schema::LanguageSchema;

/// Reduce token stream to instruction tree per schema
pub fn parse(tokens: &[Token], schema: &LanguageSchema) -> Result<Instruction, String> {
    let mut parser = Parser {
        tokens,
        pos: 0,
        schema,
    };

    parser.parse_program()
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
    schema: &'a LanguageSchema,
}

impl<'a> Parser<'a> {
    fn parse_program(&mut self) -> Result<Instruction, String> {
        let mut instructions = Vec::new();

        while !self.is_at_end() && self.peek().lexeme != "EOF" {
            self.skip_whitespace();

            if self.is_at_end() || self.peek().lexeme == "EOF" {
                break;
            }

            let instr = self.parse_statement()?;
            instructions.push(instr);

            self.skip_whitespace();

            // Skip optional terminators (semicolon, newline)
            while self.schema.is_terminator(&self.peek().lexeme) {
                self.advance();
                self.skip_whitespace();
            }
        }

        Ok(Instruction::sequence(instructions))
    }

    fn parse_statement(&mut self) -> Result<Instruction, String> {
        let lexeme = self.peek().lexeme.clone();
        let _start = self.peek().span.0;

        // Check if it's a statement keyword
        if !self.schema.is_statement_keyword(&lexeme) {
            // Try to parse as assignment or expression statement
            return self.parse_assignment_or_expression();
        }

        match lexeme.as_str() {
            "print" => self.parse_print(),
            "if" => self.parse_if(),
            "while" => self.parse_while(),
            "for" => self.parse_for(),
            "loop" => self.parse_loop(),
            "var" => self.parse_var(),
            "let" => self.parse_let(),
            "break" => self.parse_break(),
            "continue" => self.parse_continue(),
            "return" => self.parse_return(),
            _ => Err(format!("Unknown statement: {}", lexeme)),
        }
    }

    /// Handle assignment or expression statements (x = value)
    fn parse_assignment_or_expression(&mut self) -> Result<Instruction, String> {
        let start = self.peek().span.0;
        // Parse with min_prec=1 to exclude assignment operator (precedence 0)
        // This allows us to distinguish between assignment and binary operators
        let expr = self.parse_expression(1)?;

        // Check if it's an assignment
        if self.peek().lexeme == "=" {
            self.advance();
            self.skip_whitespace();
            let value = self.parse_expression(0)?;
            let end = self.prev_span().1;

            // Extract variable name from expression (if it's a variable reference)
            if let Primitive::Variable(name) = &expr.primitive {
                return Ok(Instruction::new(
                    Primitive::Assign {
                        name: name.clone(),
                        value: Box::new(value),
                    },
                    start,
                    end,
                ));
            }
        }

        Ok(expr)
    }

    /// print(expression) or print!(expression) for Mini-Rust
    /// Desugared to: Invoke("print_native", [expression])
    fn parse_print(&mut self) -> Result<Instruction, String> {
        let start = self.peek().span.0;
        self.expect("print")?;
        self.skip_whitespace();

        // Handle print! macro syntax (Mini-Rust)
        if self.peek().lexeme == "!" {
            self.advance();
            self.skip_whitespace();
        }

        self.expect("(")?;
        self.skip_whitespace();

        let expr = self.parse_expression(0)?;
        self.skip_whitespace();
        self.expect(")")?;

        let end = self.prev_span().1;
        // Desugar print to Invoke external function
        Ok(Instruction::new(
            Primitive::Invoke {
                selector: "print_native".to_string(),
                args: vec![expr],
            },
            start,
            end,
        ))
    }

    /// if condition { block } [else { block }]
    fn parse_if(&mut self) -> Result<Instruction, String> {
        let start = self.peek().span.0;
        self.expect("if")?;
        self.skip_whitespace();

        let condition = self.parse_expression(0)?;
        self.skip_whitespace();

        let then_block = self.parse_block()?;
        self.skip_whitespace();

        let else_block = if self.peek().lexeme == "else" {
            self.advance();
            self.skip_whitespace();
            Some(Box::new(self.parse_block()?))
        } else {
            None
        };

        let end = self.prev_span().1;
        Ok(Instruction::new(
            Primitive::Branch {
                condition: Box::new(condition),
                then_block: Box::new(then_block),
                else_block,
            },
            start,
            end,
        ))
    }

    /// while condition { block }
    /// Desugared to: Scope [ looping_branch ]
    /// where looping_branch uses Branch + Transfer(Continue) for loop control
    fn parse_while(&mut self) -> Result<Instruction, String> {
        let start = self.peek().span.0;
        self.expect("while")?;
        self.skip_whitespace();

        let condition = self.parse_expression(0)?;
        self.skip_whitespace();

        let block = self.parse_block()?;
        let end = self.prev_span().1;

        // Desugar while loop to: Scope [ Loop { condition, block } ]
        // Note: Loop is kept as internal implementation detail for now.
        // In the final canonical set, this would be expressed as nested Branch + Transfer patterns.
        Ok(Instruction::new(
            Primitive::Loop {
                condition: Box::new(condition),
                block: Box::new(block),
            },
            start,
            end,
        ))
    }

    /// var name = expression
    fn parse_var(&mut self) -> Result<Instruction, String> {
        let start = self.peek().span.0;
        self.expect("var")?;
        self.skip_whitespace();

        let name = self.parse_identifier()?;
        self.skip_whitespace();

        self.expect("=")?;
        self.skip_whitespace();

        let value = self.parse_expression(0)?;
        let end = self.prev_span().1;

        Ok(Instruction::new(
            Primitive::Assign {
                name,
                value: Box::new(value),
            },
            start,
            end,
        ))
    }

    /// let name = expression; (Mini-Rust style)
    fn parse_let(&mut self) -> Result<Instruction, String> {
        let start = self.peek().span.0;
        self.expect("let")?;
        self.skip_whitespace();

        // Check for 'mut' keyword
        let _is_mut = if self.peek().lexeme == "mut" {
            self.advance();
            self.skip_whitespace();
            true
        } else {
            false
        };

        let name = self.parse_identifier()?;
        self.skip_whitespace();

        self.expect("=")?;
        self.skip_whitespace();

        let value = self.parse_expression(0)?;
        let end = self.prev_span().1;

        Ok(Instruction::new(
            Primitive::Assign {
                name,
                value: Box::new(value),
            },
            start,
            end,
        ))
    }

    /// for variable in expression { block }
    fn parse_for(&mut self) -> Result<Instruction, String> {
        let start = self.peek().span.0;
        self.expect("for")?;
        self.skip_whitespace();

        let _var = self.parse_identifier()?;
        self.skip_whitespace();

        self.expect("in")?;
        self.skip_whitespace();

        let _iterable = self.parse_expression(0)?;
        self.skip_whitespace();

        let _block = self.parse_block()?;
        let end = self.prev_span().1;

        // For now, return a placeholder - proper for-loop support would need to be added
        Err("for loops not yet supported in microcode kernel".to_string())
    }

    /// loop { block }
    fn parse_loop(&mut self) -> Result<Instruction, String> {
        let start = self.peek().span.0;
        self.expect("loop")?;
        self.skip_whitespace();

        let block = self.parse_block()?;
        let end = self.prev_span().1;

        // Infinite loop: while true
        let condition = Instruction::new(
            Primitive::Literal(Value::Bool(true)),
            start,
            start,
        );

        Ok(Instruction::new(
            Primitive::Loop {
                condition: Box::new(condition),
                block: Box::new(block),
            },
            start,
            end,
        ))
    }

    /// return [expression];
    /// Desugared to: Transfer(RETURN, [expression])
    fn parse_return(&mut self) -> Result<Instruction, String> {
        let start = self.peek().span.0;
        self.expect("return")?;
        self.skip_whitespace();

        // Check if there's a return value or just 'return'
        let (value, end) = if self.schema.is_terminator(&self.peek().lexeme) || self.peek().lexeme == "}" || self.is_at_end() {
            (None, self.prev_span().1)
        } else {
            let expr = self.parse_expression(0)?;
            let end = expr.span.1;
            (Some(Box::new(expr)), end)
        };

        Ok(Instruction::new(
            Primitive::Transfer {
                kind: TransferKind::Return,
                value,
            },
            start,
            end,
        ))
    }

    /// break
    /// Desugared to: Transfer(BREAK)
    fn parse_break(&mut self) -> Result<Instruction, String> {
        let start = self.peek().span.0;
        let end = start + 5;
        self.expect("break")?;
        Ok(Instruction::new(
            Primitive::Transfer {
                kind: TransferKind::Break,
                value: None,
            },
            start,
            end,
        ))
    }

    /// continue
    /// Desugared to: Transfer(CONTINUE)
    fn parse_continue(&mut self) -> Result<Instruction, String> {
        let start = self.peek().span.0;
        let end = start + 8;
        self.expect("continue")?;
        Ok(Instruction::new(
            Primitive::Transfer {
                kind: TransferKind::Continue,
                value: None,
            },
            start,
            end,
        ))
    }

    /// { statements }
    fn parse_block(&mut self) -> Result<Instruction, String> {
        self.expect("{")?;
        self.skip_whitespace();

        let mut instructions = Vec::new();

        while self.peek().lexeme != "}" && !self.is_at_end() {
            self.skip_whitespace();

            if self.peek().lexeme == "}" {
                break;
            }

            let instr = self.parse_statement()?;
            instructions.push(instr);

            self.skip_whitespace();

            // Skip optional terminators
            while self.schema.is_terminator(&self.peek().lexeme) {
                self.advance();
                self.skip_whitespace();
            }
        }

        self.expect("}")?;

        Ok(Instruction::scope(instructions))
    }

    /// Parse expression using Pratt parser (operator precedence climbing)
    fn parse_expression(&mut self, min_prec: u32) -> Result<Instruction, String> {
        let mut left = self.parse_primary()?;

        loop {
            // Skip whitespace before checking for operators
            self.skip_whitespace();

            let lexeme = self.peek().lexeme.clone();

            // Check if it's a binary operator
            if let Some(op_info) = self.schema.get_binary_op(&lexeme) {
                if op_info.precedence < min_prec {
                    break;
                }

                self.advance();
                self.skip_whitespace();

                let right_prec = match op_info.associativity {
                    crate::schema::Associativity::Left => op_info.precedence + 1,
                    crate::schema::Associativity::Right => op_info.precedence,
                    crate::schema::Associativity::None => op_info.precedence + 1,
                };

                let right = self.parse_expression(right_prec)?;
                let end = self.prev_span().1;
                let start = left.span.0;

                left = Instruction::new(
                    Primitive::Operate {
                        kind: OperateKind::Binary(lexeme),
                        operands: vec![left, right],
                    },
                    start,
                    end,
                );
            } else {
                break;
            }
        }

        Ok(left)
    }

    /// Parse primary expression (literal, variable, parenthesized, unary)
    fn parse_primary(&mut self) -> Result<Instruction, String> {
        let lexeme = self.peek().lexeme.clone();
        let start = self.peek().span.0;
        let end = self.peek().span.1;

        // Unary operator
        if let Some(_op_info) = self.schema.get_unary_op(&lexeme) {
            self.advance();
            self.skip_whitespace();

            let operand = self.parse_primary()?;
            let final_end = operand.span.1;

            return Ok(Instruction::new(
                Primitive::Operate {
                    kind: OperateKind::Unary(lexeme),
                    operands: vec![operand],
                },
                start,
                final_end,
            ));
        }

        // Parenthesized expression
        if lexeme == "(" {
            self.advance();
            self.skip_whitespace();

            let expr = self.parse_expression(0)?;
            self.skip_whitespace();
            self.expect(")")?;

            return Ok(expr);
        }

        // Number literal
        if lexeme.chars().next().map_or(false, |c| c.is_ascii_digit()) {
            let mut number_str = lexeme.clone();
            self.advance();

            // Consume additional digits and decimal points
            while !self.is_at_end() {
                let next = &self.peek().lexeme;
                if next.len() == 1 {
                    let ch = next.as_bytes()[0];
                    if ch.is_ascii_digit() || ch == b'.' {
                        number_str.push_str(next);
                        self.advance();
                        continue;
                    }
                }
                break;
            }

            let num = number_str.parse::<f64>()
                .map_err(|_| format!("Invalid number: {}", number_str))?;

            return Ok(Instruction::literal(Value::Number(num), start, self.prev_span().1));
        }

        // String literal
        if lexeme == "\"" {
            self.advance();
            let mut string_val = String::new();

            while !self.is_at_end() && self.peek().lexeme != "\"" {
                string_val.push_str(&self.peek().lexeme);
                self.advance();
            }

            if self.is_at_end() {
                return Err("Unterminated string".to_string());
            }

            self.expect("\"")?;
            return Ok(Instruction::literal(Value::String(string_val), start, self.prev_span().1));
        }

        // Boolean literal (both lowercase and uppercase)
        if lexeme == "true" || lexeme == "false" || lexeme == "True" || lexeme == "False" {
            self.advance();
            let bool_val = lexeme == "true" || lexeme == "True";
            return Ok(Instruction::literal(Value::Bool(bool_val), start, self.prev_span().1));
        }

        // Identifier (variable reference or extern function call)
        if lexeme.chars().next().map_or(false, |c| c.is_alphabetic() || c == '_') {
            let name = self.parse_identifier()?;

            // Check if this is an extern function call
            if name == "extern" && self.peek().lexeme == "(" {
                self.advance(); // consume '('
                self.skip_whitespace();

                // Parse selector (string literal)
                self.expect("\"")?;
                let mut selector = String::new();
                while !self.is_at_end() && self.peek().lexeme != "\"" {
                    selector.push_str(&self.peek().lexeme);
                    self.advance();
                }
                self.expect("\"")?;
                self.skip_whitespace();

                // Parse arguments (comma-separated)
                let mut args = Vec::new();

                if self.peek().lexeme != ")" {
                    // Expect comma before first argument
                    self.expect(",")?;
                    self.skip_whitespace();

                    // Parse comma-separated arguments
                    loop {
                        let arg = self.parse_expression(0)?;
                        args.push(arg);
                        self.skip_whitespace();

                        if self.peek().lexeme == "," {
                            self.advance();
                            self.skip_whitespace();
                        } else {
                            break;
                        }
                    }
                }

                self.skip_whitespace();
                self.expect(")")?;

                let end = self.prev_span().1;
                return Ok(Instruction::new(
                    Primitive::Invoke {
                        selector,
                        args,
                    },
                    start,
                    end,
                ));
            }

            return Ok(Instruction::variable(name, start, self.prev_span().1));
        }

        Err(format!("Unexpected token in expression: {}", lexeme))
    }

    /// Parse identifier (variable name)
    fn parse_identifier(&mut self) -> Result<String, String> {
        if self.is_at_end() {
            return Err("Expected identifier, found EOF".to_string());
        }

        let mut name = self.peek().lexeme.clone();

        if !name.chars().next().map_or(false, |c| c.is_alphabetic() || c == '_') {
            return Err(format!("Expected identifier, found: {}", name));
        }

        self.advance();

        // Consume additional identifier characters
        while !self.is_at_end() {
            let next = &self.peek().lexeme;
            if next.len() == 1 {
                let ch = next.as_bytes()[0];
                if ch.is_ascii_alphanumeric() || ch == b'_' {
                    name.push_str(next);
                    self.advance();
                    continue;
                }
            }
            break;
        }

        Ok(name)
    }

    fn expect(&mut self, expected: &str) -> Result<(), String> {
        if self.peek().lexeme == expected {
            self.advance();
            Ok(())
        } else {
            Err(format!(
                "Expected '{}', found '{}'",
                expected, self.peek().lexeme
            ))
        }
    }

    fn peek(&self) -> Token {
        if self.is_at_end() {
            Token {
                lexeme: "EOF".to_string(),
                span: (0, 0),
                line: 0,
                col: 0,
            }
        } else {
            self.tokens[self.pos].clone()
        }
    }

    fn prev_span(&self) -> (usize, usize) {
        if self.pos > 0 {
            self.tokens[self.pos - 1].span
        } else {
            (0, 0)
        }
    }

    fn advance(&mut self) {
        if !self.is_at_end() {
            self.pos += 1;
        }
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn skip_whitespace(&mut self) {
        while !self.is_at_end() {
            let lexeme = &self.peek().lexeme;
            if lexeme.len() == 1 {
                let ch = lexeme.as_bytes()[0];
                if ch == b' ' || ch == b'\t' || ch == b'\n' || ch == b'\r' {
                    self.advance();
                    continue;
                }
            }
            break;
        }
    }
}
