// src/stmt/while_loop.rs
//
// while <expr>
//     <block>

use crate::src_stream::kernel::ast::{Control, ExprNode, StmtNode};
use crate::src_stream::kernel::parser::Parser;
use crate::src_stream::kernel::registry::{LumenResult, Registry, StmtHandler};
use crate::src_stream::kernel::runtime::Env;
use crate::src_stream::src_mini_python::structure::structural;
use crate::src_stream::src_mini_python::values::as_bool;

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
        parser.peek().lexeme == "while"
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn StmtNode>> {
        parser.advance(); // consume 'while'

        // parse condition expression
        let condition = parser.parse_expr()?;

        // parse indented body
        let body = structural::parse_block(parser)?;

        Ok(Box::new(WhileStmt { condition, body }))
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // No tokens to register (uses "while" keyword registered in dispatcher)
    // Register handlers
    reg.register_stmt(Box::new(WhileStmtHandler));
}
