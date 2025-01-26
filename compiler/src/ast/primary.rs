use crate::ast::expression::expression_parser;
use crate::span::Span;

use super::{expression::Expression, primitive::*, string_eq_parser, strings_eq_parser, Parser};

pub fn primary_parser() -> Parser<Expression> {
    number_parser()
        .map(Expression::Number)
        .or_else(|_| char_lit_parser().map(Expression::CharLit))
        .or_else(|_| string_lit_parser().map(Expression::StrLit))
        .or_else(|_| ident_parser().map(Expression::Ident))
        .or_else(|_| group_parser())
        .or_else(|_| array_parser())
}

fn group_parser() -> Parser<Expression> {
    symbol_parser("(").and_then(|lparen| {
        expression_parser().and_then(move |expr| {
            symbol_parser(")")
                .map(move |rparen| Expression::Group(lparen.combine(rparen, |_, _| Box::new(expr))))
        })
    })
}

fn array_parser() -> Parser<Expression> {
    symbol_parser("[").and_then(|lparen| {
        args_parser().and_then(move |elements| {
            symbol_parser("]")
                .map(move |rparen| Expression::Array(lparen.combine(rparen, |_, _| elements)))
        })
    })
}

pub fn args_parser() -> Parser<Vec<Expression>> {
    expression_parser().map(|expr| vec![expr]).fold(
        || symbol_parser(",").and_then(|_| expression_parser()),
        |mut args, arg| {
            args.push(arg);
            args
        },
    )
}

pub fn symbol_parser(symbol: &'static str) -> Parser<Span<&'static str>> {
    skip_parser().and_then(move |_| string_eq_parser(symbol))
}
pub fn symbols_parser(symbols: &'static [&'static str]) -> Parser<Span<&'static str>> {
    skip_parser().and_then(move |_| strings_eq_parser(symbols))
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
            "[1, 2, 3]",
            "(1 + (2 + 3 * (4 + 5)))",
        ];
        let answers = [
            "321e-2:10",
            "ident",
            "ff:16",
            "\"a string test!\\n\"",
            "'p'",
            "[1:10, 2:10, 3:10]",
            "(+ 1:10 (+ 2:10 (* 3:10 (+ 4:10 5:10))))"
        ];
        for (test, answer) in tests.into_iter().zip(answers) {
            let (_, result) = primary_parser().parse(Scanner::new(test)).unwrap();
            assert_eq!(result.to_string(), answer);
        }
    }
}
