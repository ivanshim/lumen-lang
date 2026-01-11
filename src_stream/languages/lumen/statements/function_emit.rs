use crate::languages::lumen::prelude::*;
// Primitive emit() function
//
// emit(string) - kernel-level output primitive
//
// This is the ONLY side-effectful I/O operation in the kernel.
// It accepts a string only and writes it directly to stdout.
// No formatting, conversion, newline handling, or implicit stringification.
// All higher-level I/O behavior (write, print, etc) is implemented in the
// Lumen standard library using emit() as the foundation.

use crate::kernel::ast::{Control, ExprNode, StmtNode};
use crate::kernel::parser::Parser;
use crate::languages::lumen::patterns::PatternSet;
use crate::kernel::runtime::Env;
use crate::languages::lumen::structure::structural::{LPAREN, RPAREN};
use crate::languages::lumen::values::as_string;

#[derive(Debug)]
struct EmitStmt {
    expr: Box<dyn ExprNode>,
}

impl StmtNode for EmitStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        let val = self.expr.eval(env)?;

        // Require string input - no implicit conversion
        match as_string(val.as_ref()) {
            Ok(str_val) => {
                print!("{}", str_val.value);
                Ok(Control::None)
            }
            Err(_) => Err("emit() requires a string argument".into()),
        }
    }
}

pub struct EmitStmtHandler;

impl StmtHandler for EmitStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == "emit"
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn StmtNode>> {
        // consume `emit`
        parser.advance();
        parser.skip_tokens();

        // expect '('
        if parser.advance().lexeme != LPAREN {
            return Err("Expected '(' after emit".into());
        }
        parser.skip_tokens();

        let expr = parser.parse_expr(registry)?;
        parser.skip_tokens();

        // expect ')'
        if parser.advance().lexeme != RPAREN {
            return Err("Expected ')' after expression".into());
        }

        Ok(Box::new(EmitStmt { expr }))
    }
}

// --------------------
// Pattern Declaration
// --------------------

/// Declare what patterns this module recognizes
pub fn patterns() -> PatternSet {
    PatternSet::new()
        .with_literals(vec!["emit", "(", ")"])
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // No tokens to register (uses "emit" keyword registered in dispatcher)
    // Register handler
    reg.register_stmt(Box::new(EmitStmtHandler));
}
