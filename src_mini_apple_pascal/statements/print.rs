// writeln statement
use crate::kernel::ast::{Control, ExprNode, StmtNode};
use crate::kernel::lexer::Token;
use crate::kernel::parser::Parser;
use crate::kernel::registry::{err_at, LumenResult, Registry, StmtHandler};
use crate::kernel::runtime::{Env, Value};
use crate::src_mini_apple_pascal::structure::structural::{LPAREN, RPAREN};

pub const WRITELN: &str = "WRITELN";

#[derive(Debug)]
struct PrintStmt {
    expr: Box<dyn ExprNode>,
}
impl StmtNode for PrintStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        let value = self.expr.eval(env)?;
        match value {
            Value::Number(s) => println!("{}", s),
            Value::Bool(b) => println!("{}", b),
        }
        Ok(Control::None)
    }
}

pub struct PrintStmtHandler;
impl StmtHandler for PrintStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        matches!(parser.peek(), Token::Feature(WRITELN))
    }
    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn StmtNode>> {
        parser.advance(); // consume keyword
        match parser.advance() {
            Token::Feature(LPAREN) => {}
            _ => return Err(err_at(parser, "Expected '(' after 'writeln'")),
        }
        let expr = parser.parse_expr()?;
        match parser.advance() {
            Token::Feature(RPAREN) => {}
            _ => return Err(err_at(parser, "Expected ')'")),
        }
        Ok(Box::new(PrintStmt { expr }))
    }
}

pub fn register(reg: &mut Registry) {
    reg.tokens.add_keyword("writeln", WRITELN);
    reg.register_stmt(Box::new(PrintStmtHandler));
}
