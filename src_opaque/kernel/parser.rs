// Generic parser for opaque kernel
// Language-specific parsing logic delegated to language modules via opaque analysis

use crate::kernel::ast::{ExprNode, StmtNode, Program};
use crate::kernel::lexer::Token;
use std::collections::VecDeque;

/// Parser token with position information
#[derive(Debug, Clone)]
pub struct ParseToken {
    pub token_type: String,
    pub lexeme: String,
    pub line: usize,
    pub col: usize,
}

impl From<Token> for ParseToken {
    fn from(t: Token) -> Self {
        ParseToken {
            token_type: t.token_type,
            lexeme: t.lexeme,
            line: t.line,
            col: t.col,
        }
    }
}

/// Generic parser - delegates semantic decisions to language modules
pub struct Parser {
    tokens: VecDeque<ParseToken>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        let parse_tokens: VecDeque<ParseToken> = tokens.into_iter().map(|t| t.into()).collect();
        Parser {
            tokens: parse_tokens,
            current: 0,
        }
    }

    /// Get current token without consuming
    pub fn peek(&self) -> Option<&ParseToken> {
        self.tokens.get(self.current)
    }

    /// Get next token without consuming
    pub fn peek_ahead(&self, n: usize) -> Option<&ParseToken> {
        self.tokens.get(self.current + n)
    }

    /// Consume and return current token
    pub fn next(&mut self) -> Option<ParseToken> {
        if self.current < self.tokens.len() {
            let token = self.tokens[self.current].clone();
            self.current += 1;
            Some(token)
        } else {
            None
        }
    }

    /// Check if current token matches a type
    pub fn check(&self, token_type: &str) -> bool {
        self.peek().map(|t| t.token_type == token_type).unwrap_or(false)
    }

    /// Check if current lexeme matches a string
    pub fn check_lexeme(&self, lexeme: &str) -> bool {
        self.peek().map(|t| t.lexeme == lexeme).unwrap_or(false)
    }

    /// Consume token if it matches, otherwise error
    pub fn expect(&mut self, token_type: &str) -> Result<ParseToken, String> {
        if self.check(token_type) {
            Ok(self.next().unwrap())
        } else {
            let found = self.peek().map(|t| t.token_type.clone()).unwrap_or_else(|| "EOF".to_string());
            Err(format!("Expected {}, got {}", token_type, found))
        }
    }

    /// Consume token if it matches
    pub fn consume(&mut self, token_type: &str) -> bool {
        if self.check(token_type) {
            self.next();
            true
        } else {
            false
        }
    }

    /// Check if at end of tokens
    pub fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len()
    }

    /// Synchronize to next statement after error
    pub fn synchronize(&mut self) {
        while !self.is_at_end() {
            if let Some(token) = self.peek() {
                match token.token_type.as_str() {
                    "keyword_if" | "keyword_while" | "keyword_for" | "keyword_fn"
                    | "keyword_let" | "keyword_print" | "keyword_return" => break,
                    _ => {}
                }
            }
            self.next();
        }
    }

    /// Get all remaining tokens (used for debugging)
    pub fn remaining_count(&self) -> usize {
        self.tokens.len() - self.current
    }
}

/// Parse a program from tokens
pub fn parse_program(mut parser: Parser) -> Result<Program, String> {
    let mut statements = Vec::new();

    while !parser.is_at_end() {
        // Skip newlines and markers between statements
        while parser.consume("newline") || parser.consume("marker_indent_start")
            || parser.consume("marker_indent_end") {
        }

        if parser.is_at_end() {
            break;
        }

        match parse_statement(&mut parser) {
            Ok(stmt) => statements.push(stmt),
            Err(e) => {
                // For now, report the error but try to continue
                // In production, you might want to collect all errors
                eprintln!("Parse error: {}", e);
                parser.synchronize();
            }
        }
    }

    Ok(Program { statements })
}

/// Parse a single statement
pub fn parse_statement(parser: &mut Parser) -> Result<StmtNode, String> {
    if parser.is_at_end() {
        return Err("Unexpected end of input".to_string());
    }

    if let Some(token) = parser.peek() {
        match token.token_type.as_str() {
            // Keywords that start statements
            "keyword_if" => parse_if_statement(parser),
            "keyword_while" => parse_while_statement(parser),
            "keyword_until" => parse_until_statement(parser),
            "keyword_for" => parse_for_statement(parser),
            "keyword_fn" => parse_function_definition(parser),
            "keyword_let" => parse_let_statement(parser),
            "keyword_let_mut" => parse_let_mut_statement(parser),
            "keyword_print" => parse_print_statement(parser),
            "keyword_return" => parse_return_statement(parser),
            "keyword_break" => {
                parser.next();
                Ok(StmtNode::Break)
            }
            "keyword_continue" => {
                parser.next();
                Ok(StmtNode::Continue)
            }
            // Check for assignment (identifier followed by assign operator)
            "identifier" => {
                // Look ahead to check for assignment, skipping whitespace and markers
                let mut lookahead_idx = 1;
                let mut found_assign = false;
                while let Some(next_token) = parser.peek_ahead(lookahead_idx) {
                    if next_token.token_type == "assign" {
                        found_assign = true;
                        break;
                    } else if matches!(next_token.token_type.as_str(),
                        "newline" | "marker_indent_start" | "marker_indent_end") {
                        // Skip these tokens when checking for assignment
                        lookahead_idx += 1;
                    } else {
                        // Found a different token, not an assignment
                        break;
                    }
                }

                if found_assign {
                    // Parse as assignment statement
                    let target_token = parser.next().unwrap();
                    let target = target_token.lexeme;

                    // Skip whitespace and markers
                    while parser.check("newline") || parser.check("marker_indent_start")
                        || parser.check("marker_indent_end") {
                        parser.next();
                    }

                    parser.expect("assign")?;
                    let value = parse_expression(parser)?;
                    return Ok(StmtNode::Assign {
                        target,
                        value,
                        analysis: (),
                    });
                }

                // Not an assignment, parse as expression statement
                let expr = parse_expression(parser)?;
                Ok(StmtNode::Expr { expr })
            }
            // Expression statement (other expression)
            _ => {
                let expr = parse_expression(parser)?;
                Ok(StmtNode::Expr { expr })
            }
        }
    } else {
        Err("Unexpected end of input".to_string())
    }
}

// Statement parsing helpers (stubs - will be populated with actual Lumen semantics)

fn parse_if_statement(parser: &mut Parser) -> Result<StmtNode, String> {
    parser.expect("keyword_if")?;
    let condition = parse_expression(parser)?;

    // Consume newline and markers if present
    while parser.consume("newline") || parser.consume("marker_indent_start") {}

    let mut then_block = Vec::new();
    while !parser.is_at_end() && !parser.check("keyword_end") && !parser.check("keyword_else")
        && !parser.check("marker_indent_end") {
        if parser.consume("newline") || parser.consume("marker_indent_start")
            || parser.consume("marker_indent_end") {
            continue;
        }
        then_block.push(parse_statement(parser)?);
    }

    let else_block = if parser.check("keyword_else") {
        parser.next();
        while parser.consume("newline") || parser.consume("marker_indent_start") {}

        let mut block = Vec::new();
        while !parser.is_at_end() && !parser.check("keyword_end")
            && !parser.check("marker_indent_end") {
            if parser.consume("newline") || parser.consume("marker_indent_start")
                || parser.consume("marker_indent_end") {
                continue;
            }
            block.push(parse_statement(parser)?);
        }
        Some(block)
    } else {
        None
    };

    if parser.check("keyword_end") {
        parser.next();
    }

    Ok(StmtNode::If {
        condition,
        then_block,
        else_block,
        analysis: (),
    })
}

fn parse_while_statement(parser: &mut Parser) -> Result<StmtNode, String> {
    parser.expect("keyword_while")?;
    let condition = parse_expression(parser)?;

    while parser.consume("newline") || parser.consume("marker_indent_start") {}

    let mut body = Vec::new();
    while !parser.is_at_end() && !parser.check("keyword_end")
        && !parser.check("marker_indent_end") {
        if parser.consume("newline") || parser.consume("marker_indent_start")
            || parser.consume("marker_indent_end") {
            continue;
        }
        body.push(parse_statement(parser)?);
    }

    if parser.check("keyword_end") {
        parser.next();
    }

    Ok(StmtNode::While {
        condition,
        body,
        analysis: (),
    })
}

fn parse_until_statement(parser: &mut Parser) -> Result<StmtNode, String> {
    parser.expect("keyword_until")?;
    let condition = parse_expression(parser)?;

    while parser.consume("newline") || parser.consume("marker_indent_start") {}

    let mut body = Vec::new();
    while !parser.is_at_end() && !parser.check("keyword_end")
        && !parser.check("marker_indent_end") {
        if parser.consume("newline") || parser.consume("marker_indent_start")
            || parser.consume("marker_indent_end") {
            continue;
        }
        body.push(parse_statement(parser)?);
    }

    if parser.check("keyword_end") {
        parser.next();
    }

    Ok(StmtNode::Until {
        condition,
        body,
        analysis: (),
    })
}

fn parse_for_statement(parser: &mut Parser) -> Result<StmtNode, String> {
    parser.expect("keyword_for")?;
    let variable_token = parser.expect("identifier")?;
    let variable = variable_token.lexeme;

    parser.expect("keyword_in")?;
    let iterator = parse_expression(parser)?;

    while parser.consume("newline") || parser.consume("marker_indent_start") {}

    let mut body = Vec::new();
    while !parser.is_at_end() && !parser.check("keyword_end")
        && !parser.check("marker_indent_end") {
        if parser.consume("newline") || parser.consume("marker_indent_start")
            || parser.consume("marker_indent_end") {
            continue;
        }
        body.push(parse_statement(parser)?);
    }

    if parser.check("keyword_end") {
        parser.next();
    }

    Ok(StmtNode::For {
        variable,
        iterator,
        body,
        analysis: (),
    })
}

fn parse_function_definition(parser: &mut Parser) -> Result<StmtNode, String> {
    parser.expect("keyword_fn")?;
    let name_token = parser.expect("identifier")?;
    let name = name_token.lexeme;

    parser.expect("lparen")?;
    let mut params = Vec::new();

    while !parser.check("rparen") {
        let param = parser.expect("identifier")?;
        params.push(param.lexeme);

        if parser.check("comma") {
            parser.next();
        } else if !parser.check("rparen") {
            return Err("Expected comma or )".to_string());
        }
    }

    parser.expect("rparen")?;

    while parser.consume("newline") || parser.consume("marker_indent_start") {}

    let mut body = Vec::new();
    while !parser.is_at_end() && !parser.check("keyword_end")
        && !parser.check("marker_indent_end") {
        if parser.consume("newline") || parser.consume("marker_indent_start")
            || parser.consume("marker_indent_end") {
            continue;
        }
        body.push(parse_statement(parser)?);
    }

    if parser.check("keyword_end") {
        parser.next();
    }

    Ok(StmtNode::FnDef {
        name,
        params,
        body,
        analysis: (),
    })
}

fn parse_let_statement(parser: &mut Parser) -> Result<StmtNode, String> {
    parser.expect("keyword_let")?;

    // Check if this is "let mut" or just "let"
    let is_mut = parser.check("keyword_mut");
    if is_mut {
        parser.next(); // consume "mut"
    }

    let name_token = parser.expect("identifier")?;
    let name = name_token.lexeme;

    let value = if parser.consume("assign") {
        Some(parse_expression(parser)?)
    } else {
        None
    };

    if is_mut {
        Ok(StmtNode::LetMut {
            name,
            value,
            analysis: (),
        })
    } else {
        Ok(StmtNode::Let {
            name,
            value,
            analysis: (),
        })
    }
}

fn parse_let_mut_statement(parser: &mut Parser) -> Result<StmtNode, String> {
    // This is called when we see "keyword_let" and the next token is "keyword_mut"
    // But with the updated parse_let_statement, we don't need this separately
    // However, keep it for compatibility if needed
    parse_let_statement(parser)
}

fn parse_print_statement(parser: &mut Parser) -> Result<StmtNode, String> {
    parser.expect("keyword_print")?;

    let mut arguments = Vec::new();

    if parser.consume("lparen") {
        while !parser.check("rparen") {
            arguments.push(parse_expression(parser)?);

            if parser.check("comma") {
                parser.next();
            } else if !parser.check("rparen") {
                break;
            }
        }
        parser.expect("rparen")?;
    } else {
        // Print with space-separated args
        while !parser.is_at_end() && !parser.check("newline")
            && !parser.check("marker_indent_end") && !parser.check("keyword_end") {
            arguments.push(parse_expression(parser)?);
            if parser.check("comma") {
                parser.next();
            } else {
                break;
            }
        }
    }

    Ok(StmtNode::Print {
        arguments,
        analysis: (),
    })
}

fn parse_return_statement(parser: &mut Parser) -> Result<StmtNode, String> {
    parser.expect("keyword_return")?;

    let value = if !parser.is_at_end() && !parser.check("newline")
        && !parser.check("marker_indent_end") && !parser.check("keyword_end") {
        Some(parse_expression(parser)?)
    } else {
        None
    };

    Ok(StmtNode::Return {
        value,
        analysis: (),
    })
}

/// Parse an expression
pub fn parse_expression(parser: &mut Parser) -> Result<ExprNode, String> {
    parse_assignment(parser)
}

/// Parse assignment or lower precedence
fn parse_assignment(parser: &mut Parser) -> Result<ExprNode, String> {
    let expr = parse_logical_or(parser)?;

    // Assignment is handled at statement level, so we just return the expression here
    Ok(expr)
}

/// Parse logical OR
fn parse_logical_or(parser: &mut Parser) -> Result<ExprNode, String> {
    let mut left = parse_logical_and(parser)?;

    while parser.check_lexeme("or") {
        let op_token = parser.next().unwrap();
        let right = parse_logical_and(parser)?;
        left = ExprNode::Infix {
            left: Box::new(left),
            operator: op_token.lexeme,
            right: Box::new(right),
            analysis: (),
        };
    }

    Ok(left)
}

/// Parse logical AND
fn parse_logical_and(parser: &mut Parser) -> Result<ExprNode, String> {
    let mut left = parse_range(parser)?;

    while parser.check_lexeme("and") {
        let op_token = parser.next().unwrap();
        let right = parse_range(parser)?;
        left = ExprNode::Infix {
            left: Box::new(left),
            operator: op_token.lexeme,
            right: Box::new(right),
            analysis: (),
        };
    }

    Ok(left)
}

/// Parse range operators (.., ..=)
fn parse_range(parser: &mut Parser) -> Result<ExprNode, String> {
    let mut left = parse_comparison(parser)?;

    if let Some(token) = parser.peek() {
        if token.token_type == "range_op" {
            let op_token = parser.next().unwrap();
            let right = parse_comparison(parser)?;
            left = ExprNode::Infix {
                left: Box::new(left),
                operator: op_token.lexeme,
                right: Box::new(right),
                analysis: (),
            };
        }
    }

    Ok(left)
}

/// Parse comparison operators
fn parse_comparison(parser: &mut Parser) -> Result<ExprNode, String> {
    let mut left = parse_additive(parser)?;

    loop {
        if let Some(token) = parser.peek() {
            match token.token_type.as_str() {
                "operator" if matches!(token.lexeme.as_str(), "==" | "!=" | "<" | ">" | "<=" | ">=") => {
                    let op_token = parser.next().unwrap();
                    let right = parse_additive(parser)?;
                    left = ExprNode::Infix {
                        left: Box::new(left),
                        operator: op_token.lexeme,
                        right: Box::new(right),
                        analysis: (),
                    };
                }
                _ => break,
            }
        } else {
            break;
        }
    }

    Ok(left)
}

/// Parse addition and subtraction
fn parse_additive(parser: &mut Parser) -> Result<ExprNode, String> {
    let mut left = parse_multiplicative(parser)?;

    loop {
        if let Some(token) = parser.peek() {
            match token.token_type.as_str() {
                "operator" if matches!(token.lexeme.as_str(), "+" | "-") => {
                    let op_token = parser.next().unwrap();
                    let right = parse_multiplicative(parser)?;
                    left = ExprNode::Infix {
                        left: Box::new(left),
                        operator: op_token.lexeme,
                        right: Box::new(right),
                        analysis: (),
                    };
                }
                _ => break,
            }
        } else {
            break;
        }
    }

    Ok(left)
}

/// Parse multiplication, division, modulo
fn parse_multiplicative(parser: &mut Parser) -> Result<ExprNode, String> {
    let mut left = parse_exponential(parser)?;

    loop {
        if let Some(token) = parser.peek() {
            match token.token_type.as_str() {
                "operator" if matches!(token.lexeme.as_str(), "*" | "/" | "%") => {
                    let op_token = parser.next().unwrap();
                    let right = parse_exponential(parser)?;
                    left = ExprNode::Infix {
                        left: Box::new(left),
                        operator: op_token.lexeme,
                        right: Box::new(right),
                        analysis: (),
                    };
                }
                _ => break,
            }
        } else {
            break;
        }
    }

    Ok(left)
}

/// Parse exponentiation (right-associative)
fn parse_exponential(parser: &mut Parser) -> Result<ExprNode, String> {
    let mut left = parse_prefix(parser)?;

    if let Some(token) = parser.peek() {
        if token.token_type == "operator" && token.lexeme == "**" {
            let op_token = parser.next().unwrap();
            // Right-associative: recursively call parse_exponential for the right side
            let right = parse_exponential(parser)?;
            left = ExprNode::Infix {
                left: Box::new(left),
                operator: op_token.lexeme,
                right: Box::new(right),
                analysis: (),
            };
        }
    }

    Ok(left)
}

/// Parse prefix operators
fn parse_prefix(parser: &mut Parser) -> Result<ExprNode, String> {
    if let Some(token) = parser.peek() {
        match token.token_type.as_str() {
            "operator" if matches!(token.lexeme.as_str(), "-" | "!") => {
                let op_token = parser.next().unwrap();
                let right = parse_prefix(parser)?;
                return Ok(ExprNode::Prefix {
                    operator: op_token.lexeme,
                    right: Box::new(right),
                    analysis: (),
                });
            }
            "keyword_not" => {
                parser.next();
                let right = parse_prefix(parser)?;
                return Ok(ExprNode::Prefix {
                    operator: "not".to_string(),
                    right: Box::new(right),
                    analysis: (),
                });
            }
            _ => {}
        }
    }

    parse_call(parser)
}

/// Parse function calls and postfix operations
fn parse_call(parser: &mut Parser) -> Result<ExprNode, String> {
    let mut expr = parse_primary(parser)?;

    loop {
        if parser.consume("lparen") {
            let mut arguments = Vec::new();

            while !parser.check("rparen") {
                arguments.push(parse_expression(parser)?);

                if parser.consume("comma") {
                    // Continue
                } else if !parser.check("rparen") {
                    break;
                }
            }

            parser.expect("rparen")?;

            expr = ExprNode::Call {
                function: Box::new(expr),
                arguments,
                analysis: (),
            };
        } else {
            break;
        }
    }

    Ok(expr)
}

/// Parse primary expressions
fn parse_primary(parser: &mut Parser) -> Result<ExprNode, String> {
    if parser.is_at_end() {
        return Err("Unexpected end of input".to_string());
    }

    if let Some(token) = parser.peek() {
        match token.token_type.as_str() {
            "identifier" => {
                let name = parser.next().unwrap().lexeme;
                Ok(ExprNode::Identifier { name })
            }
            "number" | "string" | "keyword_true" | "keyword_false" | "keyword_none" => {
                let lit = parser.next().unwrap();
                Ok(ExprNode::Literal {
                    lexeme: lit.lexeme,
                    handler_type: lit.token_type,
                })
            }
            "lparen" => {
                parser.next();
                let expr = parse_expression(parser)?;
                parser.expect("rparen")?;
                Ok(ExprNode::Grouped {
                    expr: Box::new(expr),
                })
            }
            _ => Err(format!("Unexpected token: {}", token.lexeme)),
        }
    } else {
        Err("Unexpected end of input".to_string())
    }
}
