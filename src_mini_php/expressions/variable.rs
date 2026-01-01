// Mini-PHP: Variable reference with $ prefix

use crate::kernel::ast::ExprNode;
use crate::kernel::lexer::Token;
use crate::kernel::parser::Parser;
use crate::kernel::registry::{err_at, ExprPrefix, LumenResult, Registry};
use crate::kernel::runtime::{Env, Value};
use crate::src_mini_php::structure::structural::DOLLAR;

#[derive(Debug)]
struct VarExpr {
    name: String,
}

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
        parser.advance(); // consume $
        match parser.advance() {
            Token::Ident(name) => Ok(Box::new(VarExpr { name })),
            _ => Err(err_at(parser, "Expected identifier after '$'")),
        }
    }
}

pub fn register(reg: &mut Registry) {
    reg.register_prefix(Box::new(VariablePrefix));
}
