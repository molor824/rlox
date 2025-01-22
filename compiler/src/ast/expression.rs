use super::{
    binary::{binary_expression_parser, Binary},
    unary::Unary,
    Parser, Span,
};

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
    Binary(Binary),
}

pub fn expression_parser() -> Parser<Expression> {
    binary_expression_parser()
}
