// Mini-PHP: while loop

use crate::kernel::ast::{Control, ExprNode, StmtNode};
use crate::kernel::lexer::Token;
use crate::kernel::parser::Parser;
use crate::kernel::registry::{err_at, LumenResult, Registry, StmtHandler};
use crate::kernel::runtime::Env;
use crate::src_mini_php::structure::structural;

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
            match cond {
                crate::kernel::runtime::Value::Bool(true) => {
                    for stmt in &self.body {
                        match stmt.exec(env)? {
                            Control::Break => return Ok(Control::None),
                            Control::Continue => break,
                            Control::None => {}
                        }
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
        parser.peek().lexeme == WHILE
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn StmtNode>> {
        parser.advance(); // consume 'while'
        if parser.advance().lexeme != structural::LPAREN {
            return Err(err_at(parser, "Expected '(' after 'while'"));
        }
        let condition = parser.parse_expr()?;
        if parser.advance().lexeme != structural::RPAREN {
            return Err(err_at(parser, "Expected ')' after condition"));
        }
        let body = structural::parse_block(parser)?;
        Ok(Box::new(WhileStmt { condition, body }))
    }
}

pub fn register(reg: &mut Registry) {    reg.register_stmt(Box::new(WhileStmtHandler));
}
