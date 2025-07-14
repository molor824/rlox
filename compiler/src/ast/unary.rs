use super::{expression::Expression, Parser, Span, *};
use crate::ast::expression::inline_expression_parser;
use crate::ast::primary::{args_parser, symbol_parser};
use crate::ast::primitive::{ident_parser, Ident};
use primary::{primary_parser, symbols_parser};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefixOperator {
    Negate,
    Not,
    BitNot,
}
impl PrefixOperator {
    pub fn try_from_str(op: &str) -> Option<PrefixOperator> {
        match op {
            "-" => Some(PrefixOperator::Negate),
            "!" => Some(PrefixOperator::Not),
            "~" => Some(PrefixOperator::BitNot),
            _ => None,
        }
    }
}
impl fmt::Display for PrefixOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrefixOperator::Negate => write!(f, "-"),
            PrefixOperator::Not => write!(f, "!"),
            PrefixOperator::BitNot => write!(f, "~"),
        }
    }
}
#[derive(Debug)]
pub struct PrefixUnary {
    pub operator: Span<PrefixOperator>,
    pub operand: Box<Expression>,
}
impl fmt::Display for PrefixUnary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({})", self.operator.value, self.operand)
    }
}

#[derive(Debug)]
pub enum PostfixOperator {
    Call(Vec<Expression>),
    Property(Ident),
    Index(Box<Expression>),
}
impl fmt::Display for PostfixOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Call(args) => write!(
                f,
                "({})",
                args.iter()
                    .map(|arg| arg.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            Self::Property(property) => write!(f, ".{}", property),
            Self::Index(index) => write!(f, "[{}]", index),
        }
    }
}
#[derive(Debug)]
pub struct PostfixUnary {
    pub operator: Span<PostfixOperator>,
    pub operand: Box<Expression>,
}
impl fmt::Display for PostfixUnary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}){}", self.operand, self.operator.value)
    }
}

pub fn unary_expression_parser(skip_newline: bool) -> Parser<Expression> {
    prefix_unary_parser(skip_newline)
}
fn prefix_unary_parser(skip_newline: bool) -> Parser<Expression> {
    symbols_parser(skip_newline, &["-", "!", "~"])
        .map(|str| str.map(|op| PrefixOperator::try_from_str(op).unwrap()))
        .and_then(move |operator| {
            prefix_unary_parser(skip_newline).map(move |expr| {
                Expression::PrefixUnary(PrefixUnary {
                    operator,
                    operand: expr.into(),
                })
            })
        })
        .or_else(move |_| postfix_unary_parser(skip_newline))
}
fn postfix_unary_parser(skip_newline: bool) -> Parser<Expression> {
    primary_parser(skip_newline).fold(
        move || {
            postfix_property_parser(skip_newline)
                .or_else(move |_| postfix_call_parser(skip_newline))
                .or_else(move |_| postfix_index_parser(skip_newline))
        },
        |operand, operator| {
            Expression::PostfixUnary(PostfixUnary {
                operand: operand.into(),
                operator,
            })
        },
    )
}
fn postfix_property_parser(skip_newline: bool) -> Parser<Span<PostfixOperator>> {
    symbol_parser(skip_newline, ".").and_then(move |dot| {
        ident_parser(skip_newline)
            .map(move |ident| dot.combine(ident.span(), |_, _| PostfixOperator::Property(ident)))
    })
}
fn postfix_index_parser(skip_newline: bool) -> Parser<Span<PostfixOperator>> {
    symbol_parser(skip_newline, "[")
        .and_then(|lparen| inline_expression_parser().map(move |expr| (lparen, expr)))
        .and_then(move |(lparen, expr)| {
            symbol_parser(skip_newline, "]").map(move |rparen| {
                lparen.combine(rparen, |_, _| PostfixOperator::Index(expr.into()))
            })
        })
}
fn postfix_call_parser(skip_newline: bool) -> Parser<Span<PostfixOperator>> {
    symbol_parser(skip_newline, "(").and_then(move |lparen| {
        args_parser(skip_newline)
            .or_else(|_| Parser::new_ok(vec![]))
            .and_then(move |args| {
                symbol_parser(skip_newline, ")")
                    .map(move |rparen| lparen.combine(rparen, |_, _| PostfixOperator::Call(args)))
            })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prefix_unary() {
        let test = " - !~~ ident";
        let answer = "-(!(~(~(ident))))";
        assert_eq!(
            unary_expression_parser(true)
                .parse(Scanner::new(test))
                .unwrap()
                .1
                .to_string(),
            answer
        );
    }
    #[test]
    fn test_postfix_unary() {
        let test = "a.c(d[f(1, 2)].e(3)(4)[5])";
        let answer = "((a).c)((((((d)[(f)(1,2)]).e)(3))(4))[5])";
        assert_eq!(
            unary_expression_parser(true)
                .parse(Scanner::new(test))
                .unwrap()
                .1
                .to_string(),
            answer,
        );
    }
}
