// src/stmt/while_loop.rs
//
// while <expr>
//     <block>

use crate::ast::{Control, ExprNode, StmtNode};
use crate::lexer::Token;
use crate::parser::Parser;
use crate::registry::{LumenResult, Registry, StmtHandler};
use crate::runtime::Env;

// --------------------
// Token definitions
// --------------------

pub const WHILE: &str = "WHILE";

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
                crate::runtime::Value::Bool(true) => {
                    for stmt in &self.body {
                        match stmt.exec(env)? {
                            Control::Break => return Ok(Control::None),
                            Control::Continue => break,
                            Control::None => {}
                        }
                    }
                }
                crate::runtime::Value::Bool(false) => break,
                _ => return Err("while condition must be boolean".into()),
            }
        }
        Ok(Control::None)
    }
}

pub struct WhileStmtHandler;

impl StmtHandler for WhileStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        matches!(parser.peek(), Token::Feature(WHILE))
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn StmtNode>> {
        parser.advance(); // consume 'while'

        // parse condition expression
        let condition = parser.parse_expr()?;

        // parse indented body
        let body = parser.parse_block()?;

        Ok(Box::new(WhileStmt { condition, body }))
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // Register tokens
    reg.tokens.add_keyword("while", WHILE);

    // Register handlers
    reg.register_stmt(Box::new(WhileStmtHandler));
}
