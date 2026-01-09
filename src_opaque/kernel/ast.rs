// Abstract Syntax Tree structures for opaque kernel
// These are semantic-neutral - kernel doesn't interpret the data

use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

/// Opaque analysis type - kernel knows nothing about what this contains
/// For now using () as placeholder since language modules provide handlers
pub type OpaqueAnalysis = ();

/// Expression node in the AST
#[derive(Debug, Clone)]
pub enum ExprNode {
    // Literals and identifiers (opaque handler decides meaning)
    Literal {
        lexeme: String,
        handler_type: String,
    },
    Identifier {
        name: String,
    },

    // Prefix operations (opaque analysis for operator)
    Prefix {
        operator: String,
        right: Box<ExprNode>,
        analysis: OpaqueAnalysis,
    },

    // Infix operations (opaque analysis for operator)
    Infix {
        left: Box<ExprNode>,
        operator: String,
        right: Box<ExprNode>,
        analysis: OpaqueAnalysis,
    },

    // Function calls
    Call {
        function: Box<ExprNode>,
        arguments: Vec<ExprNode>,
        analysis: OpaqueAnalysis,
    },

    // Grouping (parentheses)
    Grouped {
        expr: Box<ExprNode>,
    },
}

/// Statement node in the AST
#[derive(Debug, Clone)]
pub enum StmtNode {
    // Expression statement
    Expr {
        expr: ExprNode,
    },

    // Assignment
    Assign {
        target: String,
        value: ExprNode,
        analysis: OpaqueAnalysis,
    },

    // Variable binding
    Let {
        name: String,
        value: Option<ExprNode>,
        analysis: OpaqueAnalysis,
    },

    // Mutable variable binding
    LetMut {
        name: String,
        value: Option<ExprNode>,
        analysis: OpaqueAnalysis,
    },

    // If/else statement
    If {
        condition: ExprNode,
        then_block: Vec<StmtNode>,
        else_block: Option<Vec<StmtNode>>,
        analysis: OpaqueAnalysis,
    },

    // While loop
    While {
        condition: ExprNode,
        body: Vec<StmtNode>,
        analysis: OpaqueAnalysis,
    },

    // Until loop (inverted while)
    Until {
        condition: ExprNode,
        body: Vec<StmtNode>,
        analysis: OpaqueAnalysis,
    },

    // For loop
    For {
        variable: String,
        iterator: ExprNode,
        body: Vec<StmtNode>,
        analysis: OpaqueAnalysis,
    },

    // Function definition
    FnDef {
        name: String,
        params: Vec<String>,
        body: Vec<StmtNode>,
        analysis: OpaqueAnalysis,
    },

    // Print statement
    Print {
        arguments: Vec<ExprNode>,
        analysis: OpaqueAnalysis,
    },

    // Return statement
    Return {
        value: Option<ExprNode>,
        analysis: OpaqueAnalysis,
    },

    // Break statement
    Break,

    // Continue statement
    Continue,
}

/// Program is a sequence of statements
#[derive(Debug)]
pub struct Program {
    pub statements: Vec<StmtNode>,
}

/// Runtime value - opaque type for any language value
/// Uses Arc to allow cloning across function calls and returns
pub type RuntimeValue = Arc<dyn Any + Send + Sync>;

/// Environment for variable and function storage
#[derive(Debug, Clone)]
pub struct Environment {
    pub variables: HashMap<String, RuntimeValue>,
    pub functions: HashMap<String, (Vec<String>, Vec<StmtNode>)>,
}

impl Environment {
    pub fn new() -> Self {
        Environment {
            variables: HashMap::new(),
            functions: HashMap::new(),
        }
    }

    pub fn get(&self, name: &str) -> Option<RuntimeValue> {
        self.variables.get(name).map(|v| v.clone())
    }

    pub fn set(&mut self, name: String, value: RuntimeValue) {
        self.variables.insert(name, value);
    }

    pub fn define_function(&mut self, name: String, params: Vec<String>, body: Vec<StmtNode>) {
        self.functions.insert(name, (params, body));
    }

    pub fn get_function(&self, name: &str) -> Option<(Vec<String>, Vec<StmtNode>)> {
        self.functions.get(name).cloned()
    }
}

/// Control flow result during evaluation
#[derive(Debug, Clone)]
pub enum ControlFlow {
    Normal(RuntimeValue),
    Return(RuntimeValue),
    Break,
    Continue,
}

impl ControlFlow {
    pub fn value(&self) -> RuntimeValue {
        match self {
            ControlFlow::Normal(v) => v.clone(),
            ControlFlow::Return(v) => v.clone(),
            _ => Arc::new(()),
        }
    }
}
