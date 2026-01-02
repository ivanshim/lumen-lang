// src/stmt/while_loop.rs
//
// while <expr>
//     <block>

use crate::kernel::ast::{Control, ExprNode, StmtNode};
use crate::kernel::parser::Parser;
use crate::kernel::registry::{LumenResult, Registry, StmtHandler};
use crate::kernel::runtime::Env;
use crate::src_mini_python::structure::structural;

#[derive(Debug)]
struct WhileStmt {
    condition: Box<dyn ExprNode>,
    body: Vec<Box<dyn StmtNode>>,
}

impl StmtNode for WhileStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        loop {
            let cond = self.condition.eval(env)?;
            match cond {
                crate::kernel::runtime::Value::Bool(true) => {
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
                }
                crate::kernel::runtime::Value::Bool(false) => break,
                _ => return Err("while condition must be boolean".into()),
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
