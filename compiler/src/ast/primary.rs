use super::{expression::*, primitive::*, Parser, Span};

pub fn primary_parser() -> Parser<Expression> {
    skip_parser().and_then(|_| {
        number_parser()
            .map(Expression::Number)
            .or_else(|_| char_lit_parser().map(Expression::CharLit))
            .or_else(|_| string_lit_parser().map(Expression::StrLit))
            .or_else(|_| ident_parser().map(Expression::Ident))
    })
}

#[cfg(test)]
mod tests {
    use crate::ast::scanner::Scanner;

    use super::*;

    #[test]
    fn primary_test() {
        let tests = [
            "/* comment */ 3.21",
            " ident",
            " 0xff",
            " \"a string test!\\n\"",
            "'p'",
        ];
        let answers = [
            Expression::Number(Span::from_len(
                "/* comment */ ".len(),
                "3.21".len(),
                Number {
                    radix: 10,
                    integer: 321_u32.into(),
                    exponent: Some(-2),
                },
            )),
            Expression::Ident(Span::from_len(
                " ".len(),
                "ident".len(),
                "ident".to_string(),
            )),
            Expression::Number(Span::from_len(
                " ".len(),
                "0xff".len(),
                Number {
                    radix: 16,
                    integer: 0xff_u32.into(),
                    exponent: None,
                },
            )),
            Expression::StrLit(Span::from_len(
                " ".len(),
                "\"a string test!\\n\"".len(),
                "a string test!\n".to_string(),
            )),
            Expression::CharLit(Span::new("".len(), "'p'".len(), 'p')),
        ];
        for (test, answer) in tests.into_iter().zip(answers) {
            let (_, result) = primary_parser().parse(Scanner::new(test)).unwrap();
            assert_eq!(result, answer);
        }
    }
}
