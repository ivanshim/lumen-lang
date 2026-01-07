use crate::languages::rust::prelude::*;
// Assignment statement: x = expr

use crate::kernel::ast::{Control, ExprNode, StmtNode};
use crate::kernel::lexer::Token;
use crate::kernel::parser::Parser;
use crate::kernel::registry::{LumenResult, err_at};
use crate::languages::rust::registry::{Registry, StmtHandler};
use crate::kernel::runtime::{Env, Value};

// --------------------
// Token definitions
// --------------------

pub const EQUALS: &str = "=";

#[derive(Debug)]
struct AssignStmt {
    name: String,
    expr: Box<dyn ExprNode>,
}

impl StmtNode for AssignStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        let val: Value = self.expr.eval(env)?;
        env.assign(&self.name, val)?;
        Ok(Control::None)
    }
}

pub struct AssignStmtHandler;

impl StmtHandler for AssignStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        // Check if current token is the start of an identifier
        let curr = &parser.peek().lexeme;
        let is_ident_start = curr.chars().next().map_or(false, |c| c.is_alphabetic() || c == '_');

        if !is_ident_start {
            return false;
        }

        // Look ahead for '=' (skip whitespace tokens and identifier continuation tokens)
        // Since the kernel lexer is agnostic, multi-character identifiers are split into single chars
        let mut i = 1;
        while let Some(t) = parser.peek_n(i) {
            let lexeme = &t.lexeme;

            // Skip whitespace tokens
            if lexeme.len() == 1 {
                let ch = lexeme.as_bytes()[0];
                if ch == b' ' || ch == b'\t' || ch == b'\n' || ch == b'\r' {
                    i += 1;
                    continue;
                }
                // Skip identifier continuation characters (letters, digits, underscores)
                if ch.is_ascii_alphanumeric() || ch == b'_' {
                    i += 1;
                    continue;
                }
            }

            // Check if we found '='
            if lexeme == EQUALS {
                return true;
            }

            // Anything else means not an assignment
            break;
        }

        false
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn StmtNode>> {
        // Consume first character of identifier
        let mut name = parser.advance().lexeme;

        // Since kernel lexer is agnostic, consume remaining identifier characters
        loop {
            if parser.peek().lexeme.len() == 1 {
                let ch = parser.peek().lexeme.as_bytes()[0];
                if ch.is_ascii_alphanumeric() || ch == b'_' {
                    name.push_str(&parser.advance().lexeme);
                    continue;
                }
            }
            break;
        }

        parser.skip_tokens();

        if parser.advance().lexeme != EQUALS {
            return Err(err_at(parser, "Expected '=' in assignment"));
        }
        parser.skip_tokens();

        let expr = parser.parse_expr(registry)?;
        Ok(Box::new(AssignStmt { name, expr }))
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // No token registration needed - kernel handles all segmentation
    // Register tokens
    // Register handlers
    reg.register_stmt(Box::new(AssignStmtHandler));
}
