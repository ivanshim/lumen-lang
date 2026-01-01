// writeln statement
use crate::framework::ast::{Control, ExprNode, StmtNode};
use crate::framework::lexer::Token;
use crate::framework::parser::Parser;
use crate::framework::registry::{err_at, LumenResult, Registry, StmtHandler};
use crate::framework::runtime::{Env, Value};
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
            Value::Number(n) => {
                if n.fract() == 0.0 {
                    println!("{}", n as i64);
                } else {
                    println!("{}", n);
                }
            }
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
