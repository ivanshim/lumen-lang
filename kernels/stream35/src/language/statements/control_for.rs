// for <identifier> in <start>..<end>
//     <block>
//
// A counted loop. The range is loop syntax, not a value: the definition's
// range operator between two expressions, or its range call with two
// arguments (Python's range(a, b)). The end is evaluated once.

use crate::language::prelude::*;
use crate::kernel::ast::{Control, ExprNode, StmtNode};
use crate::kernel::parser::Parser;
use crate::kernel::runtime::Env;
use crate::language::expressions::calls;
use crate::language::structure::structural;
use crate::language::values::{as_number, LumenNumber};
use num_bigint::BigInt;

#[derive(Debug)]
struct ForStmt {
    var: String,
    start: Box<dyn ExprNode>,
    end: Box<dyn ExprNode>,
    body: Vec<Box<dyn StmtNode>>,
}

impl StmtNode for ForStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        let start = as_number(self.start.eval(env)?.as_ref())?.value.clone();
        let end = as_number(self.end.eval(env)?.as_ref())?.value.clone();

        let mut current = start;
        while current < end {
            env.assign(&self.var, Box::new(LumenNumber::new(current.clone())))?;

            // The body runs in the same scope, as in the microcode kernel
            let mut break_occurred = false;
            for stmt in &self.body {
                match stmt.exec(env)? {
                    Control::Break => {
                        break_occurred = true;
                        break;
                    }
                    Control::Continue => break,
                    Control::ExprValue(_) => {}
                    Control::Return(val) => return Ok(Control::Return(val)),
                    Control::None => {}
                }
            }
            if break_occurred {
                return Ok(Control::None);
            }

            current += BigInt::from(1);
        }

        Ok(Control::None)
    }
}

pub struct ForStmtHandler;

impl StmtHandler for ForStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        def().is("stmt.for", &parser.peek().lexeme)
    }

    fn parse(
        &self,
        parser: &mut Parser,
        registry: &super::super::registry::Registry,
    ) -> LumenResult<Box<dyn StmtNode>> {
        let d = def();
        parser.advance(); // consume 'for'
        parser.skip_tokens();

        let var_name = parser
            .take_identifier()
            .ok_or_else(|| err_at(parser, "Expected a loop variable"))?;
        parser.skip_tokens();

        if !d.is("stmt.for.in", &parser.peek().lexeme) {
            return Err(format!("Expected '{}' after for loop variable", d.first("stmt.for.in")));
        }
        parser.advance();
        parser.skip_tokens();

        // The range: a call spelling it, or start, the range operator, end.
        let (start, end) = if d.is("builtin.range", &word_ahead(parser)) {
            parser.take_identifier();
            parser.skip_tokens();
            let mut args = calls::parse_arguments(parser, registry, "in the range")?;
            if args.len() != 2 {
                return Err(err_at(parser, "A range takes a start and an end"));
            }
            let end = args.pop().expect("two arguments");
            (args.pop().expect("two arguments"), end)
        } else {
            let start = parser.parse_expr(registry)?;
            parser.skip_tokens();
            if !d.is("op.range", &parser.peek().lexeme) {
                return Err(err_at(parser, &format!("Expected '{}' in the for loop's range", d.first("op.range"))));
            }
            parser.advance();
            parser.skip_tokens();
            (start, parser.parse_expr(registry)?)
        };
        parser.skip_tokens();

        let body = structural::parse_block(parser, registry)?;

        Ok(Box::new(ForStmt { var: var_name, start, end, body }))
    }
}

/// The word at the current position, assembled from its character tokens
/// without consuming them.
fn word_ahead(parser: &Parser) -> String {
    let mut word = String::new();
    let mut n = 0;
    while let Some(t) = parser.peek_n(n) {
        let mut chars = t.lexeme.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) if crate::language::word_char(c) => word.push(c),
            _ => break,
        }
        n += 1;
    }
    word
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    reg.register_stmt(Box::new(ForStmtHandler));
}
