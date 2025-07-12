use std::fmt;

use crate::span::{Span, Spanning};

use super::{
    expression::Expression, primary::symbols_parser, unary::unary_expression_parser, Parser,
};

#[derive(Debug)]
pub struct Binary {
    pub left: Box<Expression>,
    pub right: Box<Expression>,
    pub operator: Span<Operator>,
}
impl Spanning for Binary {
    fn range(&self) -> std::ops::Range<usize> {
        self.left.start()..self.right.end()
    }
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
            "and" => Some(Operator::And),
            "or" => Some(Operator::Or),
            _ => None,
        }
    }
}

pub fn binary_expression_parser() -> Parser<Expression> {
    logic_or_parser()
}
fn logic_or_parser() -> Parser<Expression> {
    l_binary_parser(logic_and_parser, || operator_parser(&["or"]))
}
fn logic_and_parser() -> Parser<Expression> {
    l_binary_parser(bit_or_parser, || operator_parser(&["and"]))
}
fn bit_or_parser() -> Parser<Expression> {
    l_binary_parser(bit_xor_parser, || operator_parser(&["|"]))
}
fn bit_xor_parser() -> Parser<Expression> {
    l_binary_parser(bit_and_parser, || operator_parser(&["^"]))
}
fn bit_and_parser() -> Parser<Expression> {
    l_binary_parser(eq_parser, || operator_parser(&["&"]))
}
fn eq_parser() -> Parser<Expression> {
    l_binary_parser(non_eq_parser, || operator_parser(&["==", "!="]))
}
fn non_eq_parser() -> Parser<Expression> {
    l_binary_parser(shift_parser, || operator_parser(&["<=", ">=", "<", ">"]))
}
fn shift_parser() -> Parser<Expression> {
    l_binary_parser(term_parser, || operator_parser(&["<<", ">>"]))
}
fn term_parser() -> Parser<Expression> {
    l_binary_parser(product_parser, || operator_parser(&["+", "-"]))
}
fn product_parser() -> Parser<Expression> {
    l_binary_parser(unary_expression_parser, || {
        operator_parser(&["*", "/", "%"])
    })
}
fn operator_parser(strings: &'static [&'static str]) -> Parser<Span<Operator>> {
    symbols_parser(strings).map(|i| i.map(|i| Operator::try_from_str(i).unwrap()))
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

#[cfg(test)]
mod tests {
    use crate::ast::scanner::Scanner;

    use super::*;

    #[test]
    fn binary_parser_test() {
        let test = "1 + 2 + 3 * 4 >= 5 and 6 * 7 < 8 or 9 == 10 == 11 == 12";
        let answer =
            "(((((1)+(2))+((3)*(4)))>=(5))and(((6)*(7))<(8)))or((((9)==(10))==(11))==(12))";
        assert_eq!(
            binary_expression_parser()
                .parse(Scanner::new(test))
                .unwrap()
                .1
                .to_string(),
            answer
        )
    }
}
