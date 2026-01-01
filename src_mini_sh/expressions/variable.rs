// Variable reference with $ prefix
use crate::framework::ast::ExprNode;
use crate::framework::lexer::Token;
use crate::framework::parser::Parser;
use crate::framework::registry::{err_at, ExprPrefix, LumenResult, Registry};
use crate::framework::runtime::{Env, Value};
use crate::src_mini_sh::structure::structural::DOLLAR;

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
        matches!(parser.peek(), Token::Feature(DOLLAR))
    }
    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance(); // $
        match parser.advance() {
            Token::Ident(name) => Ok(Box::new(VarExpr { name })),
            _ => Err(err_at(parser, "Expected identifier after '$'")),
        }
    }
}

pub fn register(reg: &mut Registry) {
    reg.register_prefix(Box::new(VariablePrefix));
}
