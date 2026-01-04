use crate::languages::mini_rust::prelude::*;
// if / else statement for mini-rust

use crate::kernel::ast::{Control, ExprNode, StmtNode};
use crate::kernel::parser::Parser;
use crate::kernel::registry::LumenResult;
use crate::languages::mini_rust::registry::{Registry, StmtHandler};
use crate::kernel::runtime::Env;
use crate::languages::mini_rust::structure::structural;
use crate::languages::mini_rust::values::as_bool;

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

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn StmtNode>> {
        parser.advance(); // consume 'if'
        parser.skip_whitespace();

        let cond = parser.parse_expr(registry)?;
        parser.skip_whitespace();

        let then_block = structural::parse_block(parser, registry)?;

        structural::consume_newlines(parser);
        parser.skip_whitespace();

        let else_block = if parser.peek().lexeme == ELSE {
            parser.advance(); // consume 'else'
            parser.skip_whitespace();
            Some(structural::parse_block(parser, registry)?)
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
