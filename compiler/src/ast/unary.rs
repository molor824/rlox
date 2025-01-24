use super::{expression::Expression, Parser, Span, *};
use crate::ast::expression::expression_parser;
use crate::ast::primary::symbol_parser;
use crate::ast::primitive::ident_parser;
use primary::{primary_parser, symbols_parser};
use std::fmt;
use std::fmt::Formatter;

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrefixUnary {
    pub operator: Span<PrefixOperator>,
    pub operand: Box<Expression>,
}
impl fmt::Display for PrefixUnary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({} {})", self.operator.value, self.operand)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PostfixOperator {
    Call(Vec<Expression>),
    Property(String),
}
impl fmt::Display for PostfixOperator {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Call(args) => write!(
                f,
                "()[{}]",
                args.iter()
                    .map(|arg| arg.to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            Self::Property(property) => write!(f, ".[{}]", property),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostfixUnary {
    pub operator: Span<PostfixOperator>,
    pub operand: Box<Expression>,
}
impl fmt::Display for PostfixUnary {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "({} {})", self.operand, self.operator.value)
    }
}

pub fn unary_expression_parser() -> Parser<Expression> {
    prefix_unary_parser()
}
fn prefix_unary_parser() -> Parser<Expression> {
    symbols_parser(&["-", "!", "~"])
        .map(|str| str.map(|op| PrefixOperator::try_from_str(op).unwrap()))
        .and_then(|operator| {
            prefix_unary_parser().map(move |expr| {
                Expression::PrefixUnary(PrefixUnary {
                    operator,
                    operand: expr.into(),
                })
            })
        })
        .or_else(|_| postfix_unary_parser())
}
fn postfix_unary_parser() -> Parser<Expression> {
    primary_parser().fold(
        || postfix_property_parser().or_else(|_| postfix_call_parser()),
        |operand, operator| {
            Expression::PostfixUnary(PostfixUnary {
                operand: operand.into(),
                operator,
            })
        },
    )
}
fn postfix_property_parser() -> Parser<Span<PostfixOperator>> {
    symbol_parser(".").and_then(|dot| {
        ident_parser().map(move |ident| dot.combine(ident, |_, i| PostfixOperator::Property(i)))
    })
}
fn postfix_call_parser() -> Parser<Span<PostfixOperator>> {
    symbol_parser("(").and_then(|lparen| {
        expression_parser()
            .map(|arg| vec![arg])
            .fold(
                || symbol_parser(",").and_then(|_| expression_parser()),
                |mut args, arg| {
                    args.push(arg);
                    args
                },
            )
            .or_else(|_| Parser::new_ok(vec![]))
            .and_then(move |args| {
                symbol_parser(")")
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
        let answer = "(- (! (~ (~ ident))))";
        assert_eq!(
            unary_expression_parser()
                .parse(Scanner::new(test))
                .unwrap()
                .1
                .to_string(),
            answer
        );
    }
    #[test]
    fn test_postfix_unary() {
        let test = "a.b.c().d(e.f, g.h(1, 2, 3), i())";
        let answer = "(((((a .[b]) .[c]) ()[]) .[d]) ()[(e .[f]) ((g .[h]) ()[1:10 2:10 3:10]) (i ()[])])";
        assert_eq!(
            unary_expression_parser()
                .parse(Scanner::new(test))
                .unwrap()
                .1
                .to_string(),
            answer,
        );
    }
}
