/// Lumen Statement Handlers
/// Each handler encapsulates language-specific parsing logic for one statement type

use crate::kernel::ast::StmtNode;
use crate::kernel::parser::{Parser, parse_expression};
use crate::kernel::handlers::StatementHandler;

/// Handler for if statements in Lumen
pub struct IfHandler;

impl StatementHandler for IfHandler {
    fn can_handle(&self, parser: &Parser) -> bool {
        parser.check("keyword_if")
    }

    fn parse(&self, parser: &mut Parser) -> Result<StmtNode, String> {
        parser.expect("keyword_if")?;
        let condition = parse_expression(parser)?;

        // Consume optional brace or newline/markers
        let uses_braces = parser.consume("lbrace");
        while parser.consume("newline") || parser.consume("marker_indent_start") {}

        let mut then_block = Vec::new();
        let end_markers = if uses_braces {
            vec!["rbrace", "keyword_else", "keyword_end"]
        } else {
            vec!["marker_indent_end", "keyword_else", "keyword_end"]
        };

        while !parser.is_at_end() && !end_markers.iter().any(|&m| parser.check(m)) {
            if parser.consume("newline") || parser.consume("marker_indent_start")
                || parser.consume("marker_indent_end") || parser.consume("semicolon") {
                continue;
            }
            // Recursively parse nested statements - will use handlers
            then_block.push(parse_statement_internal(parser)?);
            parser.consume("semicolon");
        }

        if uses_braces {
            parser.consume("rbrace");
        }

        let else_block = if parser.check("keyword_else") {
            parser.next();
            let else_uses_braces = parser.consume("lbrace");
            while parser.consume("newline") || parser.consume("marker_indent_start") {}

            let mut block = Vec::new();
            let else_end_markers = if else_uses_braces {
                vec!["rbrace", "keyword_end"]
            } else {
                vec!["marker_indent_end", "keyword_end"]
            };

            while !parser.is_at_end() && !else_end_markers.iter().any(|&m| parser.check(m)) {
                if parser.consume("newline") || parser.consume("marker_indent_start")
                    || parser.consume("marker_indent_end") || parser.consume("semicolon") {
                    continue;
                }
                block.push(parse_statement_internal(parser)?);
                parser.consume("semicolon");
            }

            if else_uses_braces {
                parser.consume("rbrace");
            }

            Some(block)
        } else {
            None
        };

        if parser.check("keyword_end") {
            parser.next();
        }

        Ok(StmtNode::If {
            condition,
            then_block,
            else_block,
            analysis: (),
        })
    }
}

/// Handler for while loops in Lumen
pub struct WhileHandler;

impl StatementHandler for WhileHandler {
    fn can_handle(&self, parser: &Parser) -> bool {
        parser.check("keyword_while")
    }

    fn parse(&self, parser: &mut Parser) -> Result<StmtNode, String> {
        parser.expect("keyword_while")?;
        let condition = parse_expression(parser)?;

        let uses_braces = parser.consume("lbrace");
        while parser.consume("newline") || parser.consume("marker_indent_start") {}

        let mut body = Vec::new();
        let end_markers = if uses_braces {
            vec!["rbrace", "keyword_end"]
        } else {
            vec!["marker_indent_end", "keyword_end"]
        };

        while !parser.is_at_end() && !end_markers.iter().any(|&m| parser.check(m)) {
            if parser.consume("newline") || parser.consume("marker_indent_start")
                || parser.consume("marker_indent_end") || parser.consume("semicolon") {
                continue;
            }
            body.push(parse_statement_internal(parser)?);
            parser.consume("semicolon");
        }

        if uses_braces {
            parser.consume("rbrace");
        } else if parser.check("keyword_end") {
            parser.next();
        }

        Ok(StmtNode::While {
            condition,
            body,
            analysis: (),
        })
    }
}

/// Handler for until loops in Lumen
pub struct UntilHandler;

impl StatementHandler for UntilHandler {
    fn can_handle(&self, parser: &Parser) -> bool {
        parser.check("keyword_until")
    }

    fn parse(&self, parser: &mut Parser) -> Result<StmtNode, String> {
        parser.expect("keyword_until")?;
        let condition = parse_expression(parser)?;

        while parser.consume("newline") || parser.consume("marker_indent_start") {}

        let mut body = Vec::new();
        while !parser.is_at_end() && !parser.check("keyword_end")
            && !parser.check("marker_indent_end") {
            if parser.consume("newline") || parser.consume("marker_indent_start")
                || parser.consume("marker_indent_end") {
                continue;
            }
            body.push(parse_statement_internal(parser)?);
        }

        if parser.check("keyword_end") {
            parser.next();
        }

        Ok(StmtNode::Until {
            condition,
            body,
            analysis: (),
        })
    }
}

/// Handler for for loops in Lumen
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

        while parser.consume("newline") || parser.consume("marker_indent_start") {}

        let mut body = Vec::new();
        while !parser.is_at_end() && !parser.check("keyword_end")
            && !parser.check("marker_indent_end") {
            if parser.consume("newline") || parser.consume("marker_indent_start")
                || parser.consume("marker_indent_end") {
                continue;
            }
            body.push(parse_statement_internal(parser)?);
        }

        if parser.check("keyword_end") {
            parser.next();
        }

        Ok(StmtNode::For {
            variable,
            iterator,
            body,
            analysis: (),
        })
    }
}

/// Handler for function definitions in Lumen
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

            if parser.check("comma") {
                parser.next();
            } else if !parser.check("rparen") {
                return Err("Expected comma or )".to_string());
            }
        }

        parser.expect("rparen")?;

        while parser.consume("newline") || parser.consume("marker_indent_start") {}

        let mut body = Vec::new();
        while !parser.is_at_end() && !parser.check("keyword_end")
            && !parser.check("marker_indent_end") {
            if parser.consume("newline") || parser.consume("marker_indent_start")
                || parser.consume("marker_indent_end") {
                continue;
            }
            body.push(parse_statement_internal(parser)?);
        }

        if parser.check("keyword_end") {
            parser.next();
        }

        Ok(StmtNode::FnDef {
            name,
            params,
            body,
            analysis: (),
        })
    }
}

/// Handler for let bindings in Lumen
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

/// Handler for print statements in Lumen
pub struct PrintHandler;

impl StatementHandler for PrintHandler {
    fn can_handle(&self, parser: &Parser) -> bool {
        parser.check("keyword_print")
    }

    fn parse(&self, parser: &mut Parser) -> Result<StmtNode, String> {
        parser.expect("keyword_print")?;

        let mut arguments = Vec::new();

        if parser.consume("lparen") {
            while !parser.check("rparen") {
                arguments.push(parse_expression(parser)?);

                if parser.check("comma") {
                    parser.next();
                } else if !parser.check("rparen") {
                    break;
                }
            }
            parser.expect("rparen")?;
        } else {
            // Print with space-separated args
            while !parser.is_at_end() && !parser.check("newline")
                && !parser.check("marker_indent_end") && !parser.check("keyword_end") {
                arguments.push(parse_expression(parser)?);
                if parser.check("comma") {
                    parser.next();
                } else {
                    break;
                }
            }
        }

        Ok(StmtNode::Print {
            arguments,
            analysis: (),
        })
    }
}

/// Handler for return statements in Lumen
pub struct ReturnHandler;

impl StatementHandler for ReturnHandler {
    fn can_handle(&self, parser: &Parser) -> bool {
        parser.check("keyword_return")
    }

    fn parse(&self, parser: &mut Parser) -> Result<StmtNode, String> {
        parser.expect("keyword_return")?;

        let value = if !parser.is_at_end() && !parser.check("newline")
            && !parser.check("marker_indent_end") && !parser.check("keyword_end") {
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

/// Handler for break statements in Lumen
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

/// Handler for continue statements in Lumen
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

/// Create and register all Lumen statement handlers
pub fn create_handlers() -> Vec<Box<dyn StatementHandler>> {
    vec![
        Box::new(IfHandler),
        Box::new(WhileHandler),
        Box::new(UntilHandler),
        Box::new(ForHandler),
        Box::new(FunctionDefHandler),
        Box::new(LetHandler),
        Box::new(PrintHandler),
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
