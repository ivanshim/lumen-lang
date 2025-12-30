// src/stmt/assignment.rs
//
// assignment:  name = expr

use crate::ast::{Control, StmtNode};
use crate::lexer::Token;
use crate::parser::Parser;
use crate::registry::{err_at, LumenResult, StmtHandler};
use crate::runtime::Env;

#[derive(Debug)]
struct AssignStmt {
    name: String,
    expr: Box<dyn crate::ast::ExprNode>,
}

impl StmtNode for AssignStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        let val = self.expr.eval(env)?;
        env.set(&self.name, val);
        Ok(Control::None)
    }
}

pub struct AssignStmtHandler;

impl StmtHandler for AssignStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        matches!(parser.peek(), Token::Ident(_))
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn StmtNode>> {
        // identifier
        let name = match parser.advance() {
            Token::Ident(s) => s,
            _ => return Err(err_at(parser, "Expected identifier")),
        };

        // =
        match parser.advance() {
            Token::Eq => {}
            _ => return Err(err_at(parser, "Expected '=' in assignment")),
        }

        // expression
        let expr = parser.parse_expr()?;

        Ok(Box::new(AssignStmt { name, expr }))
    }
}
