// MEMOIZATION system capability
//
// MEMOIZATION is a reserved system identifier that enables/disables function result caching.
//
// Syntax:
//   MEMOIZATION = true     (enable memoization)
//   MEMOIZATION = false    (disable memoization, default)
//
// Semantics:
// - MEMOIZATION is dynamically scoped
// - Affects all function calls made while enabled
// - Inherited by callees
// - Automatically restored on scope exit
// - NOT a normal variable (reserved system identifier)
// - NOT readable, passable, or storable as data
// - Reject any other assignment to MEMOIZATION
// - Reject attempts to read MEMOIZATION as a value

use crate::kernel::ast::{StmtNode, Control};
use crate::kernel::parser::Parser;
use crate::languages::lumen::prelude::*;
use crate::kernel::runtime::Env;

#[derive(Debug)]
struct MemoizationStmt {
    enabled: bool,
}

impl StmtNode for MemoizationStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        crate::languages::lumen::memo::set_enabled(env, self.enabled);
        Ok(Control::None)
    }
}

pub struct MemoizationHandler;

impl crate::languages::lumen::registry::StmtHandler for MemoizationHandler {
    fn matches(&self, parser: &Parser) -> bool {
        // The memoization switch is a reserved identifier
        def().is("system.memoization", &parser.peek().lexeme)
    }

    fn parse(&self, parser: &mut Parser, _registry: &super::super::registry::Registry) -> LumenResult<Box<dyn StmtNode>> {
        parser.advance(); // consume 'MEMOIZATION'

        // Skip whitespace to find '='
        loop {
            let lexeme = &parser.peek().lexeme;
            if lexeme.chars().count() == 1 {
                let ch = lexeme.chars().next().unwrap();
                if ch == ' ' || ch == '\t' || ch == '\n' || ch == '\r' {
                    parser.advance();
                    continue;
                }
            }
            break;
        }

        // Expect the assignment sign
        if !def().is("stmt.assign", &parser.peek().lexeme) {
            return Err(format!("Expected '{}' after {}", def().first("stmt.assign"), def().first("system.memoization")));
        }
        parser.advance(); // consume the assignment sign

        // Skip whitespace to find boolean value
        loop {
            let lexeme = &parser.peek().lexeme;
            if lexeme.chars().count() == 1 {
                let ch = lexeme.chars().next().unwrap();
                if ch == ' ' || ch == '\t' || ch == '\n' || ch == '\r' {
                    parser.advance();
                    continue;
                }
            }
            break;
        }

        // Expect a boolean literal word, lexed whole as a reserved word
        let value = parser.advance().lexeme;
        let enabled = if def().is("literal.true", &value) {
            true
        } else if def().is("literal.false", &value) {
            false
        } else {
            return Err(format!(
                "{} must be set to '{}' or '{}', got: {}",
                def().first("system.memoization"),
                def().first("literal.true"),
                def().first("literal.false"),
                value
            ));
        };

        Ok(Box::new(MemoizationStmt { enabled }))
    }
}

pub fn register(reg: &mut super::super::registry::Registry) {
    reg.register_stmt(Box::new(MemoizationHandler));
}
