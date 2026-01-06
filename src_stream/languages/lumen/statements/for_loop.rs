// src/stmt/for_loop.rs
//
// for <identifier> in <iterable>
//     <block>
//
// Iterates over iterable (range or collection).
// Desugars into: iterator initialization + while loop

use crate::languages::lumen::prelude::*;
use crate::kernel::ast::{Control, ExprNode, StmtNode};
use crate::kernel::parser::Parser;
use crate::languages::lumen::patterns::PatternSet;
use crate::kernel::runtime::Env;
use crate::languages::lumen::structure::structural;
use crate::languages::lumen::expressions::range_expr::as_range;
use crate::languages::lumen::values::LumenNumber;

#[derive(Debug)]
struct ForStmt {
    var: String,
    iterable: Box<dyn ExprNode>,
    body: Vec<Box<dyn StmtNode>>,
}

impl StmtNode for ForStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        // Evaluate the iterable expression
        let iterable_val = self.iterable.eval(env)?;

        // Handle range iteration
        let range = as_range(iterable_val.as_ref())?;

        let mut current = range.start;
        while current < range.end {
            // Set loop variable to current value
            env.assign(&self.var, Box::new(LumenNumber::new((current as i64).to_string())))?;

            // Execute loop body with new scope
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

            current += 1.0;
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

        // Parse loop variable name
        let var_name = parser.peek().lexeme.clone();
        parser.advance();
        parser.skip_tokens();

        // Expect 'in' keyword
        if parser.peek().lexeme != "in" {
            return Err("Expected 'in' after for loop variable".to_string());
        }
        parser.advance();
        parser.skip_tokens();

        // Parse iterable expression
        let iterable = parser.parse_expr(registry)?;
        parser.skip_tokens();

        // Parse indented body
        let body = structural::parse_block(parser, registry)?;

        Ok(Box::new(ForStmt {
            var: var_name,
            iterable,
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
