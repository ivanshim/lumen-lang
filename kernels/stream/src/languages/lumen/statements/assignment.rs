use crate::languages::lumen::prelude::*;
// src/stmt/assignment.rs
//
// x = expr

use crate::kernel::ast::{Control, ExprNode, StmtNode};
use crate::kernel::parser::Parser;
use crate::kernel::runtime::{Env, Value};

#[derive(Debug)]
struct AssignStmt {
    name: String,
    expr: Box<dyn ExprNode>,
}

impl StmtNode for AssignStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        // The arguments binding is a system-provided immutable value
        if def().is("system.args", &self.name) {
            return Err(format!("Cannot reassign {} (system-provided immutable value)", self.name));
        }
        let val: Value = self.expr.eval(env)?;
        env.assign(&self.name, val)?;
        Ok(Control::None)
    }
}

pub struct AssignStmtHandler;

impl StmtHandler for AssignStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        if !parser.at_identifier() {
            return false;
        }

        // Look ahead for the assignment sign, skipping whitespace tokens and
        // the single-character tokens that continue the identifier.
        let mut i = 1;
        while let Some(t) = parser.peek_n(i) {
            let lexeme = &t.lexeme;
            if lexeme.chars().count() == 1 {
                let ch = lexeme.chars().next().unwrap();
                if ch == ' ' || ch == '\t' || ch == '\n' || ch == '\r' || word_char(ch) {
                    i += 1;
                    continue;
                }
            }
            return def().is("stmt.assign", lexeme);
        }

        false
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn StmtNode>> {
        let name = parser.take_identifier().ok_or_else(|| err_at(parser, "Expected identifier"))?;
        parser.skip_tokens();

        if !def().is("stmt.assign", &parser.advance().lexeme) {
            return Err(err_at(parser, &format!("Expected '{}' in assignment", def().first("stmt.assign"))));
        }
        parser.skip_tokens();

        let expr = parser.parse_expr(registry)?;
        Ok(Box::new(AssignStmt { name, expr }))
    }
}

pub fn register(reg: &mut Registry) {
    reg.register_stmt(Box::new(AssignStmtHandler));
}
