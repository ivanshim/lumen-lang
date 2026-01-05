// Function registry for storing function definitions
// Functions are stored globally during parsing/execution
// Uses a simple thread-local HashMap approach

use std::cell::RefCell;
use std::collections::HashMap;
use super::primitives::Instruction;

/// Stores a function definition: parameters and instruction body
#[derive(Debug, Clone)]
pub struct FunctionDef {
    pub params: Vec<String>,
    pub body: Instruction,
}

thread_local! {
    /// Global function registry - stores all defined functions
    /// Maps function name -> FunctionDef
    static FUNCTION_REGISTRY: RefCell<HashMap<String, FunctionDef>> = RefCell::new(HashMap::new());
}

/// Register a function definition with its parameters and body
pub fn define_function(name: String, params: Vec<String>, body: Instruction) {
    FUNCTION_REGISTRY.with(|registry| {
        let def = FunctionDef { params, body };
        registry.borrow_mut().insert(name, def);
    });
}

/// Get a function definition by name
pub fn get_function(name: &str) -> Option<FunctionDef> {
    FUNCTION_REGISTRY.with(|registry| {
        registry.borrow().get(name).cloned()
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
