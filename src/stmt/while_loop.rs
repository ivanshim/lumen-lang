// src/stmt/while_loop.rs
//
// while <condition>
//     <block>

use crate::ast::{Control, ExprNode, StmtNode};
use crate::lexer::Token;
use crate::parser::Parser;
use crate::registry::{LumenResult, StmtHandler};
use crate::runtime::Env;

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
                            Control::None => {}
                            Control::Break => return Ok(Control::None),
                            Control::Continue => break,
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
        matches!(parser.peek(), Token::While)
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn StmtNode>> {
        // consume `while`
        parser.advance();

        // parse condition expression
        let condition = parser.parse_expr()?;

        // allow newline before block
        parser.consume_newlines();

        // parse indented block
        let body = parser.parse_block()?;

        Ok(Box::new(WhileStmt { condition, body }))
    }
}
