// if / else statement

use crate::ast::{Control, ExprNode, StmtNode};
use crate::lexer::Token;
use crate::parser::Parser;
use crate::registry::{LumenResult, Registry, StmtHandler};
use crate::runtime::Env;

// --------------------
// Token definitions
// --------------------

pub const IF: &str = "IF";
pub const ELSE: &str = "ELSE";

#[derive(Debug)]
struct IfStmt {
    cond: Box<dyn ExprNode>,
    then_block: Vec<Box<dyn StmtNode>>,
    else_block: Option<Vec<Box<dyn StmtNode>>>,
}

impl StmtNode for IfStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        let cond = self.cond.eval(env)?;
        let branch_taken = match cond {
            crate::runtime::Value::Bool(b) => b,
            _ => return Err("Condition must be a boolean".into()),
        };

        if branch_taken {
            for stmt in &self.then_block {
                let ctl = stmt.exec(env)?;
                if !matches!(ctl, Control::None) {
                    return Ok(ctl);
                }
            }
        } else if let Some(ref else_block) = self.else_block {
            for stmt in else_block {
                let ctl = stmt.exec(env)?;
                if !matches!(ctl, Control::None) {
                    return Ok(ctl);
                }
            }
        }

        Ok(Control::None)
    }
}

pub struct IfStmtHandler;

impl StmtHandler for IfStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        matches!(parser.peek(), Token::Feature(IF))
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn StmtNode>> {
        parser.advance(); // consume 'if'

        let cond = parser.parse_expr()?;
        let then_block = parser.parse_block()?;

        parser.consume_newlines();

        let else_block = if matches!(parser.peek(), Token::Feature(ELSE)) {
            parser.advance(); // consume 'else'
            Some(parser.parse_block()?)
        } else {
            None
        };

        Ok(Box::new(IfStmt {
            cond,
            then_block,
            else_block,
        }))
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // Register tokens
    reg.tokens.add_keyword("if", IF);
    reg.tokens.add_keyword("else", ELSE);

    // Register handlers
    reg.register_stmt(Box::new(IfStmtHandler));
}
