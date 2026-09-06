// Expression statement handler
// Handles bare expressions as statements (for implicit returns and expression statements)

use crate::languages::lumen::prelude::*;
use crate::kernel::ast::{Control, StmtNode};
use crate::kernel::parser::Parser;
use crate::kernel::runtime::Env;

#[derive(Debug)]
struct ExprStmt {
    expr: Box<dyn ExprNode>,
}

impl StmtNode for ExprStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        let val = self.expr.eval(env)?;
        // Expression statements return their value as ExprValue.
        // This allows function bodies to continue executing multiple statements,
        // while explicit return statements break the loop immediately.
        Ok(Control::ExprValue(val))
    }
}

pub struct ExprStmtHandler;

impl StmtHandler for ExprStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        // This is a fallback handler - it matches any token that could start an expression
        let lexeme = &parser.peek().lexeme;

        let d = def();

        // Don't match keywords that are handled by other statements
        let statement_words = [
            "stmt.if", "stmt.else", "stmt.while", "stmt.until", "stmt.for", "stmt.break",
            "stmt.continue", "stmt.return", "stmt.function", "stmt.let", "builtin.extern",
        ];
        if statement_words.iter().any(|label| d.is(label, lexeme)) {
            return false;
        }

        // Match if it could be the start of an expression:
        // - unary operator, grouping, array literal, literal word
        let expression_words = [
            "syntax.group.open", "syntax.array.open", "op.negate", "op.not",
            "literal.true", "literal.false", "literal.null",
        ];
        if expression_words.iter().any(|label| d.is(label, lexeme)) {
            return true;
        }

        // Check if it's an identifier, number literal or string literal
        if let Some(ch) = lexeme.chars().next() {
            if word_start(ch) || ch.is_numeric() || d.chars("lexical.string_quotes").contains(&ch) {
                return true;
            }
        }

        false
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn StmtNode>> {
        let expr = parser.parse_expr(registry)?;
        Ok(Box::new(ExprStmt { expr }))
    }
}

pub fn register(reg: &mut Registry) {
    // Register as lowest priority - should be tried last
    reg.register_stmt(Box::new(ExprStmtHandler));
}
