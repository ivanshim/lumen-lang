// Expression statement handler
// Handles bare expressions as statements (for implicit returns and expression statements)

use crate::languages::lumen::prelude::*;
use crate::kernel::ast::{Control, StmtNode};
use crate::kernel::parser::Parser;
use crate::languages::lumen::patterns::PatternSet;
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

        // Don't match keywords that are handled by other statements
        let reserved = [
            "if", "else", "while", "break", "continue", "return",
            "fn", "let", "print", "extern"
        ];

        if reserved.contains(&lexeme.as_str()) {
            return false;
        }

        // Match if it could be the start of an expression:
        // - identifier
        // - literal (number, string, true, false, none)
        // - unary operator (-, not)
        // - grouping (
        if lexeme == "(" || lexeme == "-" || lexeme == "not" {
            return true;
        }

        if lexeme == "true" || lexeme == "false" || lexeme == "none" {
            return true;
        }

        // Check if it's an identifier or number literal
        if let Some(ch) = lexeme.chars().next() {
            if ch.is_alphabetic() || ch == '_' || ch.is_numeric() || ch == '"' || ch == '\'' {
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

pub fn patterns() -> PatternSet {
    PatternSet::new()
}

pub fn register(reg: &mut Registry) {
    // Register as lowest priority - should be tried last
    reg.register_stmt(Box::new(ExprStmtHandler));
}
