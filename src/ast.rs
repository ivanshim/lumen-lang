#[derive(Debug, Clone)]
pub enum Expr {
    Number(i64),
    Var(String),
    Binary(Box<Expr>, BinOp, Box<Expr>),
    Compare(Box<Expr>, CmpOp, Box<Expr>),
}

#[derive(Debug, Clone)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone)]
pub enum CmpOp {
    Lt,
    Gt,
    Eq,
    Ne,
}

#[derive(Debug)]
pub enum Stmt {
    Assign(String, Expr),
    While { cond: Expr, body: Vec<Stmt> },
    If { cond: Expr, then_branch: Vec<Stmt>, else_branch: Vec<Stmt> },
    Print(Expr),
}
