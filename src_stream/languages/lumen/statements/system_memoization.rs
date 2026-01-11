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
use crate::languages::lumen::patterns::PatternSet;
use crate::kernel::runtime::Env;
use crate::languages::lumen::structure::structural::LPAREN;

#[derive(Debug)]
struct MemoizationStmt {
    enabled: bool,
}

impl StmtNode for MemoizationStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        env.set_memoization(self.enabled);
        Ok(Control::None)
    }
}

pub struct MemoizationHandler;

impl crate::languages::lumen::registry::StmtHandler for MemoizationHandler {
    fn matches(&self, parser: &Parser) -> bool {
        // Check for MEMOIZATION keyword (reserved identifier)
        parser.peek().lexeme == "MEMOIZATION"
    }

    fn parse(&self, parser: &mut Parser, _registry: &super::super::registry::Registry) -> LumenResult<Box<dyn StmtNode>> {
        parser.advance(); // consume 'MEMOIZATION'

        // Skip whitespace to find '='
        loop {
            let lexeme = &parser.peek().lexeme;
            if lexeme.len() == 1 {
                let ch = lexeme.as_bytes()[0];
                if ch == b' ' || ch == b'\t' || ch == b'\n' || ch == b'\r' {
                    parser.advance();
                    continue;
                }
            }
            break;
        }

        // Expect '='
        if parser.peek().lexeme != "=" {
            return Err("Expected '=' after MEMOIZATION".into());
        }
        parser.advance(); // consume '='

        // Skip whitespace to find boolean value
        loop {
            let lexeme = &parser.peek().lexeme;
            if lexeme.len() == 1 {
                let ch = lexeme.as_bytes()[0];
                if ch == b' ' || ch == b'\t' || ch == b'\n' || ch == b'\r' {
                    parser.advance();
                    continue;
                }
            }
            break;
        }

        // Expect 'true' or 'false' (kernel lexer may split multi-char identifiers)
        let mut value = String::new();
        // Consume identifier characters
        loop {
            let lexeme = parser.peek().lexeme.clone();
            if lexeme.len() == 1 {
                let ch = lexeme.as_bytes()[0];
                if ch.is_ascii_alphabetic() || ch == b'_' {
                    value.push_str(&lexeme);
                    parser.advance();
                    continue;
                }
            }
            break;
        }

        let enabled = match value.as_str() {
            "true" => true,
            "false" => false,
            _ => {
                return Err(format!("MEMOIZATION must be set to 'true' or 'false', got: {}", value));
            }
        };

        Ok(Box::new(MemoizationStmt { enabled }))
    }
}

pub fn patterns() -> PatternSet {
    PatternSet::new()
        .with_literals(vec!["MEMOIZATION"])
}

pub fn register(reg: &mut super::super::registry::Registry) {
    reg.register_stmt(Box::new(MemoizationHandler));
}
