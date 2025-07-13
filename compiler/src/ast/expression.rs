use super::{
    binary::{binary_expression_parser, Binary},
    unary::PrefixUnary,
    Parser, Span,
};
use std::fmt;

use crate::ast::primitive::Ident;
use crate::ast::unary::PostfixUnary;
use num_bigint::BigUint;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Number {
    pub radix: u32,
    pub integer: BigUint,
    pub exponent: Option<i32>,
}
impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let radix_prefix = match self.radix {
            2 => "0b",
            8 => "0o",
            16 => "0x",
            _ => "",
        };
        match self.exponent {
            Some(exp) => write!(
                f,
                "{}{}e{}",
                radix_prefix,
                self.integer.to_str_radix(self.radix),
                exp
            ),
            None => write!(
                f,
                "{}{}",
                radix_prefix,
                self.integer.to_str_radix(self.radix)
            ),
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
                    .join(",")
            ),
            Expression::PrefixUnary(unary) => write!(f, "{}", unary),
            Expression::PostfixUnary(unary) => write!(f, "{}", unary),
            Expression::Binary(binary) => write!(f, "{}", binary),
        }
    }
}

pub fn expression_parser() -> Parser<Expression> {
    binary_expression_parser()
}
