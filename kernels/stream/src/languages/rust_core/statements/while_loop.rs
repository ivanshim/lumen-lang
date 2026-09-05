use crate::languages::rust_core::prelude::*;
// while loop statement for mini-rust

use crate::kernel::ast::{Control, ExprNode, StmtNode};
use crate::kernel::parser::Parser;
use crate::kernel::registry::LumenResult;
use crate::languages::rust_core::registry::{Registry, StmtHandler};
use crate::kernel::runtime::Env;
use crate::languages::rust_core::structure::structural;
use crate::languages::rust_core::values::as_bool;

// --------------------
// Token definitions
// --------------------

pub const WHILE: &str = "while";

#[derive(Debug)]
struct WhileStmt {
    condition: Box<dyn ExprNode>,
    body: Vec<Box<dyn StmtNode>>,
}

impl StmtNode for WhileStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        loop {
            let cond = self.condition.eval(env)?;
            let cond_bool = as_bool(cond.as_ref())?;

            if cond_bool.value {
                // Each iteration gets its own scope
                env.push_scope();
                let mut break_occurred = false;
                for stmt in &self.body {
                    match stmt.exec(env)? {
                        Control::Break => {
                            break_occurred = true;
                            break;
                        }
                        Control::Continue => break,
                        Control::ExprValue(_) => {
                            // Expression statement value - continue loop
                        }
                        Control::Return(val) => {
                            env.pop_scope();
                            return Ok(Control::Return(val));
                        }
                        Control::None => {}
                    }
                }
                env.pop_scope();
                if break_occurred {
                    return Ok(Control::None);
                }
            } else {
                break;
            }
        }
        Ok(Control::None)
    }
}

pub struct WhileStmtHandler;

impl StmtHandler for WhileStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == WHILE
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn StmtNode>> {
        parser.advance(); // consume 'while'
        parser.skip_tokens();

        // parse condition expression
        let condition = parser.parse_expr(registry)?;
        parser.skip_tokens();

        // parse indented body
        let body = structural::parse_block(parser, registry)?;

        Ok(Box::new(WhileStmt { condition, body }))
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // No token registration needed - kernel handles all segmentation
    // Register tokens
    // Register handlers
    reg.register_stmt(Box::new(WhileStmtHandler));
}
