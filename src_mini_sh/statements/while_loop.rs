// while loop
use crate::framework::ast::{Control, ExprNode, StmtNode};
use crate::framework::lexer::Token;
use crate::framework::parser::Parser;
use crate::framework::registry::{err_at, LumenResult, Registry, StmtHandler};
use crate::framework::runtime::Env;
use crate::src_mini_sh::structure::structural;

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
                crate::framework::runtime::Value::Bool(true) => {
                    for stmt in &self.body {
                        match stmt.exec(env)? {
                            Control::Break => return Ok(Control::None),
                            Control::Continue => break,
                            Control::None => {}
                        }
                    }
                }
                crate::framework::runtime::Value::Bool(false) => break,
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
        match parser.advance() {
            Token::Feature(structural::LPAREN) => {}
            _ => return Err(err_at(parser, "Expected '(' after 'while'")),
        }
        let condition = parser.parse_expr()?;
        match parser.advance() {
            Token::Feature(structural::RPAREN) => {}
            _ => return Err(err_at(parser, "Expected ')' after condition")),
        }
        let body = structural::parse_block(parser)?;
        Ok(Box::new(WhileStmt { condition, body }))
    }
}

pub fn register(reg: &mut Registry) {
    reg.tokens.add_keyword("while", WHILE);
    reg.register_stmt(Box::new(WhileStmtHandler));
}
