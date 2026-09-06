// Return statement handler
// return [expression]

use crate::languages::lumen::prelude::*;
use crate::kernel::ast::{Control, ExprNode, StmtNode};
use crate::kernel::parser::Parser;
use crate::kernel::runtime::Env;

#[derive(Debug)]
struct ReturnStmt {
    value: Option<Box<dyn ExprNode>>,
}

impl StmtNode for ReturnStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        let val = if let Some(ref expr) = self.value {
            expr.eval(env)?
        } else {
            Box::new(crate::languages::lumen::values::LumenNull) as crate::kernel::runtime::Value
        };

        Ok(Control::Return(val))
    }
}

pub struct ReturnStmtHandler;

impl StmtHandler for ReturnStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        def().is("stmt.return", &parser.peek().lexeme)
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn StmtNode>> {
        parser.advance(); // consume 'return'
        parser.skip_tokens();

        // Check if there's an expression after return (before the line ends)
        use crate::languages::lumen::structure::structural::{DEDENT, EOF, NEWLINE};
        let at_line_end = matches!(parser.peek().lexeme.as_str(), NEWLINE | DEDENT | EOF);
        let value = if at_line_end || parser.i >= parser.toks.len() {
            None
        } else {
            Some(parser.parse_expr(registry)?)
        };

        Ok(Box::new(ReturnStmt { value }))
    }
}

pub fn register(reg: &mut Registry) {
    reg.register_stmt(Box::new(ReturnStmtHandler));
}
