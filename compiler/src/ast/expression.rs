use std::fmt;

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
impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.exponent {
            Some(exp) => write!(
                f,
                "{}e{}:{}",
                self.integer.to_str_radix(self.radix),
                exp,
                self.radix
            ),
            None => write!(
                f,
                "{}:{}",
                self.integer.to_str_radix(self.radix),
                self.radix
            ),
        }
    }
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
impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expression::Ident(ident) => write!(f, "{}", ident.value),
            Expression::CharLit(char_lit) => write!(f, "{:?}", char_lit.value),
            Expression::StrLit(str_lit) => write!(f, "{:?}", str_lit.value),
            Expression::Number(number) => write!(f, "{}", number.value),
            Expression::Unary(unary) => write!(f, "{}", unary),
            Expression::Binary(binary) => write!(f, "{}", binary),
        }
    }
}

pub fn expression_parser() -> Parser<Expression> {
    binary_expression_parser()
}
