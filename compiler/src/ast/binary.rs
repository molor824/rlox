use crate::span::Span;

use super::{
    char_eq_parser, expression::Expression, primitive::skip_parser, strings_eq_parser,
    unary::unary_expression_parser, Parser,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binary {
    pub left: Box<Expression>,
    pub right: Box<Expression>,
    pub operator: Span<Operator>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operator {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    And,
    Or,
    Xor,
    BitAnd,
    BitOr,
    BitXor,
    LShift,
    RShift,
    LessThan,
    LessThanEq,
    MoreThan,
    MoreThanEq,
    Equals,
    NotEq,
    Assign(Option<Box<Operator>>),
}

pub fn binary_expression_parser() -> Parser<Expression> {
    assign_parser()
}
fn assign_parser() -> Parser<Expression> {
    r_binary_parser(logic_or_parser, || {
        skip_parser().and_then(|_| {
            operator_parser(
                &[
                    "+", "-", "*", "/", "%", "<<", ">>", "&", "^", "|", "&&", "^^", "||",
                ],
                &[
                    Operator::Add,
                    Operator::Sub,
                    Operator::Mul,
                    Operator::Div,
                    Operator::Mod,
                    Operator::LShift,
                    Operator::RShift,
                    Operator::BitAnd,
                    Operator::BitXor,
                    Operator::BitOr,
                    Operator::And,
                    Operator::Xor,
                    Operator::Or,
                ],
            )
            .and_then(|op| {
                char_eq_parser('=')
                    .map(|eq| op.combine(eq, |op, _| Operator::Assign(Some(op.into()))))
            })
            .or_else(|_| char_eq_parser('=').map(|eq| eq.map(|_| Operator::Assign(None))))
        })
    })
}
fn logic_or_parser() -> Parser<Expression> {
    l_binary_parser(logic_xor_parser, || {
        operator_parser(&["||"], &[Operator::Or])
    })
}
fn logic_xor_parser() -> Parser<Expression> {
    l_binary_parser(logic_and_parser, || {
        operator_parser(&["^^"], &[Operator::Xor])
    })
}
fn logic_and_parser() -> Parser<Expression> {
    l_binary_parser(bit_or_parser, || operator_parser(&["&&"], &[Operator::And]))
}
fn bit_or_parser() -> Parser<Expression> {
    l_binary_parser(bit_xor_parser, || {
        operator_parser(&["|"], &[Operator::BitOr])
    })
}
fn bit_xor_parser() -> Parser<Expression> {
    l_binary_parser(bit_and_parser, || {
        operator_parser(&["^"], &[Operator::BitXor])
    })
}
fn bit_and_parser() -> Parser<Expression> {
    l_binary_parser(eq_parser, || operator_parser(&["&"], &[Operator::BitAnd]))
}
fn eq_parser() -> Parser<Expression> {
    l_binary_parser(non_eq_parser, || {
        operator_parser(&["==", "/="], &[Operator::Equals, Operator::NotEq])
    })
}
fn non_eq_parser() -> Parser<Expression> {
    l_binary_parser(shift_parser, || {
        operator_parser(
            &["<=", ">=", "<", ">"],
            &[
                Operator::LessThanEq,
                Operator::MoreThanEq,
                Operator::LessThan,
                Operator::MoreThan,
            ],
        )
    })
}
fn shift_parser() -> Parser<Expression> {
    l_binary_parser(term_parser, || {
        operator_parser(&["<<", ">>"], &[Operator::LShift, Operator::RShift])
    })
}
fn term_parser() -> Parser<Expression> {
    l_binary_parser(product_parser, || {
        operator_parser(&["+", "-"], &[Operator::Add, Operator::Sub])
    })
}
fn product_parser() -> Parser<Expression> {
    l_binary_parser(unary_expression_parser, || {
        operator_parser(
            &["*", "/", "%"],
            &[Operator::Mul, Operator::Div, Operator::Mod],
        )
    })
}
fn operator_parser(
    operator_strings: &'static [&'static str],
    operators: &'static [Operator],
) -> Parser<Span<Operator>> {
    skip_parser().and_then(move |_| {
        strings_eq_parser(operator_strings).map(|i| i.map(|i| operators[i].clone()))
    })
}
fn r_binary_parser(
    mut lower: impl FnMut() -> Parser<Expression> + 'static,
    mut operator: impl FnMut() -> Parser<Span<Operator>> + 'static,
) -> Parser<Expression> {
    let lower1 = lower();
    lower()
        .and_then(|left| {
            operator().and_then(|op| {
                r_binary_parser(lower, operator).map(|right| match right {
                    Expression::Binary(mut binary) => {
                        binary.left = Expression::Binary(Binary {
                            left: left.into(),
                            right: binary.left.into(),
                            operator: op,
                        })
                        .into();
                        Expression::Binary(binary)
                    }
                    right => Expression::Binary(Binary {
                        left: left.into(),
                        right: right.into(),
                        operator: op,
                    }),
                })
            })
        })
        .or_else(move |_| lower1)
}
fn l_binary_parser(
    mut lower: impl FnMut() -> Parser<Expression> + 'static,
    mut operator: impl FnMut() -> Parser<Span<Operator>> + 'static,
) -> Parser<Expression> {
    let lower1 = lower();
    lower()
        .and_then(|left| {
            operator().and_then(|op| {
                l_binary_parser(lower, operator).map(|right| {
                    Expression::Binary(Binary {
                        left: left.into(),
                        right: right.into(),
                        operator: op,
                    })
                })
            })
        })
        .or_else(move |_| lower1)
}

#[cfg(test)]
mod tests {
    #[test]
    fn binary_parser_test() {
        todo!()
    }
}
