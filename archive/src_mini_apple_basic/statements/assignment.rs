// Assignment: LET x = expr
use crate::kernel::ast::{Control, ExprNode, StmtNode};
use crate::kernel::parser::Parser;
use crate::kernel::registry::{err_at, LumenResult, Registry, StmtHandler};
use crate::kernel::runtime::{Env, Value};

pub const LET: &str = "LET";
pub const ASSIGN: &str = "=";

#[derive(Debug)]
struct AssignStmt {
    name: String,
    expr: Box<dyn ExprNode>,
}
impl StmtNode for AssignStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        let val: Value = self.expr.eval(env)?;
        env.assign(&self.name, val)?;
        Ok(Control::None)
    }
}

pub struct AssignStmtHandler;
impl StmtHandler for AssignStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == LET
    }
    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn StmtNode>> {
        parser.advance(); // consume LET
        let name = parser.advance().lexeme;
        if parser.advance().lexeme != ASSIGN {
            return Err(err_at(parser, "Expected '=' after variable"));
        }
        let expr = parser.parse_expr()?;
        Ok(Box::new(AssignStmt { name, expr }))
    }
}

pub fn register(reg: &mut Registry) {    reg.register_stmt(Box::new(AssignStmtHandler));
}
