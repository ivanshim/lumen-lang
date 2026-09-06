use crate::languages::lumen::prelude::*;
// Push primitive: push(arr, value)
//
// Appends a value to an array, mutating it in place.
// This is a kernel-level primitive for array mutation.

use crate::kernel::ast::{Control, StmtNode};
use crate::kernel::parser::Parser;
use crate::kernel::runtime::Env;

#[derive(Debug)]
struct PushStmt {
    arr_name: String,  // The variable name of the array
    value_expr: Box<dyn crate::kernel::ast::ExprNode>,
}

impl StmtNode for PushStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        // Evaluate the value to push
        let value = self.value_expr.eval(env)?;

        // Append in place; the array type is Lumen's, so the downcast lives here.
        let slot = env.get_mut(&self.arr_name).ok_or_else(|| format!("Undefined variable '{}'", self.arr_name))?;
        let arr = slot
            .as_any_mut()
            .downcast_mut::<crate::languages::lumen::values::LumenArray>()
            .ok_or_else(|| format!("Variable '{}' is not an array", self.arr_name))?;
        arr.elements.push(value);

        Ok(Control::None)
    }
}

pub struct PushStmtHandler;

impl StmtHandler for PushStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        def().is("builtin.push", &parser.peek().lexeme)
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn StmtNode>> {
        // consume `push`
        parser.advance();
        parser.skip_tokens();

        let d = def();
        // expect the call bracket
        if !d.is("syntax.call.open", &parser.advance().lexeme) {
            return Err(format!("Expected '{}' after {}", d.first("syntax.call.open"), d.first("builtin.push")));
        }
        parser.skip_tokens();

        // Parse array name (must be an identifier)
        let arr_name = parser
            .take_identifier()
            .ok_or_else(|| err_at(parser, "Expected an array name as the first argument to push"))?;
        parser.skip_tokens();

        // expect the separator
        if !d.is("syntax.call.separator", &parser.advance().lexeme) {
            return Err(format!("Expected '{}' after first argument to push", d.first("syntax.call.separator")));
        }
        parser.skip_tokens();

        let value_expr = parser.parse_expr(registry)?;
        parser.skip_tokens();

        // expect the closing bracket
        if !d.is("syntax.call.close", &parser.advance().lexeme) {
            return Err(format!("Expected '{}' after push arguments", d.first("syntax.call.close")));
        }

        Ok(Box::new(PushStmt { arr_name, value_expr }))
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    reg.register_stmt(Box::new(PushStmtHandler));
}
