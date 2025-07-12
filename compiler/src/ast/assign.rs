use std::fmt;

use crate::{
    ast::{
        binary::binary_expression_parser,
        error::Error,
        expression::{expression_parser, Expression},
        primary::symbol_parser,
        primitive::Ident,
        unary::{postfix_unary_parser, PostfixOperator},
        Parser,
    },
    span::Spanning,
};

#[derive(Debug)]
pub struct Assign {
    pub assignee: Assignee,
    pub expr: Box<Expression>,
}
impl Spanning for Assign {
    fn range(&self) -> std::ops::Range<usize> {
        self.assignee.start()..self.expr.end()
    }
}
impl fmt::Display for Assign {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(= {} {})", self.assignee, self.expr)
    }
}

#[derive(Debug)]
pub enum Assignee {
    Ident(Ident),
    Property(Box<Expression>, Ident),
}
impl fmt::Display for Assignee {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ident(i) => write!(f, "{}", i),
            Self::Property(expression, i) => write!(f, "{}.{}", expression, i),
        }
    }
}
impl Spanning for Assignee {
    fn range(&self) -> std::ops::Range<usize> {
        match self {
            Self::Ident(ident) => ident.0.range(),
            Self::Property(expression, property) => expression.start()..property.0.end,
        }
    }
}

pub fn assign_expression_parser() -> Parser<Expression> {
    assignee_parser()
        .and_then(|assignee| symbol_parser("=").map(|_| assignee))
        .and_then(|assignee| {
            expression_parser().map(|expr| {
                Expression::Assign(Assign {
                    assignee,
                    expr: expr.into(),
                })
            })
        })
        .or_else(|_| binary_expression_parser())
}

fn assignee_parser() -> Parser<Assignee> {
    postfix_unary_parser().and_then(|expr| match expr {
        Expression::Ident(ident) => Parser::new_ok(Assignee::Ident(ident)),
        Expression::PostfixUnary(unary) => match unary.operator.value {
            PostfixOperator::Property(property) => {
                Parser::new_ok(Assignee::Property(unary.operand, property))
            }
            _ => Parser::new_err(unary.operator.map(|_| Error::CannotAssign)),
        },
        e => Parser::new_err(e.span().map(|_| Error::CannotAssign)),
    })
}

#[cfg(test)]
mod tests {
    use crate::ast::scanner::Scanner;

    use super::*;
    #[test]
    fn assign_test() {
        let test = "a = b.c = d[0](1, 2).e.f = 10 + 321";
        let answer = "(= a (= b.c (= (((d [0:10]) (1:10, 2:10)) .e).f (+ 10:10 321:10))))";
        println!(
            "{:?}",
            assign_expression_parser()
                .parse(Scanner::new(test))
                .unwrap()
                .1
        );
        assert_eq!(
            assign_expression_parser()
                .parse(Scanner::new(test))
                .unwrap()
                .1
                .to_string(),
            answer
        );
    }
}
