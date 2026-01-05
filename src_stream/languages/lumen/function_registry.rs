// Function registry for storing function definitions
// Functions are stored globally during parsing/execution
// Uses Rc<RefCell<>> to store statement bodies without requiring Clone

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use crate::kernel::ast::StmtNode;

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

/// Check if a function exists
pub fn function_exists(name: &str) -> bool {
    FUNCTION_REGISTRY.with(|registry| {
        registry.borrow().contains_key(name)
    })
}

/// Get function parameter names
pub fn get_function_params(name: &str) -> Option<Vec<String>> {
    FUNCTION_REGISTRY.with(|registry| {
        registry.borrow().get(name).map(|def| def.params.clone())
    })
}

/// Clear all function definitions (useful for testing)
pub fn clear_functions() {
    FUNCTION_REGISTRY.with(|registry| {
        registry.borrow_mut().clear();
    });
}
