// Let binding statement (immutable)
// let name [: Type] = expression

use crate::languages::lumen::prelude::*;
use crate::kernel::ast::{Control, ExprNode, StmtNode};
use crate::kernel::parser::Parser;
use crate::languages::lumen::patterns::PatternSet;
use crate::kernel::runtime::Env;

#[derive(Debug)]
struct LetStmt {
    name: String,
    _type_annotation: Option<String>, // Optional type annotation
    expr: Box<dyn ExprNode>,
}

impl StmtNode for LetStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        let val = self.expr.eval(env)?;
        env.define(self.name.clone(), val);
        Ok(Control::None)
    }
}

pub struct LetStmtHandler;

impl StmtHandler for LetStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == "let"
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn StmtNode>> {
        parser.advance(); // consume 'let'
        parser.skip_tokens();

        // Parse variable name
        let mut name = String::new();
        if parser.peek().lexeme.chars().next().map_or(false, |c| c.is_alphabetic() || c == '_') {
            name = parser.advance().lexeme;
            parser.skip_tokens();

            // Handle multi-character identifiers split by lexer
            loop {
                if parser.peek().lexeme.len() == 1 {
                    let ch = parser.peek().lexeme.as_bytes()[0];
                    if ch.is_ascii_alphanumeric() || ch == b'_' {
                        name.push_str(&parser.advance().lexeme);
                        parser.skip_tokens();
                        continue;
                    }
                }
                break;
            }
        } else {
            return Err(err_at(parser, "Expected identifier after 'let'"));
        }

        // Parse optional type annotation ": Type"
        let _type_annotation = if parser.peek().lexeme == ":" {
            parser.advance(); // consume ':'
            parser.skip_tokens();

            // Parse type name
            let mut type_name = String::new();
            if parser.peek().lexeme.chars().next().map_or(false, |c| c.is_alphabetic()) {
                type_name = parser.advance().lexeme;
                parser.skip_tokens();

                loop {
                    if parser.peek().lexeme.len() == 1 {
                        let ch = parser.peek().lexeme.as_bytes()[0];
                        if ch.is_ascii_alphanumeric() || ch == b'_' {
                            type_name.push_str(&parser.advance().lexeme);
                            parser.skip_tokens();
                            continue;
                        }
                    }
                    break;
                }
            }
            Some(type_name)
        } else {
            None
        };

        // Expect '='
        if parser.advance().lexeme != "=" {
            return Err(err_at(parser, "Expected '=' in let binding"));
        }
        parser.skip_tokens();

        // Parse expression
        let expr = parser.parse_expr(registry)?;

        Ok(Box::new(LetStmt {
            name,
            _type_annotation,
            expr,
        }))
    }
}

pub fn patterns() -> PatternSet {
    PatternSet::new()
        .with_literals(vec!["let", ":"])
}

pub fn register(reg: &mut Registry) {
    reg.register_stmt(Box::new(LetStmtHandler));
}
