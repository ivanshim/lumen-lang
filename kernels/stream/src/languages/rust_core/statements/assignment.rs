use crate::languages::rust_core::prelude::*;
// Assignment statement: x = expr

use crate::kernel::ast::{Control, ExprNode, StmtNode};
use crate::kernel::parser::Parser;
use crate::kernel::registry::{KernelResult as LumenResult, err_at};
use crate::languages::rust_core::registry::{Registry, StmtHandler};
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
        // Assignment targets the binding that already exists (loop bodies and
        // branches run in nested scopes), creating it only when there is none.
        env.update(&self.name, val);
        Ok(Control::None)
    }
}

pub struct AssignStmtHandler;

impl StmtHandler for AssignStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        // Check if current token is the start of an identifier
        let curr = &parser.peek().lexeme;
        let is_ident_start = curr.chars().next().map_or(false, word_start);

        if !is_ident_start {
            return false;
        }

        // Look ahead for '=' (skip whitespace tokens and identifier continuation tokens)
        // Since the kernel lexer is agnostic, multi-character identifiers are split into single chars
        let mut i = 1;
        while let Some(t) = parser.peek_n(i) {
            let lexeme = &t.lexeme;

            // Skip whitespace tokens
            if lexeme.chars().count() == 1 {
                let ch = lexeme.chars().next().unwrap();
                if ch == ' ' || ch == '\t' || ch == '\n' || ch == '\r' {
                    i += 1;
                    continue;
                }
                // Skip identifier continuation characters (letters, digits, underscores)
                if word_char(ch) {
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
            if parser.peek().lexeme.chars().count() == 1 {
                let ch = parser.peek().lexeme.chars().next().unwrap();
                if word_char(ch) {
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
