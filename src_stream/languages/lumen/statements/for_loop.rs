// src/stmt/for_loop.rs
//
// for i = expr; condition; update
//     <block>
//
// Desugars into: i = expr; while condition { block; update }

use crate::languages::lumen::prelude::*;
use crate::kernel::ast::{Control, ExprNode, StmtNode};
use crate::kernel::parser::Parser;
use crate::languages::lumen::patterns::PatternSet;
use crate::kernel::runtime::Env;
use crate::languages::lumen::structure::structural;
use crate::languages::lumen::values::as_bool;

// Internal statement types for for loop components

#[derive(Debug)]
struct InitStmt {
    var: String,
    expr: Box<dyn ExprNode>,
}

impl StmtNode for InitStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        let val = self.expr.eval(env)?;
        env.assign(&self.var, val)?;
        Ok(Control::None)
    }
}

#[derive(Debug)]
struct UpdateStmt {
    expr: Box<dyn ExprNode>,
}

impl StmtNode for UpdateStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        let _ = self.expr.eval(env)?;
        Ok(Control::None)
    }
}

#[derive(Debug)]
struct ForStmt {
    init: Box<dyn StmtNode>,
    condition: Box<dyn ExprNode>,
    update: Box<dyn StmtNode>,
    body: Vec<Box<dyn StmtNode>>,
}

impl StmtNode for ForStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        // Execute initializer
        self.init.exec(env)?;

        // Loop while condition is true
        loop {
            let cond = self.condition.eval(env)?;
            let cond_bool = as_bool(cond.as_ref())?;

            if cond_bool.value {
                // Each iteration gets its own scope
                env.push_scope();
                let mut break_occurred = false;
                for stmt in &self.body {
                    match stmt.exec(env)? {
                        Control::Break => {
                            break_occurred = true;
                            break;
                        }
                        Control::Continue => break,
                        Control::Return(val) => {
                            env.pop_scope();
                            return Ok(Control::Return(val));
                        }
                        Control::None => {}
                    }
                }
                env.pop_scope();
                if break_occurred {
                    return Ok(Control::None);
                }

                // Execute update expression
                self.update.exec(env)?;
            } else {
                break;
            }
        }
        Ok(Control::None)
    }
}

pub struct ForStmtHandler;

impl StmtHandler for ForStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == "for"
    }

    fn parse(
        &self,
        parser: &mut Parser,
        registry: &super::super::registry::Registry,
    ) -> LumenResult<Box<dyn StmtNode>> {
        parser.advance(); // consume 'for'
        parser.skip_tokens();

        // Parse: var = init_expr
        let init_var = parser.peek().lexeme.clone();
        parser.advance();
        parser.skip_tokens();

        if parser.peek().lexeme != "=" {
            return Err("Expected '=' in for loop initializer".to_string());
        }
        parser.advance();
        parser.skip_tokens();

        let init_expr = parser.parse_expr(registry)?;
        parser.skip_tokens();

        // Build init statement: var = init_expr
        let init_stmt: Box<dyn StmtNode> = Box::new(InitStmt {
            var: init_var,
            expr: init_expr,
        });

        // Expect semicolon
        if parser.peek().lexeme != ";" {
            return Err("Expected ';' after for loop initializer".to_string());
        }
        parser.advance();
        parser.skip_tokens();

        // Parse condition expression
        let condition = parser.parse_expr(registry)?;
        parser.skip_tokens();

        // Expect semicolon
        if parser.peek().lexeme != ";" {
            return Err("Expected ';' after for loop condition".to_string());
        }
        parser.advance();
        parser.skip_tokens();

        // Parse update expression (assignment or similar)
        let update_expr = parser.parse_expr(registry)?;
        parser.skip_tokens();

        // Build update statement from expression
        let update_stmt: Box<dyn StmtNode> = Box::new(UpdateStmt {
            expr: update_expr,
        });

        // Parse indented body
        let body = structural::parse_block(parser, registry)?;

        Ok(Box::new(ForStmt {
            init: init_stmt,
            condition,
            update: update_stmt,
            body,
        }))
    }
}

// --------------------
// Pattern Declaration
// --------------------

/// Declare what patterns this module recognizes
pub fn patterns() -> PatternSet {
    PatternSet::new().with_literals(vec!["for"])
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    reg.register_stmt(Box::new(ForStmtHandler));
}
