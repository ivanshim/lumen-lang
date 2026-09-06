// Function definition and registry
// Lumen functions are user-defined statements that can be called as expressions
// This module is entirely optional - removing it removes function support

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use crate::kernel::ast::{StmtNode, Control};
use crate::kernel::parser::Parser;
use crate::language::prelude::*;
use crate::kernel::runtime::Env;

// ============================================================================
// FUNCTION REGISTRY
// ============================================================================

/// Stores a function definition: parameters and statement body
pub struct FunctionDef {
    pub params: Vec<String>,
    pub body: Rc<RefCell<Vec<Box<dyn StmtNode>>>>,
}

thread_local! {
    /// Global function registry - stores all defined functions
    /// Maps function name -> FunctionDef
    static FUNCTION_REGISTRY: RefCell<HashMap<String, FunctionDef>> = RefCell::new(HashMap::new());
}

/// Register a function definition with its parameters and body
pub fn define_function(name: String, params: Vec<String>, body: Vec<Box<dyn StmtNode>>) {
    FUNCTION_REGISTRY.with(|registry| {
        let def = FunctionDef {
            params,
            body: Rc::new(RefCell::new(body)),
        };
        registry.borrow_mut().insert(name, def);
    });
}

/// Get a function definition by name (returns Rc to allow shared access)
pub fn get_function(name: &str) -> Option<(Vec<String>, Rc<RefCell<Vec<Box<dyn StmtNode>>>>)> {
    FUNCTION_REGISTRY.with(|registry| {
        registry.borrow().get(name).map(|def| {
            (def.params.clone(), Rc::clone(&def.body))
        })
    })
}


// ============================================================================
// FUNCTION DEFINITION STATEMENT HANDLER
// ============================================================================

// Function definition statement handler
// fn name(param1, param2, ...) { statements }

/// The definition is registered at parse time; executing it is a no-op.
#[derive(Debug)]
struct FnDefStmt;

impl StmtNode for FnDefStmt {
    fn exec(&self, _env: &mut Env) -> LumenResult<Control> {
        // Function is already registered during parsing
        Ok(Control::None)
    }
}

pub struct FnDefStmtHandler;

impl StmtHandler for FnDefStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        def().is("stmt.function", &parser.peek().lexeme)
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn StmtNode>> {
        let d = def();
        let keyword = parser.advance().lexeme; // consume the function keyword
        parser.skip_tokens();

        let name = parser
            .take_identifier()
            .ok_or_else(|| err_at(parser, &format!("Expected function name after '{}'", keyword)))?;
        parser.skip_tokens();

        // Expect the parameter list
        if !d.is("syntax.call.open", &parser.advance().lexeme) {
            return Err(err_at(parser, &format!("Expected '{}' after function name", d.first("syntax.call.open"))));
        }
        parser.skip_tokens();

        // Parse parameters: identifiers, each with an optional type annotation
        let mut params = Vec::new();
        while !d.is("syntax.call.close", &parser.peek().lexeme) {
            let param_name = parser.take_identifier().ok_or_else(|| err_at(parser, "Expected parameter name"))?;
            params.push(param_name);
            parser.skip_tokens();
            if d.is("stmt.let.annotation", &parser.peek().lexeme) {
                parser.advance();
                parser.skip_tokens();
                parser.take_identifier().ok_or_else(|| err_at(parser, "Expected a type name"))?;
                parser.skip_tokens();
            }

            if d.is("syntax.call.separator", &parser.peek().lexeme) {
                parser.advance();
                parser.skip_tokens();
            } else if !d.is("syntax.call.close", &parser.peek().lexeme) {
                return Err(err_at(
                    parser,
                    &format!("Expected '{}' or '{}' after parameter", d.first("syntax.call.separator"), d.first("syntax.call.close")),
                ));
            }
        }
        parser.advance(); // consume the closing bracket
        parser.skip_tokens();

        // An optional return type: the marker and one type name
        if d.is("stmt.function.returns", &parser.peek().lexeme) {
            parser.advance();
            parser.skip_tokens();
            parser.take_identifier().ok_or_else(|| err_at(parser, "Expected a return type"))?;
            parser.skip_tokens();
        }

        // Parse function body (a block)
        let body = crate::language::structure::structural::parse_block(parser, registry)?;

        // Register the function
        define_function(name, params, body);

        Ok(Box::new(FnDefStmt))
    }
}

pub fn register(reg: &mut super::super::registry::Registry) {
    reg.register_stmt(Box::new(FnDefStmtHandler));
}
