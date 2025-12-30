#[derive(Debug, Clone)]
pub enum Expr {
    Number(f64),
    Var(String),
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
pub enum BinOp {
    Add,
    Sub,
}

#[derive(Debug, Clone, Copy)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Gt,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Assign {
        name: String,
        value: Expr,
    },
    While {
        cond: Expr,
        body: Vec<Stmt>,
    },
    If {
        cond: Expr,
        then_block: Vec<Stmt>,
        else_block: Option<Vec<Stmt>>,
    },
    Print {
        expr: Expr,
    },
}
