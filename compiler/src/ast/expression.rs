use super::{primary::*, unary::*, Parser, Span};

use num_bigint::BigUint;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Number {
    pub radix: u32,
    pub integer: BigUint,
    pub exponent: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expression {
    Ident(Span<String>),
    CharLit(Span<char>),
    StrLit(Span<String>),
    Number(Span<Number>),
    Unary(Unary),
}

pub fn expression_parser() -> Parser<Expression> {
    primary_parser()
}
