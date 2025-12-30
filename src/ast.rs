#[derive(Debug, Clone)]
pub enum Expr {
    Number(f64),
    Var(String),
    Binary(Box<Expr>, BinOp, Box<Expr>),
    Compare(Box<Expr>, CmpOp, Box<Expr>),
}

#[derive(Debug, Clone)]
pub enum BinOp {
    Add,
    Sub,
}

#[derive(Debug, Clone)]
pub enum CmpOp {
    Eq,
    Lt,
    Gt,
    Ne,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Assign { name: String, value: Expr },
    While { cond: Expr, body: Vec<Stmt> },
    If {
        cond: Expr,
        then_block: Vec<Stmt>,
        else_block: Option<Vec<Stmt>>,
    },
    Print { expr: Expr },
}
