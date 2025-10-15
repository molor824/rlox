use super::{
    binary::{binary_expression_parser, Binary},
    unary::PrefixUnary,
    Parser, SpanOf,
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

#[derive(Debug, Clone)]
pub enum Expression {
    Ident(Ident),
    CharLit(SpanOf<char>),
    StrLit(SpanOf<String>),
    Number(SpanOf<Number>),
    Group(SpanOf<Box<Expression>>),
    Array(SpanOf<Vec<Expression>>),
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

pub fn inline_expression_parser() -> Parser<Expression> {
    expression_parser(false)
}
pub fn multiline_expression_parser() -> Parser<Expression> {
    expression_parser(true)
}
pub fn expression_parser(skip_newline: bool) -> Parser<Expression> {
    binary_expression_parser(skip_newline)
}

#[cfg(test)]
mod tests {
    use crate::ast::scanner::Scanner;

    use super::*;

    #[test]
    fn inline_test() {
        let test = r"a = (
        3 
        +
         2
    ) * 3 + 1 / ( 10 - a[0] 
     
    
    )";
        let answer = "(a)=((((3)+(2))*(3))+((1)/((10)-((a)[0]))))";
        let result = inline_expression_parser()
            .parse(Scanner::new(test.chars()))
            .unwrap()
            .1;
        assert_eq!(result.to_string(), answer);

        let err_test = r#"a =
        "this should throw error"
        +
        301
        "#;
        let answer = "a";
        // the expression parser never really throws error when the resulting string is incomplete, but compiles up to as much as it can
        // and the resulting parser should only parse up to the "a" ident and no further
        let err_result = inline_expression_parser()
            .parse(Scanner::new(err_test.chars()))
            .unwrap()
            .1;
        assert_eq!(err_result.to_string(), answer);
    }

    #[test]
    fn multiline_test() {
        let test = r"
        (
        a + b * c) / d
        [0]
        (1, 2, 3 * 
        
        
        
        10)";
        let answer = "((a)+((b)*(c)))/(((d)[0])(1,2,(3)*(10)))";
        let result = multiline_expression_parser()
            .parse(Scanner::new(test.chars()))
            .unwrap()
            .1;
        assert_eq!(result.to_string(), answer);
    }
}
