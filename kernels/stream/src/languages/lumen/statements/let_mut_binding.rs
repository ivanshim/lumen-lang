// Let mut binding statement (mutable)
// let mut name [: Type] = expression

use crate::languages::lumen::prelude::*;
use crate::kernel::ast::{Control, ExprNode, StmtNode};
use crate::kernel::parser::Parser;
use crate::kernel::runtime::Env;
use super::let_binding::parse_binding_tail;

#[derive(Debug)]
struct LetMutStmt {
    name: String,
    _type_annotation: Option<String>, // Optional type annotation
    expr: Box<dyn ExprNode>,
}

impl StmtNode for LetMutStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        let val = self.expr.eval(env)?;
        env.define(self.name.clone(), val);
        Ok(Control::None)
    }
}

pub struct LetMutStmtHandler;

impl StmtHandler for LetMutStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        if def().is("stmt.let", &parser.peek().lexeme) {
            // Look ahead for the mutable modifier
            if let Some(next) = parser.peek_n(1) {
                return def().is("stmt.let.mutable", &next.lexeme);
            }
        }
        false
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn StmtNode>> {
        let keyword = parser.advance().lexeme; // consume the binding keyword
        parser.skip_tokens();
        let modifier = parser.advance().lexeme; // consume the mutable modifier
        parser.skip_tokens();
        let context = format!("{} {}", keyword, modifier);
        let (name, _type_annotation, expr) = parse_binding_tail(parser, registry, &context)?;
        Ok(Box::new(LetMutStmt { name, _type_annotation, expr }))
    }
}

pub fn register(reg: &mut Registry) {
    reg.register_stmt(Box::new(LetMutStmtHandler));
}
