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
use crate::kernel::runtime::Env;
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
        def().is("builtin.emit", &parser.peek().lexeme)
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn StmtNode>> {
        // consume `emit`
        parser.advance();
        parser.skip_tokens();

        // expect the call bracket
        if !def().is("syntax.call.open", &parser.advance().lexeme) {
            return Err(format!("Expected '{}' after {}", def().first("syntax.call.open"), def().first("builtin.emit")));
        }
        parser.skip_tokens();

        let expr = parser.parse_expr(registry)?;
        parser.skip_tokens();

        // expect the closing bracket
        if !def().is("syntax.call.close", &parser.advance().lexeme) {
            return Err(format!("Expected '{}' after expression", def().first("syntax.call.close")));
        }

        Ok(Box::new(EmitStmt { expr }))
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // No tokens to register (uses "emit" keyword registered in dispatcher)
    // Register handler
    reg.register_stmt(Box::new(EmitStmtHandler));
}
