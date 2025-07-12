use super::{binary::Binary, unary::PrefixUnary, Parser, Span};
use std::fmt;

use crate::ast::{
    assign::{assign_expression_parser, Assign},
    unary::PostfixUnary,
};
use crate::{ast::primitive::Ident, span::Spanning};
use num_bigint::BigUint;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Number {
    pub radix: u32,
    pub integer: BigUint,
    pub exponent: Option<i32>,
}
impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.exponent {
            Some(exp) => write!(f, "{}e{}:{}", self.integer, exp, self.radix,),
            None => write!(f, "{}:{}", self.integer, self.radix,),
        }
    }
}

#[derive(Debug)]
pub enum Expression {
    Ident(Ident),
    CharLit(Span<char>),
    StrLit(Span<String>),
    Number(Span<Number>),
    Group(Span<Box<Expression>>),
    Array(Span<Vec<Expression>>),
    PrefixUnary(PrefixUnary),
    PostfixUnary(PostfixUnary),
    Binary(Binary),
    Assign(Assign),
}
impl Spanning for Expression {
    fn range(&self) -> std::ops::Range<usize> {
        match self {
            Self::Ident(ident) => ident.0.range(),
            Self::CharLit(ch) => ch.range(),
            Self::StrLit(str) => str.range(),
            Self::Number(num) => num.range(),
            Self::Group(group) => group.range(),
            Self::Array(array) => array.range(),
            Self::PrefixUnary(prefix) => prefix.range(),
            Self::PostfixUnary(postfix) => postfix.range(),
            Self::Binary(binary) => binary.range(),
            Self::Assign(assign) => assign.range(),
        }
    }
}
impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expression::Ident(ident) => write!(f, "{}", ident),
            Expression::CharLit(char_lit) => write!(f, "{:?}", char_lit.value),
            Expression::StrLit(str_lit) => write!(f, "{:?}", str_lit.value),
            Expression::Number(number) => write!(f, "{}", number.value),
            Expression::Group(expr) => write!(f, "{}", expr.value),
            Expression::Array(array) => write!(
                f,
                "[{}]",
                array
                    .value
                    .iter()
                    .map(Expression::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Expression::PrefixUnary(unary) => write!(f, "{}", unary),
            Expression::PostfixUnary(unary) => write!(f, "{}", unary),
            Expression::Binary(binary) => write!(f, "{}", binary),
            Expression::Assign(assign) => write!(f, "{}", assign),
        }
    }
}

pub fn expression_parser() -> Parser<Expression> {
    assign_expression_parser()
}
