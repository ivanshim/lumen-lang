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
    condition: Box<dyn ExprNode>,
    body: Vec<Box<dyn StmtNode>>,
}

impl StmtNode for WhileStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        loop {
            let cond = self.condition.eval(env)?;
            if !cond.is_truthy() {
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
        let condition = parser.parse_expr()?;

        // condition must end at newline
        match parser.peek() {
            Token::Newline => {
                parser.advance();
            }
            _ => return Err("Expected newline after while condition".into()),
        }

        // parse indented body
        let body = parser.parse_block()?;

        Ok(Box::new(WhileStmt { condition, body }))
    }
}
