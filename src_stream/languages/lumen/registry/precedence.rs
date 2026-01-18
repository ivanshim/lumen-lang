// Lumen operator precedence levels
// Each language defines its own precedence scale

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Precedence {
    Lowest = 0,
    Range = 2,
    Pipe = 5,
    Logic = 10,
    Comparison = 20,
    Term = 30,
    Factor = 40,
    Power = 45,
    Unary = 50,
    Call = 60,
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
            v if v <= 2 => Precedence::Range,
            v if v <= 5 => Precedence::Pipe,
            v if v <= 10 => Precedence::Logic,
            v if v <= 20 => Precedence::Comparison,
            v if v <= 30 => Precedence::Term,
            v if v <= 40 => Precedence::Factor,
            v if v <= 45 => Precedence::Power,
            v if v <= 50 => Precedence::Unary,
            _ => Precedence::Call,
        }
    }
}
