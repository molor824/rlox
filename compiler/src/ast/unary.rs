use std::fmt;

use primary::{primary_parser, symbols_parser};

use super::{expression::Expression, Parser, Span, *};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    Negate,
    Not,
    BitNot,
}
impl Operator {
    pub fn try_from_str(op: &str) -> Option<Operator> {
        match op {
            "-" => Some(Operator::Negate),
            "!" => Some(Operator::Not),
            "~" => Some(Operator::BitNot),
            _ => None,
        }
    }
}
impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operator::Negate => write!(f, "-"),
            Operator::Not => write!(f, "!"),
            Operator::BitNot => write!(f, "~"),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unary {
    pub operator: Span<Operator>,
    pub operand: Box<Expression>,
}
impl fmt::Display for Unary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({} {})", self.operator.value, self.operand)
    }
}

pub fn unary_expression_parser() -> Parser<Expression> {
    symbols_parser(&["-", "!", "~"])
        .map(|str| str.map(|op| Operator::try_from_str(op).unwrap()))
        .and_then(|operator| {
            unary_expression_parser().map(move |expr| {
                Expression::Unary(Unary {
                    operator,
                    operand: expr.into(),
                })
            })
        })
        .or_else(|_| primary_parser())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unary() {
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
}
