//! Operators, split by the kind of node that carries them.

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `^` (right associative)
    Pow,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum BoolOp {
    /// `and`
    And,
    /// `or`
    Or,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    /// `-`
    Neg,
    /// `not`
    Not,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ComparisonOp {
    /// `==`
    Eq,
    /// `!=`
    Neq,
    /// `<`
    Lt,
    /// `>`
    Gt,
    /// `<=`
    Le,
    /// `>=`
    Ge,
    /// `in`
    In,
    /// `not in`
    NotIn,
}
