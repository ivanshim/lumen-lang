// src/stmt/while_loop.rs
//
// while <expr>
//     <block>

use crate::ast::{Control, ExprNode, StmtNode};
use crate::lexer::Token;
use crate::parser::Parser;
use crate::registry::{LumenResult, StmtHandler};
use crate::runtime::Env;

#[derive(Debug)]
struct WhileStmt {
    cond: Box<dyn ExprNode>,
    body: Vec<Box<dyn StmtNode>>,
}

impl StmtNode for WhileStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        loop {
            let v = self.cond.eval(env)?;
            if !matches!(v, crate::runtime::Value::Bool(true)) {
                break;
            }

            for stmt in &self.body {
                match stmt.exec(env)? {
                    Control::None => {}
                    Control::Break => return Ok(Control::None),
                    Control::Continue => break,
                }
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
        let cond = parser.parse_expr()?;

        // NEWLINE is optional but common
        if matches!(parser.peek(), Token::Newline) {
            parser.advance();
        }

        // parse indented block
        let body = parser.parse_block()?;

        Ok(Box::new(WhileStmt { cond, body }))
    }
}
