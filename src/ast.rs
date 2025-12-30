#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    None,
    Bool(bool),
    Int(i64),
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
}

#[derive(Debug, Clone, Copy)]
pub enum UnOp {
    Neg, // -x
    Not, // not x
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
    Assign { name: String, value: Expr },
    Print { expr: Expr },

    If {
        cond: Expr,
        then_block: Vec<Stmt>,
        else_block: Option<Vec<Stmt>>,
    },

    While { cond: Expr, body: Vec<Stmt> },

    Break,
    Continue,
}
