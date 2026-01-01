// Assignment statement: x = expr

use crate::kernel::ast::{Control, ExprNode, StmtNode};
use crate::kernel::lexer::Token;
use crate::kernel::parser::Parser;
use crate::kernel::registry::{err_at, LumenResult, Registry, StmtHandler};
use crate::kernel::runtime::{Env, Value};

// --------------------
// Token definitions
// --------------------

pub const EQUALS: &str = "EQUALS";

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
            (Token::Ident(_), Some(Token::Feature(EQUALS)))
        )
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn StmtNode>> {
        let name = match parser.advance() {
            Token::Ident(s) => s,
            _ => unreachable!(),
        };

        match parser.advance() {
            Token::Feature(EQUALS) => {}
            _ => return Err(err_at(parser, "Expected '=' in assignment")),
        }

        let expr = parser.parse_expr()?;
        Ok(Box::new(AssignStmt { name, expr }))
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // Register tokens
    reg.tokens.add_single_char('=', EQUALS);

    // Register handlers
    reg.register_stmt(Box::new(AssignStmtHandler));
}
