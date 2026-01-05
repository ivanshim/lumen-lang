// Function registry for storing function definitions
// Functions are stored globally during parsing/execution
// This is used by function definition and function call handlers

use std::cell::RefCell;
use std::collections::HashMap;
use std::any::Any;
use crate::kernel::ast::StmtNode;

// We use a wrapper to store function bodies
// The function bodies can't be directly cloned, so we store them through Any
pub struct FunctionBody {
    // Stored as a mutable reference that can be temporarily taken
    stmts: RefCell<Option<Vec<Box<dyn StmtNode>>>>,
}

impl FunctionBody {
    pub fn new(stmts: Vec<Box<dyn StmtNode>>) -> Self {
        Self {
            stmts: RefCell::new(Some(stmts)),
        }
    }

    pub fn clone_stmts(&self) -> Option<Vec<Box<dyn StmtNode>>> {
        // This is a workaround - we can't actually clone trait objects
        // For now, we'll use a different approach
        None
    }
}

pub struct FunctionDef {
    pub params: Vec<String>,
    pub body_ptr: *const (),  // Pointer to statement vector (for reference)
}

thread_local! {
    // Store functions with their names and parameter counts
    // The actual bodies will be managed through execution context
    static FUNCTION_REGISTRY: RefCell<HashMap<String, Vec<String>>> = RefCell::new(HashMap::new());
}

/// Register function parameters (body will be handled during execution)
pub fn register_function(name: String, params: Vec<String>) {
    FUNCTION_REGISTRY.with(|registry| {
        registry.borrow_mut().insert(name, params);
    });
}

/// Check if a function exists
pub fn function_exists(name: &str) -> bool {
    FUNCTION_REGISTRY.with(|registry| {
        registry.borrow().contains_key(name)
    })
}

/// Get parameters for a function
pub fn get_function_params(name: &str) -> Option<Vec<String>> {
    FUNCTION_REGISTRY.with(|registry| {
        registry.borrow().get(name).cloned()
    })
}

/// Clear all function definitions (useful for testing)
pub fn clear_functions() {
    FUNCTION_REGISTRY.with(|registry| {
        registry.borrow_mut().clear();
    });
}
