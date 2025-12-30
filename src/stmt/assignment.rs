// src/stmt/assignment.rs
//
// x = expr

use crate::ast::{Control, ExprNode, StmtNode};
use crate::lexer::Token;
use crate::parser::Parser;
use crate::registry::{err_at, LumenResult, StmtHandler};
use crate::runtime::{Env, Value};

#[derive(Debug)]
struct AssignStmt {
    name: String,
    expr: Box<dyn ExprNode>,
}

impl StmtNode for AssignStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        let val: Value = self.expr.eval(env)?;
        env.set(self.name.clone(), val);
        Ok(Control::None)
    }
}

pub struct AssignStmtHandler;

impl StmtHandler for AssignStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        matches!(
            (parser.peek(), parser.peek_n(1)),
            (Token::Ident(_), Some(Token::Equals))
        )
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn StmtNode>> {
        let name = match parser.advance() {
            Token::Ident(s) => s,
            _ => return Err(err_at(parser, "Expected identifier in assignment")),
        };

        match parser.advance() {
            Token::Equals => {}
            _ => return Err(err_at(parser, "Expected '=' in assignment")),
        }

        let expr = parser.parse_expr()?;
        Ok(Box::new(AssignStmt { name, expr }))
    }
}
