// if / else statement

use crate::kernel::ast::{Control, ExprNode, StmtNode};
use crate::kernel::lexer::Token;
use crate::kernel::parser::Parser;
use crate::kernel::registry::{LumenResult, Registry, StmtHandler};
use crate::kernel::runtime::Env;
use crate::src_lumen::structure::structural;

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
            crate::kernel::runtime::Value::Bool(b) => b,
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
        let then_block = structural::parse_block(parser)?;

        structural::consume_newlines(parser);

        let else_block = if matches!(parser.peek(), Token::Feature(ELSE)) {
            parser.advance(); // consume 'else'
            Some(structural::parse_block(parser)?)
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
