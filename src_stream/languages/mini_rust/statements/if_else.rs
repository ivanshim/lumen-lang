// if / else statement for mini-rust

use crate::src_stream::kernel::ast::{Control, ExprNode, StmtNode};
use crate::src_stream::kernel::parser::Parser;
use crate::src_stream::kernel::registry::{LumenResult, Registry, StmtHandler};
use crate::src_stream::kernel::runtime::Env;
use crate::src_stream::src_mini_rust::structure::structural;
use crate::src_stream::src_mini_rust::values::as_bool;

// --------------------
// Token definitions
// --------------------

pub const IF: &str = "if";
pub const ELSE: &str = "else";

#[derive(Debug)]
struct IfStmt {
    cond: Box<dyn ExprNode>,
    then_block: Vec<Box<dyn StmtNode>>,
    else_block: Option<Vec<Box<dyn StmtNode>>>,
}

impl StmtNode for IfStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        let cond = self.cond.eval(env)?;
        let cond_bool = as_bool(cond.as_ref())?;
        let branch_taken = cond_bool.value;

        if branch_taken {
            env.push_scope();
            let mut result = Control::None;
            for stmt in &self.then_block {
                let ctl = stmt.exec(env)?;
                if !matches!(ctl, Control::None) {
                    result = ctl;
                    break;
                }
            }
            env.pop_scope();
            return Ok(result);
        } else if let Some(ref else_block) = self.else_block {
            env.push_scope();
            let mut result = Control::None;
            for stmt in else_block {
                let ctl = stmt.exec(env)?;
                if !matches!(ctl, Control::None) {
                    result = ctl;
                    break;
                }
            }
            env.pop_scope();
            return Ok(result);
        }

        Ok(Control::None)
    }
}

pub struct IfStmtHandler;

impl StmtHandler for IfStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == IF
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn StmtNode>> {
        parser.advance(); // consume 'if'

        let cond = parser.parse_expr()?;
        let then_block = structural::parse_block(parser)?;

        structural::consume_newlines(parser);

        let else_block = if parser.peek().lexeme == ELSE {
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
    // No token registration needed - kernel handles all segmentation
    // Register tokens
    // Register handlers
    reg.register_stmt(Box::new(IfStmtHandler));
}
