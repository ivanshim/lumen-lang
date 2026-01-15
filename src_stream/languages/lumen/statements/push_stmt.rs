use crate::languages::lumen::prelude::*;
// Push primitive: push(arr, value)
//
// Appends a value to an array, mutating it in place.
// This is a kernel-level primitive for array mutation.

use crate::kernel::ast::{Control, StmtNode};
use crate::kernel::parser::Parser;
use crate::languages::lumen::patterns::PatternSet;
use crate::kernel::runtime::Env;
use crate::languages::lumen::structure::structural::{LPAREN, RPAREN};

#[derive(Debug)]
struct PushStmt {
    arr_name: String,  // The variable name of the array
    value_expr: Box<dyn crate::kernel::ast::ExprNode>,
}

impl StmtNode for PushStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        // Evaluate the value to push
        let value = self.value_expr.eval(env)?;

        // Push to the array by name
        env.push_array(&self.arr_name, value)?;

        Ok(Control::None)
    }
}

pub struct PushStmtHandler;

impl StmtHandler for PushStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == "push"
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn StmtNode>> {
        // consume `push`
        parser.advance();
        parser.skip_tokens();

        // expect '('
        if parser.advance().lexeme != LPAREN {
            return Err("Expected '(' after push".into());
        }
        parser.skip_tokens();

        // Parse array name (must be an identifier)
        let mut arr_name = parser.advance().lexeme;
        parser.skip_tokens();

        // Continue consuming identifier characters if split across tokens
        loop {
            if parser.peek().lexeme.len() == 1 {
                let ch = parser.peek().lexeme.as_bytes()[0];
                if ch.is_ascii_alphanumeric() || ch == b'_' {
                    arr_name.push_str(&parser.advance().lexeme);
                    parser.skip_tokens();
                    continue;
                }
            }
            break;
        }

        // expect ','
        if parser.advance().lexeme != "," {
            return Err("Expected ',' after first argument to push".into());
        }
        parser.skip_tokens();

        let value_expr = parser.parse_expr(registry)?;
        parser.skip_tokens();

        // expect ')'
        if parser.advance().lexeme != RPAREN {
            return Err("Expected ')' after push arguments".into());
        }

        Ok(Box::new(PushStmt { arr_name, value_expr }))
    }
}

// --------------------
// Pattern Declaration
// --------------------

pub fn patterns() -> PatternSet {
    PatternSet::new()
        .with_literals(vec!["push", "(", ")", ","])
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    reg.register_stmt(Box::new(PushStmtHandler));
}
