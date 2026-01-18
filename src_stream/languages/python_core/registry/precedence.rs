// Mini-PythonCore operator precedence levels
// Each language defines its own precedence scale

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Precedence {
    Lowest = 0,
    Logic = 10,
    Comparison = 20,
    Term = 30,
    Factor = 40,
    Unary = 50,
}

impl Precedence {
    pub fn lowest() -> Self {
        Precedence::Lowest
    }
}

impl std::ops::Add<i32> for Precedence {
    type Output = Precedence;

    fn add(self, rhs: i32) -> Precedence {
        let v = self as i32 + rhs;
        match v {
            v if v <= 0 => Precedence::Lowest,
            v if v <= 10 => Precedence::Logic,
            v if v <= 20 => Precedence::Comparison,
            v if v <= 30 => Precedence::Term,
            v if v <= 40 => Precedence::Factor,
            _ => Precedence::Unary,
        }
    }
}
