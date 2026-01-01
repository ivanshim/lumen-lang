// let binding statement for mini-rust

use crate::framework::ast::{Control, ExprNode, StmtNode};
use crate::framework::lexer::Token;
use crate::framework::parser::Parser;
use crate::framework::registry::{LumenResult, Registry, StmtHandler};
use crate::framework::runtime::{Env, Value};

// --------------------
// Token definitions
// --------------------

pub const LET: &str = "LET";
pub const EQUALS: &str = "EQUALS";

#[derive(Debug)]
struct LetStmt {
    name: String,
    expr: Box<dyn ExprNode>,
}

impl StmtNode for LetStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        let val: Value = self.expr.eval(env)?;
        env.set(self.name.clone(), val);
        Ok(Control::None)
    }
}

pub struct LetStmtHandler;

impl StmtHandler for LetStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        matches!(parser.peek(), Token::Feature(LET))
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn StmtNode>> {
        parser.advance(); // consume 'let'

        let name = match parser.advance() {
            Token::Ident(s) => s,
            _ => return Err("Expected identifier after 'let'".into()),
        };

        match parser.advance() {
            Token::Feature(EQUALS) => {}
            _ => return Err("Expected '=' after identifier".into()),
        }

        let expr = parser.parse_expr()?;
        Ok(Box::new(LetStmt { name, expr }))
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // Register tokens
    reg.tokens.add_keyword("let", LET);
    // EQUALS is registered in assignment.rs

    // Register handlers
    reg.register_stmt(Box::new(LetStmtHandler));
}
