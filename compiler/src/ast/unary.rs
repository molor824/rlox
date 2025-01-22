use primary::primary_parser;
use primitive::skip_parser;

use super::{expression::Expression, Parser, Span, *};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    Negate,
    Not,
    BitNot,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unary {
    pub operators: Vec<Span<Operator>>,
    pub operand: Box<Expression>,
}

pub fn unary_expression_parser() -> Parser<Expression> {
    fn _operator() -> Parser<Span<Operator>> {
        skip_parser().and_then(|_| {
            chars_eq_parser(&['-', '!', '~'])
                .map(|ch| ch.map(|op| [Operator::Negate, Operator::Not, Operator::BitNot][op]))
        })
    }
    _operator()
        .map(|op| vec![op])
        .fold(
            || _operator(),
            |mut ops, op| {
                ops.push(op);
                ops
            },
        )
        .and_then(|operators| {
            primary_parser()
                .map(move |operand| {
                    Expression::Unary(Unary {
                        operators,
                        operand: operand.into(),
                    })
                })
                .map_err(|e| e.map(|_| Error::ExpectedPrimary))
        })
        .or_else(|_| primary_parser())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unary() {
        let test = " - !~~ ident";
        let answer = Expression::Unary(Unary {
            operators: vec![
                Span::from_len(1, 1, Operator::Negate),
                Span::from_len(" - ".len(), 1, Operator::Not),
                Span::from_len(" - !".len(), 1, Operator::BitNot),
                Span::from_len(" - !~".len(), 1, Operator::BitNot),
            ],
            operand: Expression::Ident(Span::from_len(
                " - !~~ ".len(),
                "ident".len(),
                "ident".to_string(),
            ))
            .into(),
        });
        assert_eq!(
            unary_expression_parser()
                .parse(Scanner::new(test))
                .unwrap()
                .1,
            answer
        );
    }
}
