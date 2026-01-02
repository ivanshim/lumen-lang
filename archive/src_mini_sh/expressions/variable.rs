// Variable reference with $ prefix
use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;
use crate::kernel::registry::{err_at, ExprPrefix, LumenResult, Registry};
use crate::kernel::runtime::{Env, Value};
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
        parser.peek().lexeme == DOLLAR
    }
    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance(); // $
        let name = parser.advance().lexeme;
        Ok(Box::new(VarExpr { name }))
    }
}

pub fn register(reg: &mut Registry) {
    // No token registration needed - kernel handles all segmentation
    reg.register_prefix(Box::new(VariablePrefix));
}
