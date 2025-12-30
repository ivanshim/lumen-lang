#[derive(Debug, Clone)]
pub enum Value {
    None,
    Bool(bool),
    Number(f64),
    Str(String),
    List(Vec<Value>),
}

#[derive(Debug, Clone)]
pub enum Expr {
    Literal(Value),
    Var(String),

    Unary {
        op: UnOp,
        expr: Box<Expr>,
    },

    Binary {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
    },

    Compare {
        left: Box<Expr>,
        op: CmpOp,
        right: Box<Expr>,
    },

    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },

    Index {
        base: Box<Expr>,
        index: Box<Expr>,
    },

    List(Vec<Expr>),
}

#[derive(Debug, Clone, Copy)]
pub enum UnOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    And,
    Or,
}

#[derive(Debug, Clone, Copy)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Assign {
        name: String,
        value: Expr,
    },

    ExprStmt(Expr),

    Print {
        expr: Expr,
    },

    If {
        // if/elif/elif/... chain
        branches: Vec<(Expr, Vec<Stmt>)>,
        else_block: Option<Vec<Stmt>>,
    },

    While {
        cond: Expr,
        body: Vec<Stmt>,
    },

    ForRange {
        name: String,
        start: Expr,
        end: Expr, // half-open [start, end)
        body: Vec<Stmt>,
    },

    FnDef {
        name: String,
        params: Vec<String>,
        body: Vec<Stmt>,
    },

    Return(Option<Expr>),
}
