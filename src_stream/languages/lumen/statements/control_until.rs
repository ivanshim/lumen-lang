// src/stmt/until_loop.rs
//
// until <expr>
//     <block>
//
// Executes block once, then loops while expr is false
// (Stops looping when expr becomes true)
//
// Desugars into: { block; while not expr { block } }

use crate::languages::lumen::prelude::*;
use crate::kernel::ast::{Control, ExprNode, StmtNode};
use crate::kernel::parser::Parser;
use crate::languages::lumen::patterns::PatternSet;
use crate::kernel::runtime::Env;
use crate::languages::lumen::structure::structural;
use crate::languages::lumen::values::as_bool;

#[derive(Debug)]
struct UntilStmt {
    condition: Box<dyn ExprNode>,
    body: Vec<Box<dyn StmtNode>>,
}

impl StmtNode for UntilStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        loop {
            // Execute body first (at least once) in same scope (matches Microcode kernel)
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
                        return Ok(Control::Return(val));
                    }
                    Control::None => {}
                }
            }

            if break_occurred {
                return Ok(Control::None);
            }

            // Check condition: exit if true, continue if false
            let cond = self.condition.eval(env)?;
            let cond_bool = as_bool(cond.as_ref())?;

            if cond_bool.value {
                // Condition is true, exit loop
                break;
            }
            // Condition is false, continue looping
        }
        Ok(Control::None)
    }
}

pub struct UntilStmtHandler;

impl StmtHandler for UntilStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == "until"
    }

    fn parse(
        &self,
        parser: &mut Parser,
        registry: &super::super::registry::Registry,
    ) -> LumenResult<Box<dyn StmtNode>> {
        parser.advance(); // consume 'until'
        parser.skip_tokens();

        // parse condition expression
        let condition = parser.parse_expr(registry)?;

        // parse indented body
        let body = structural::parse_block(parser, registry)?;

        Ok(Box::new(UntilStmt { condition, body }))
    }
}

// --------------------
// Pattern Declaration
// --------------------

/// Declare what patterns this module recognizes
pub fn patterns() -> PatternSet {
    PatternSet::new().with_literals(vec!["until"])
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    reg.register_stmt(Box::new(UntilStmtHandler));
}
