// if / else statement

use crate::ast::{Control, ExprNode, StmtNode};
use crate::lexer::Token;
use crate::parser::Parser;
use crate::registry::{LumenResult, StmtHandler};
use crate::runtime::Env;

#[derive(Debug)]
struct IfStmt {
    cond: Box<dyn ExprNode>,
    then_block: Vec<Box<dyn StmtNode>>,
    else_block: Option<Vec<Box<dyn StmtNode>>>,
}

impl StmtNode for IfStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        let cond = self.cond.eval(env)?;
        match cond {
            crate::runtime::Value::Bool(true) => {
                for s in &self.then_block {
                    s.exec(env)?;
                }
            }
            crate::runtime::Value::Bool(false) => {
                if let Some(block) = &self.else_block {
                    for s in block {
                        s.exec(env)?;
                    }
                }
            }
            _ => return Err("if condition must be boolean".into()),
        }
        Ok(Control::None)
    }
}

pub struct IfStmtHandler;

impl StmtHandler for IfStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        matches!(parser.peek(), Token::If)
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn StmtNode>> {
        parser.advance(); // if
        let cond = parser.parse_expr()?;
        let then_block = parser.parse_block()?;

        let else_block = if matches!(parser.peek(), Token::Else) {
            parser.advance();
            Some(parser.parse_block()?)
        } else {
            None
        };

        Ok(Box::new(IfStmt {
            cond,
            then_block,
            else_block,
        }))
    }
}
