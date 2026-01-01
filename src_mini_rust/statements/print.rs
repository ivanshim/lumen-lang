// print!() statement for mini-rust

use crate::framework::ast::{Control, ExprNode, StmtNode};
use crate::framework::lexer::Token;
use crate::framework::parser::Parser;
use crate::framework::registry::{LumenResult, Registry, StmtHandler};
use crate::framework::runtime::Env;
use crate::src_mini_rust::structure::structural::{LPAREN, RPAREN};

pub const PRINT: &str = "PRINT";

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
        matches!(
            parser.peek(),
            Token::Feature(PRINT)
        )
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn StmtNode>> {
        parser.advance(); // consume 'print!'

        // expect '('
        match parser.advance() {
            Token::Feature(k) if k == LPAREN => {}
            _ => return Err("Expected '(' after print!".into()),
        }

        let expr = parser.parse_expr()?;

        // expect ')'
        match parser.advance() {
            Token::Feature(k) if k == RPAREN => {}
            _ => return Err("Expected ')' after expression".into()),
        }

        Ok(Box::new(PrintStmt { expr }))
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // Register tokens
    reg.tokens.add_keyword("print", PRINT);

    // Register handlers
    reg.register_stmt(Box::new(PrintStmtHandler));
}
