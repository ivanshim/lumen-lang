// src/stmt/break_stmt.rs
//
// break statement

use crate::kernel::ast::{Control, StmtNode};
use crate::kernel::parser::Parser;
use crate::kernel::patterns::PatternSet;
use crate::kernel::registry::{LumenResult, Registry, StmtHandler};
use crate::kernel::runtime::Env;

#[derive(Debug)]
struct BreakStmt;

impl StmtNode for BreakStmt {
    fn exec(&self, _env: &mut Env) -> LumenResult<Control> {
        Ok(Control::Break)
    }
}

pub struct BreakStmtHandler;

impl StmtHandler for BreakStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == "break"
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn StmtNode>> {
        parser.advance(); // consume 'break'
        Ok(Box::new(BreakStmt))
    }
}

// --------------------
// Pattern Declaration
// --------------------

/// Declare what patterns this module recognizes
pub fn patterns() -> PatternSet {
    PatternSet::new()
        .with_literals(vec!["break"])
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // No tokens to register (uses "break" keyword registered in dispatcher)
    // Register handlers
    reg.register_stmt(Box::new(BreakStmtHandler));
}
