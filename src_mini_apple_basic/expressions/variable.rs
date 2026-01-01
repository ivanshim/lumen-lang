// Variable reference
use crate::kernel::ast::ExprNode;
use crate::kernel::lexer::Token;
use crate::kernel::parser::Parser;
use crate::kernel::registry::{ExprPrefix, LumenResult, Registry};
use crate::kernel::runtime::{Env, Value};

#[derive(Debug)]
struct VarExpr { name: String }
impl ExprNode for VarExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        env.get(&self.name)
    }
}

pub struct VariablePrefix;
impl ExprPrefix for VariablePrefix {
    fn matches(&self, parser: &Parser) -> bool {
        matches!(parser.peek(), Token::Ident(_))
    }
    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        match parser.advance() {
            Token::Ident(name) => Ok(Box::new(VarExpr { name })),
            _ => unreachable!(),
        }
    }
}

pub fn register(reg: &mut Registry) {
    reg.register_prefix(Box::new(VariablePrefix));
}
