/// Rust Statement Handlers
/// Each handler encapsulates language-specific parsing logic for one statement type

use crate::kernel::ast::StmtNode;
use crate::kernel::parser::{Parser, parse_expression};
use crate::kernel::handlers::StatementHandler;

/// Handler for if statements in Rust
pub struct IfHandler;

impl StatementHandler for IfHandler {
    fn can_handle(&self, parser: &Parser) -> bool {
        parser.check("keyword_if")
    }

    fn parse(&self, parser: &mut Parser) -> Result<StmtNode, String> {
        parser.expect("keyword_if")?;
        let condition = parse_expression(parser)?;
        parser.expect("lbrace")?;

        let mut then_block = Vec::new();
        while !parser.is_at_end() && !parser.check("rbrace") {
            if parser.consume("semicolon") {
                continue;
            }
            then_block.push(parse_statement_internal(parser)?);
        }

        parser.expect("rbrace")?;

        let else_block = if parser.check("keyword_else") {
            parser.next();
            if parser.consume("lbrace") {
                let mut else_body = Vec::new();
                while !parser.is_at_end() && !parser.check("rbrace") {
                    if parser.consume("semicolon") {
                        continue;
                    }
                    else_body.push(parse_statement_internal(parser)?);
                }
                parser.expect("rbrace")?;
                Some(else_body)
            } else {
                return Err("Expected { after else".to_string());
            }
        } else {
            None
        };

        Ok(StmtNode::If {
            condition,
            then_block,
            else_block,
            analysis: (),
        })
    }
}

/// Handler for while loops in Rust
pub struct WhileHandler;

impl StatementHandler for WhileHandler {
    fn can_handle(&self, parser: &Parser) -> bool {
        parser.check("keyword_while")
    }

    fn parse(&self, parser: &mut Parser) -> Result<StmtNode, String> {
        parser.expect("keyword_while")?;
        let condition = parse_expression(parser)?;
        parser.expect("lbrace")?;

        let mut body = Vec::new();
        while !parser.is_at_end() && !parser.check("rbrace") {
            if parser.consume("semicolon") {
                continue;
            }
            body.push(parse_statement_internal(parser)?);
        }

        parser.expect("rbrace")?;

        Ok(StmtNode::While {
            condition,
            body,
            analysis: (),
        })
    }
}

/// Handler for for loops in Rust
pub struct ForHandler;

impl StatementHandler for ForHandler {
    fn can_handle(&self, parser: &Parser) -> bool {
        parser.check("keyword_for")
    }

    fn parse(&self, parser: &mut Parser) -> Result<StmtNode, String> {
        parser.expect("keyword_for")?;
        let variable_token = parser.expect("identifier")?;
        let variable = variable_token.lexeme;

        parser.expect("keyword_in")?;
        let iterator = parse_expression(parser)?;
        parser.expect("lbrace")?;

        let mut body = Vec::new();
        while !parser.is_at_end() && !parser.check("rbrace") {
            if parser.consume("semicolon") {
                continue;
            }
            body.push(parse_statement_internal(parser)?);
        }

        parser.expect("rbrace")?;

        Ok(StmtNode::For {
            variable,
            iterator,
            body,
            analysis: (),
        })
    }
}

/// Handler for function definitions in Rust
pub struct FunctionDefHandler;

impl StatementHandler for FunctionDefHandler {
    fn can_handle(&self, parser: &Parser) -> bool {
        parser.check("keyword_fn")
    }

    fn parse(&self, parser: &mut Parser) -> Result<StmtNode, String> {
        parser.expect("keyword_fn")?;
        let name_token = parser.expect("identifier")?;
        let name = name_token.lexeme;

        parser.expect("lparen")?;
        let mut params = Vec::new();

        while !parser.check("rparen") {
            let param = parser.expect("identifier")?;
            params.push(param.lexeme);

            if parser.check("colon") {
                parser.next();
                // Skip type annotation
                while !parser.check("comma") && !parser.check("rparen") {
                    parser.next();
                }
            }

            if parser.check("comma") {
                parser.next();
            } else if !parser.check("rparen") {
                return Err("Expected comma or )".to_string());
            }
        }

        parser.expect("rparen")?;

        // Skip return type annotation if present
        if parser.consume("arrow") {
            while !parser.check("lbrace") {
                parser.next();
            }
        }

        parser.expect("lbrace")?;

        let mut body = Vec::new();
        while !parser.is_at_end() && !parser.check("rbrace") {
            if parser.consume("semicolon") {
                continue;
            }
            body.push(parse_statement_internal(parser)?);
        }

        parser.expect("rbrace")?;

        Ok(StmtNode::FnDef {
            name,
            params,
            body,
            analysis: (),
        })
    }
}

/// Handler for variable declarations in Rust
pub struct LetHandler;

impl StatementHandler for LetHandler {
    fn can_handle(&self, parser: &Parser) -> bool {
        parser.check("keyword_let")
    }

    fn parse(&self, parser: &mut Parser) -> Result<StmtNode, String> {
        parser.expect("keyword_let")?;

        let is_mut = parser.check("keyword_mut");
        if is_mut {
            parser.next();
        }

        let name_token = parser.expect("identifier")?;
        let name = name_token.lexeme;

        // Skip type annotation if present
        if parser.consume("colon") {
            while !parser.check("assign") && !parser.check("semicolon") {
                parser.next();
            }
        }

        let value = if parser.consume("assign") {
            Some(parse_expression(parser)?)
        } else {
            None
        };

        if is_mut {
            Ok(StmtNode::LetMut {
                name,
                value,
                analysis: (),
            })
        } else {
            Ok(StmtNode::Let {
                name,
                value,
                analysis: (),
            })
        }
    }
}

/// Handler for return statements in Rust
pub struct ReturnHandler;

impl StatementHandler for ReturnHandler {
    fn can_handle(&self, parser: &Parser) -> bool {
        parser.check("keyword_return")
    }

    fn parse(&self, parser: &mut Parser) -> Result<StmtNode, String> {
        parser.expect("keyword_return")?;

        let value = if !parser.is_at_end() && !parser.check("semicolon")
            && !parser.check("rbrace") {
            Some(parse_expression(parser)?)
        } else {
            None
        };

        Ok(StmtNode::Return {
            value,
            analysis: (),
        })
    }
}

/// Handler for break statements in Rust
pub struct BreakHandler;

impl StatementHandler for BreakHandler {
    fn can_handle(&self, parser: &Parser) -> bool {
        parser.check("keyword_break")
    }

    fn parse(&self, parser: &mut Parser) -> Result<StmtNode, String> {
        parser.next();
        Ok(StmtNode::Break)
    }
}

/// Handler for continue statements in Rust
pub struct ContinueHandler;

impl StatementHandler for ContinueHandler {
    fn can_handle(&self, parser: &Parser) -> bool {
        parser.check("keyword_continue")
    }

    fn parse(&self, parser: &mut Parser) -> Result<StmtNode, String> {
        parser.next();
        Ok(StmtNode::Continue)
    }
}

/// Create and register all Rust statement handlers
pub fn create_handlers() -> Vec<Box<dyn StatementHandler>> {
    vec![
        Box::new(IfHandler),
        Box::new(WhileHandler),
        Box::new(ForHandler),
        Box::new(FunctionDefHandler),
        Box::new(LetHandler),
        Box::new(ReturnHandler),
        Box::new(BreakHandler),
        Box::new(ContinueHandler),
    ]
}

/// Internal helper for parsing statements without handlers
/// Used recursively within block parsing
fn parse_statement_internal(parser: &mut Parser) -> Result<StmtNode, String> {
    use crate::kernel::parser;
    use crate::kernel::handlers::get_current_registry;

    // Get the current registry from thread-local storage
    if let Some(handlers) = get_current_registry() {
        parser::parse_statement(parser, handlers)
    } else {
        // Fallback to empty registry if no current registry (shouldn't happen)
        let empty_handlers = crate::kernel::handlers::HandlerRegistry::new();
        parser::parse_statement(parser, &empty_handlers)
    }
}
