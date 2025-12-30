#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    None,
    Bool(bool),
    Int(i64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExprTag {
    Literal,
    Var,
    Unary,
    Binary,
    Compare,
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

impl Expr {
    pub fn tag(&self) -> ExprTag {
        match self {
            Expr::Literal(_) => ExprTag::Literal,
            Expr::Var(_) => ExprTag::Var,
            Expr::Unary { .. } => ExprTag::Unary,
            Expr::Binary { .. } => ExprTag::Binary,
            Expr::Compare { .. } => ExprTag::Compare,
        }
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StmtTag {
    Assign,
    Print,
    If,
    While,
    Break,
    Continue,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Assign {
        name: String,
        value: Expr,
    },
    Print {
        expr: Expr,
    },

    If {
        cond: Expr,
        then_block: Vec<Stmt>,
        else_block: Option<Vec<Stmt>>,
    },

    While {
        cond: Expr,
        body: Vec<Stmt>,
    },

    Break,
    Continue,
}

impl Stmt {
    pub fn tag(&self) -> StmtTag {
        match self {
            Stmt::Assign { .. } => StmtTag::Assign,
            Stmt::Print { .. } => StmtTag::Print,
            Stmt::If { .. } => StmtTag::If,
            Stmt::While { .. } => StmtTag::While,
            Stmt::Break => StmtTag::Break,
            Stmt::Continue => StmtTag::Continue,
        }
    }
}
