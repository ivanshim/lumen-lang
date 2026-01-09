// Generic parser for opaque kernel
// Language-specific parsing logic delegated to language modules via opaque analysis

use crate::kernel::ast::{ExprNode, StmtNode, Program};
use crate::kernel::lexer::Token;
use crate::kernel::handlers::HandlerRegistry;
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

/// Parse a program from tokens using registered statement handlers
pub fn parse_program(mut parser: Parser, handlers: &HandlerRegistry) -> Result<Program, String> {
    let mut statements = Vec::new();

    while !parser.is_at_end() {
        // Skip newlines, markers, and semicolons between statements
        while parser.consume("newline") || parser.consume("marker_indent_start")
            || parser.consume("marker_indent_end") || parser.consume("semicolon") {
        }

        if parser.is_at_end() {
            break;
        }

        match parse_statement(&mut parser, handlers) {
            Ok(stmt) => {
                statements.push(stmt);
                // Consume optional semicolon after statement
                parser.consume("semicolon");
            }
            Err(e) => {
                // For now, report the error but try to continue
                eprintln!("Parse error: {}", e);
                parser.synchronize();
            }
        }
    }

    Ok(Program { statements })
}

/// Parse a single statement using registered handlers
/// Kernel delegates to language-specific handlers; if none match, falls back to expression statement
pub fn parse_statement(parser: &mut Parser, handlers: &HandlerRegistry) -> Result<StmtNode, String> {
    if parser.is_at_end() {
        return Err("Unexpected end of input".to_string());
    }

    // Try language-specific handlers first
    if let Some(result) = handlers.parse_statement(parser) {
        return result;
    }

    // Fall back to generic statement types (assignment and expression)
    if let Some(token) = parser.peek() {
        match token.token_type.as_str() {
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

    while parser.check_lexeme("or") || parser.check_lexeme("||") {
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

    while parser.check_lexeme("and") || parser.check_lexeme("&&") {
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
