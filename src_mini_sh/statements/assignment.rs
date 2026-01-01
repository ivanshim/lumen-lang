// Assignment: x=5 (no $ on left side)
use crate::kernel::ast::{Control, ExprNode, StmtNode};
use crate::kernel::lexer::Token;
use crate::kernel::parser::Parser;
use crate::kernel::registry::{err_at, LumenResult, Registry, StmtHandler};
use crate::kernel::runtime::{Env, Value};

pub const ASSIGN: &str = "ASSIGN";

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
        matches!(parser.peek(), Token::Ident(_))
            && matches!(parser.peek_n(1), Some(Token::Feature(ASSIGN)))
    }
    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn StmtNode>> {
        let name = match parser.advance() {
            Token::Ident(s) => s,
            _ => unreachable!(),
        };
        match parser.advance() {
            Token::Feature(ASSIGN) => {}
            _ => return Err(err_at(parser, "Expected '='")),
        }
        let expr = parser.parse_expr()?;
        Ok(Box::new(AssignStmt { name, expr }))
    }
}

pub fn register(reg: &mut Registry) {
    reg.tokens.add_single_char('=', ASSIGN);
    reg.register_stmt(Box::new(AssignStmtHandler));
}
