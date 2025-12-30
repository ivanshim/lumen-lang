// src/stmt/assignment.rs
//
// Variable assignment statement: name = expr
// Fully removable language feature.

use crate::ast::{Control, ExprNode, StmtNode};
use crate::lexer::Token;
use crate::parser::Parser;
use crate::registry::{LumenResult, StmtHandler};
use crate::runtime::Env;

#[derive(Debug)]
struct AssignStmt {
    name: String,
    expr: Box<dyn ExprNode>,
}

impl StmtNode for AssignStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        let value = self.expr.eval(env)?;
        env.set(self.name.clone(), value);
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
            _ => unreachable!(),
        };

        match parser.advance() {
            Token::Equals => {}
            _ => return Err(parser.error("Expected '=' in assignment")),
        }

        let expr = parser.parse_expr()?;

        Ok(Box::new(AssignStmt { name, expr }))
    }
}
