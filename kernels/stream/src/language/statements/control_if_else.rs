use crate::language::prelude::*;
// if / else statement

use crate::kernel::ast::{Control, ExprNode, StmtNode};
use crate::kernel::parser::Parser;
use crate::kernel::runtime::Env;
use crate::language::definition::BlockStyle;
use crate::language::structure::structural;
use crate::language::values::as_bool;

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
            let mut result = Control::None;
            for stmt in &self.then_block {
                let ctl = stmt.exec(env)?;
                match ctl {
                    Control::None => {
                        // Statement completed normally
                    }
                    Control::ExprValue(val) => {
                        // Update result for implicit return from expression
                        result = Control::ExprValue(val);
                    }
                    // Break/Continue/Return control flow
                    other => {
                        result = other;
                        break;
                    }
                }
            }
            return Ok(result);
        } else if let Some(ref else_block) = self.else_block {
            let mut result = Control::None;
            for stmt in else_block {
                let ctl = stmt.exec(env)?;
                match ctl {
                    Control::None => {
                        // Statement completed normally
                    }
                    Control::ExprValue(val) => {
                        // Update result for implicit return from expression
                        result = Control::ExprValue(val);
                    }
                    // Break/Continue/Return control flow
                    other => {
                        result = other;
                        break;
                    }
                }
            }
            return Ok(result);
        }

        Ok(Control::None)
    }
}

pub struct IfStmtHandler;

impl IfStmtHandler {
    /// Parse from the condition onward: `cond block [elif ...] [else block]`.
    /// An elif branch is an if statement nested in the else block.
    fn parse_branch(parser: &mut Parser, registry: &Registry) -> LumenResult<Box<dyn StmtNode>> {
        let d = def();
        let keyword_blocks = d.block_style == BlockStyle::Keyword;
        parser.skip_tokens();
        let cond = parser.parse_expr(registry)?;

        // In the keyword style the whole chain shares one closer, so each
        // body runs to an elif, an else or the closer, and only the end of
        // the chain consumes the closer.
        let then_block = if keyword_blocks {
            structural::skip_block_intro(parser);
            let mut stops: Vec<String> = d.list("block.close").to_vec();
            stops.extend(d.list("stmt.elif").iter().cloned());
            stops.extend(d.list("stmt.else").iter().cloned());
            structural::parse_body(parser, registry, &stops)?
        } else {
            structural::parse_block(parser, registry)?
        };

        structural::consume_separators(parser);

        let else_block = if d.is("stmt.elif", &parser.peek().lexeme) {
            parser.advance(); // consume the elif keyword
            Some(vec![Self::parse_branch(parser, registry)?])
        } else if d.is("stmt.else", &parser.peek().lexeme) {
            parser.advance(); // consume the else keyword
            parser.skip_tokens();
            if d.is("stmt.if", &parser.peek().lexeme) {
                parser.advance(); // `else if`
                Some(vec![Self::parse_branch(parser, registry)?])
            } else if keyword_blocks {
                let body = structural::parse_body(parser, registry, d.list("block.close"))?;
                structural::expect_close(parser)?;
                Some(body)
            } else {
                Some(structural::parse_block(parser, registry)?)
            }
        } else {
            if keyword_blocks {
                structural::expect_close(parser)?;
            }
            None
        };

        Ok(Box::new(IfStmt { cond, then_block, else_block }))
    }
}

impl StmtHandler for IfStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        def().is("stmt.if", &parser.peek().lexeme)
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn StmtNode>> {
        parser.advance(); // consume the if keyword
        Self::parse_branch(parser, registry)
    }
}

pub fn register(reg: &mut Registry) {
    reg.register_stmt(Box::new(IfStmtHandler));
}
