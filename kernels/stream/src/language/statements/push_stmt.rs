use crate::language::prelude::*;
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
            .downcast_mut::<crate::language::values::LumenArray>()
            .ok_or_else(|| format!("Variable '{}' is not an array", self.arr_name))?;
        arr.elements.push(value);

        Ok(Control::None)
    }
}

/// Method syntax for push where the pipe is spelled `.`: `arr.push(value)`.
/// The array must be a named variable, as with the call form, so the
/// statement is recognised from its shape: a name, the pipe, the push word
/// and the call bracket.
pub struct MethodPushHandler;

impl MethodPushHandler {
    /// How many tokens the name before the pipe spans, if the statement is a method push.
    fn shape(parser: &Parser) -> Option<usize> {
        let d = def();
        if !parser.at_identifier() {
            return None;
        }
        let prefix = d.first_char("identifier.variable_prefix");
        let mut n = 0;
        while parser.peek_n(n).map_or(false, |t| {
            let mut chars = t.lexeme.chars();
            matches!((chars.next(), chars.next()), (Some(c), None) if crate::language::word_char(c) || Some(c) == prefix)
        }) {
            n += 1;
        }
        let pipe = parser.peek_n(n)?;
        if !d.is("op.pipe", &pipe.lexeme) {
            return None;
        }
        let word = parser.peek_n(n + 1)?;
        if !d.is("builtin.push", &word.lexeme) {
            return None;
        }
        let bracket = parser.peek_n(n + 2)?;
        if !d.is("syntax.call.open", &bracket.lexeme) {
            return None;
        }
        Some(n)
    }
}

impl StmtHandler for MethodPushHandler {
    fn matches(&self, parser: &Parser) -> bool {
        Self::shape(parser).is_some()
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn StmtNode>> {
        let arr_name = parser.take_identifier().ok_or_else(|| err_at(parser, "Expected an array name"))?;
        parser.advance(); // the pipe
        parser.advance(); // the push word
        let mut args = crate::language::expressions::calls::parse_arguments(parser, registry, "after push")?;
        if args.len() != 1 {
            return Err(err_at(parser, "push takes one value"));
        }
        Ok(Box::new(PushStmt { arr_name, value_expr: args.remove(0) }))
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
    reg.register_stmt(Box::new(MethodPushHandler));
}
