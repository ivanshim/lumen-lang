// Let binding statement (immutable)
// let name [: Type] = expression

use crate::language::prelude::*;
use crate::kernel::ast::{Control, ExprNode, StmtNode};
use crate::kernel::parser::Parser;
use crate::kernel::runtime::{Env, Value};

/// The value of a declaration that gives none.
#[derive(Debug)]
struct NullValue;

impl ExprNode for NullValue {
    fn eval(&self, _env: &mut Env) -> LumenResult<Value> {
        Ok(Box::new(crate::language::values::LumenNull))
    }
}

#[derive(Debug)]
struct LetStmt {
    name: String,
    _type_annotation: Option<String>, // Optional type annotation
    expr: Box<dyn ExprNode>,
}

impl StmtNode for LetStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        let val = self.expr.eval(env)?;
        env.define(self.name.clone(), val);
        Ok(Control::None)
    }
}

/// After the binding keyword: the name, an optional annotation, the
/// assignment sign and the expression. Shared with the mutable binding.
pub fn parse_binding_tail(
    parser: &mut Parser,
    registry: &Registry,
    context: &str,
) -> LumenResult<(String, Option<String>, Box<dyn ExprNode>)> {
    let d = def();
    let name = parser
        .take_identifier()
        .ok_or_else(|| err_at(parser, &format!("Expected identifier after '{}'", context)))?;
    parser.skip_tokens();

    // Parse optional type annotation ": Type"
    let type_annotation = if d.is("stmt.let.annotation", &parser.peek().lexeme) {
        parser.advance(); // consume the annotation mark
        parser.skip_tokens();
        let type_name = parser.take_word().unwrap_or_default();
        parser.skip_tokens();
        Some(type_name)
    } else {
        None
    };

    // A declaration with a type and no value binds null (Pascal's `var x: integer;`)
    if type_annotation.is_some() && crate::language::structure::structural::at_statement_end(parser) {
        let null: Box<dyn ExprNode> = Box::new(NullValue);
        return Ok((name, type_annotation, null));
    }

    // Expect the assignment sign
    if !d.is("stmt.assign", &parser.advance().lexeme) {
        return Err(err_at(parser, &format!("Expected '{}' in {} binding", d.first("stmt.assign"), context)));
    }
    parser.skip_tokens();

    let expr = parser.parse_expr(registry)?;
    Ok((name, type_annotation, expr))
}

pub struct LetStmtHandler;

impl StmtHandler for LetStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        def().is("stmt.let", &parser.peek().lexeme)
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn StmtNode>> {
        let keyword = parser.advance().lexeme; // consume the binding keyword
        parser.skip_tokens();
        let (name, _type_annotation, expr) = parse_binding_tail(parser, registry, &keyword)?;
        Ok(Box::new(LetStmt { name, _type_annotation, expr }))
    }
}

pub fn register(reg: &mut Registry) {
    reg.register_stmt(Box::new(LetStmtHandler));
}
