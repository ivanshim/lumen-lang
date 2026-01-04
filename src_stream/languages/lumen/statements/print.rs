// src/stmt/print.rs
//
// print(expr)

use crate::kernel::ast::{Control, ExprNode, StmtNode};
use crate::kernel::parser::Parser;
use crate::kernel::patterns::PatternSet;
use crate::kernel::registry::{LumenResult, Registry, StmtHandler};
use crate::kernel::runtime::Env;
use crate::languages::lumen::structure::structural::{LPAREN, RPAREN};

#[derive(Debug)]
struct PrintStmt {
    expr: Box<dyn ExprNode>,
}

impl StmtNode for PrintStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        let val = self.expr.eval(env)?;
        println!("{val}");
        Ok(Control::None)
    }
}

pub struct PrintStmtHandler;

impl StmtHandler for PrintStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == "print"
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn StmtNode>> {
        // consume `print`
        parser.advance();
        parser.skip_whitespace();

        // expect '('
        if parser.advance().lexeme != LPAREN {
            return Err("Expected '(' after print".into());
        }
        parser.skip_whitespace();

        let expr = parser.parse_expr()?;
        parser.skip_whitespace();

        // expect ')'
        if parser.advance().lexeme != RPAREN {
            return Err("Expected ')' after expression".into());
        }

        Ok(Box::new(PrintStmt { expr }))
    }
}

// --------------------
// Pattern Declaration
// --------------------

/// Declare what patterns this module recognizes
pub fn patterns() -> PatternSet {
    PatternSet::new()
        .with_literals(vec!["print", "(", ")"])
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // No tokens to register (uses "print" keyword registered in dispatcher)
    // Register handlers
    reg.register_stmt(Box::new(PrintStmtHandler));
}
