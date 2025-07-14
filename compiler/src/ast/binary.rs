use std::fmt;

use crate::span::Span;

use super::{
    expression::Expression, primary::symbols_parser, unary::unary_expression_parser, Parser,
};

#[derive(Debug)]
pub struct Binary {
    pub left: Box<Expression>,
    pub right: Box<Expression>,
    pub operator: Span<Operator>,
}
impl fmt::Display for Binary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}){}({})", self.left, self.operator.value, self.right)
    }
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
impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Operator::Add => "+",
            Operator::Sub => "-",
            Operator::Mul => "*",
            Operator::Div => "/",
            Operator::Mod => "%",
            Operator::And => "and",
            Operator::Or => "or",
            Operator::BitAnd => "&",
            Operator::BitOr => "|",
            Operator::BitXor => "^",
            Operator::LShift => "<<",
            Operator::RShift => ">>",
            Operator::LessThan => "<",
            Operator::LessThanEq => "<=",
            Operator::MoreThan => ">",
            Operator::MoreThanEq => ">=",
            Operator::Equals => "==",
            Operator::NotEq => "!=",
            Operator::Assign(None) => "=",
            Operator::Assign(Some(op)) => return write!(f, "{}=", op),
        })
    }
}
impl Operator {
    pub fn try_from_str(op: &str) -> Option<Self> {
        match op {
            "+" => Some(Operator::Add),
            "-" => Some(Operator::Sub),
            "*" => Some(Operator::Mul),
            "/" => Some(Operator::Div),
            "%" => Some(Operator::Mod),
            "&" => Some(Operator::BitAnd),
            "|" => Some(Operator::BitOr),
            "^" => Some(Operator::BitXor),
            "<<" => Some(Operator::LShift),
            ">>" => Some(Operator::RShift),
            "<" => Some(Operator::LessThan),
            "<=" => Some(Operator::LessThanEq),
            ">" => Some(Operator::MoreThan),
            ">=" => Some(Operator::MoreThanEq),
            "==" => Some(Operator::Equals),
            "!=" => Some(Operator::NotEq),
            "=" => Some(Operator::Assign(None)),
            "and" | "&&" => Some(Operator::And),
            "or" | "||" => Some(Operator::Or),
            op if op.ends_with('=') => Operator::try_from_str(&op[0..op.len() - 1])
                .and_then(|op| match op {
                    Operator::Equals
                    | Operator::NotEq
                    | Operator::LessThan
                    | Operator::LessThanEq
                    | Operator::MoreThan
                    | Operator::MoreThanEq
                    | Operator::Assign(..) => None,
                    op => Some(op),
                })
                .map(|op| Operator::Assign(Some(Box::new(op)))),
            _ => None,
        }
    }
}

pub fn binary_expression_parser(skip_newline: bool) -> Parser<Expression> {
    assign_parser(skip_newline)
}
fn assign_parser(skip_newline: bool) -> Parser<Expression> {
    r_binary_parser(
        move || logic_or_parser(skip_newline),
        move || {
            operator_parser(
                skip_newline,
                &[
                    "+=", "-=", "*=", "/=", "%=", "<<=", ">>=", "&=", "^=", "|=", "&&=", "||=", "=",
                ],
            )
        },
    )
}
fn logic_or_parser(skip_newline: bool) -> Parser<Expression> {
    l_binary_parser(
        move || logic_and_parser(skip_newline),
        move || operator_parser(skip_newline, &["or", "||"]),
    )
}
fn logic_and_parser(skip_newline: bool) -> Parser<Expression> {
    l_binary_parser(
        move || bit_or_parser(skip_newline),
        move || operator_parser(skip_newline, &["and", "&&"]),
    )
}
fn bit_or_parser(skip_newline: bool) -> Parser<Expression> {
    l_binary_parser(
        move || bit_xor_parser(skip_newline),
        move || operator_parser(skip_newline, &["|"]),
    )
}
fn bit_xor_parser(skip_newline: bool) -> Parser<Expression> {
    l_binary_parser(
        move || bit_and_parser(skip_newline),
        move || operator_parser(skip_newline, &["^"]),
    )
}
fn bit_and_parser(skip_newline: bool) -> Parser<Expression> {
    l_binary_parser(
        move || eq_parser(skip_newline),
        move || operator_parser(skip_newline, &["&"]),
    )
}
fn eq_parser(skip_newline: bool) -> Parser<Expression> {
    l_binary_parser(
        move || non_eq_parser(skip_newline),
        move || operator_parser(skip_newline, &["==", "!="]),
    )
}
fn non_eq_parser(skip_newline: bool) -> Parser<Expression> {
    l_binary_parser(
        move || shift_parser(skip_newline),
        move || operator_parser(skip_newline, &["<=", ">=", "<", ">"]),
    )
}
fn shift_parser(skip_newline: bool) -> Parser<Expression> {
    l_binary_parser(
        move || term_parser(skip_newline),
        move || operator_parser(skip_newline, &["<<", ">>"]),
    )
}
fn term_parser(skip_newline: bool) -> Parser<Expression> {
    l_binary_parser(
        move || product_parser(skip_newline),
        move || operator_parser(skip_newline, &["+", "-"]),
    )
}
fn product_parser(skip_newline: bool) -> Parser<Expression> {
    l_binary_parser(
        move || unary_expression_parser(skip_newline),
        move || operator_parser(skip_newline, &["*", "/", "%"]),
    )
}
fn operator_parser(skip_newline: bool, strings: &'static [&'static str]) -> Parser<Span<Operator>> {
    symbols_parser(skip_newline, strings).map(|i| i.map(|i| Operator::try_from_str(i).unwrap()))
}
fn l_binary_parser(
    mut lower: impl FnMut() -> Parser<Expression> + 'static,
    mut operator: impl FnMut() -> Parser<Span<Operator>> + 'static,
) -> Parser<Expression> {
    lower().fold(
        move || {
            let lower = lower();
            operator().and_then(move |op| lower.map(|right| (op, right)))
        },
        |left, (operator, right)| {
            Expression::Binary(Binary {
                left: left.into(),
                right: right.into(),
                operator,
            })
        },
    )
}
fn r_binary_parser(
    mut lower: impl FnMut() -> Parser<Expression> + 'static,
    mut operator: impl FnMut() -> Parser<Span<Operator>> + 'static,
) -> Parser<Expression> {
    let lower1 = lower();
    lower()
        .and_then(|left| {
            operator().and_then(|op| {
                r_binary_parser(lower, operator).map(|right| {
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
    use crate::ast::scanner::Scanner;

    use super::*;

    #[test]
    fn binary_parser_test() {
        let test = "a = b = c = 1 + 2 + 3 * 4 >= 5 \nand 6 * 7 < 8 or 9 == 10 == 11 == 12";
        let answer =
            "(a)=((b)=((c)=((((((1)+(2))+((3)*(4)))>=(5))and(((6)*(7))<(8)))or((((9)==(10))==(11))==(12)))))";
        assert_eq!(
            binary_expression_parser(true)
                .parse(Scanner::new(test))
                .unwrap()
                .1
                .to_string(),
            answer
        )
    }
}
