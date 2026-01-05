// Function definition statement handler
// fn name(param1, param2, ...) { statements }

use crate::languages::lumen::prelude::*;
use crate::kernel::ast::{Control, StmtNode};
use crate::kernel::parser::Parser;
use crate::languages::lumen::patterns::PatternSet;
use crate::kernel::runtime::Env;
use crate::languages::lumen::structure::structural::{LPAREN, RPAREN};
use crate::languages::lumen::function_registry;

#[derive(Debug)]
struct FnDefStmt {
    name: String,
    // Stores the function definition in the registry during parse time
}

impl StmtNode for FnDefStmt {
    fn exec(&self, _env: &mut Env) -> LumenResult<Control> {
        // Function is already registered during parsing
        Ok(Control::None)
    }
}

pub struct FnDefStmtHandler;

impl StmtHandler for FnDefStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == "fn"
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn StmtNode>> {
        parser.advance(); // consume 'fn'
        parser.skip_tokens();

        // Parse function name
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
            return Err(err_at(parser, "Expected function name after 'fn'"));
        }

        // Expect '('
        if parser.peek().lexeme != LPAREN {
            return Err(err_at(parser, "Expected '(' after function name"));
        }
        parser.advance(); // consume '('
        parser.skip_tokens();

        // Parse parameters (comma-separated identifiers)
        let mut params = Vec::new();

        while parser.peek().lexeme != RPAREN {
            // Parse parameter name
            let mut param_name = String::new();
            if parser.peek().lexeme.chars().next().map_or(false, |c| c.is_alphabetic() || c == '_') {
                param_name = parser.advance().lexeme;
                parser.skip_tokens();

                // Handle multi-character identifiers
                loop {
                    if parser.peek().lexeme.len() == 1 {
                        let ch = parser.peek().lexeme.as_bytes()[0];
                        if ch.is_ascii_alphanumeric() || ch == b'_' {
                            param_name.push_str(&parser.advance().lexeme);
                            parser.skip_tokens();
                            continue;
                        }
                    }
                    break;
                }
            } else {
                return Err(err_at(parser, "Expected parameter name"));
            }

            params.push(param_name);

            // Check for comma (more parameters) or closing paren
            parser.skip_tokens();
            if parser.peek().lexeme == "," {
                parser.advance();
                parser.skip_tokens();
            } else if parser.peek().lexeme != RPAREN {
                return Err(err_at(parser, "Expected ',' or ')' after parameter"));
            }
        }

        // Expect ')'
        if parser.peek().lexeme != RPAREN {
            return Err(err_at(parser, "Expected ')' after parameters"));
        }
        parser.advance(); // consume ')'
        parser.skip_tokens();

        // Parse function body (indented block)
        let body = crate::languages::lumen::structure::structural::parse_block(parser, registry)?;

        // Register the function
        function_registry::define_function(name.clone(), params, body);

        Ok(Box::new(FnDefStmt { name }))
    }
}

pub fn patterns() -> PatternSet {
    PatternSet::new()
        .with_literals(vec!["fn"])
}

pub fn register(reg: &mut Registry) {
    reg.register_stmt(Box::new(FnDefStmtHandler));
}
