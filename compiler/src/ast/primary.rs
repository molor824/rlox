use crate::ast::error::Error;
use crate::ast::expression::{expression_parser, multiline_expression_parser};
use crate::span::SpanOf;

use super::{expression::Expression, primitive::*, string_eq_parser, strings_eq_parser, Parser};

pub fn primary_parser(skip_newline: bool) -> Parser<Expression> {
    number_parser(skip_newline)
        .map(Expression::Number)
        .or_else(move |_| char_lit_parser(skip_newline).map(Expression::CharLit))
        .or_else(move |_| string_lit_parser(skip_newline).map(Expression::StrLit))
        .or_else(move |_| ident_parser(skip_newline).map(Expression::Ident))
        .or_else(move |_| group_parser(skip_newline))
        .or_else(move |_| array_parser(skip_newline))
        .map_err(|err| err.map(|_| Error::NoExpression))
}

fn group_parser(skip_newline: bool) -> Parser<Expression> {
    symbol_parser(skip_newline, "(").and_then(move |lparen| {
        multiline_expression_parser().and_then(move |expr| {
            symbol_parser(true, ")")
                .map(move |rparen| Expression::Group(lparen.combine(rparen, |_, _| Box::new(expr))))
        })
    })
}

fn array_parser(skip_newline: bool) -> Parser<Expression> {
    symbol_parser(skip_newline, "[").and_then(move |lparen| {
        args_parser(true).and_then(move |elements| {
            symbol_parser(true, "]")
                .map(move |rparen| Expression::Array(lparen.combine(rparen, |_, _| elements)))
        })
    })
}

pub fn args_parser(skip_newline: bool) -> Parser<Vec<Expression>> {
    expression_parser(skip_newline).map(|expr| vec![expr]).fold(
        move || symbol_parser(skip_newline, ",").and_then(move |_| expression_parser(skip_newline)),
        |mut args, arg| {
            args.push(arg);
            args
        },
    )
}

pub fn symbol_parser(skip_newline: bool, symbol: &'static str) -> Parser<SpanOf<&'static str>> {
    skip_parser(skip_newline).and_then(move |_| string_eq_parser(symbol))
}
pub fn symbols_parser(
    skip_newline: bool,
    symbols: &'static [&'static str],
) -> Parser<SpanOf<&'static str>> {
    skip_parser(skip_newline).and_then(move |_| strings_eq_parser(symbols))
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
            "[1,\n 2\n, 3]",
            "(\n1 + \n(2 + 3 * (4 + 5)))", // regardless of skip newline mode on or off, as long as expression inside parenthesis, it should always skip newlines
        ];
        let answers = [
            "321e-2",
            "ident",
            "0xff",
            "\"a string test!\\n\"",
            "'p'",
            "[1,2,3]",
            "(1)+((2)+((3)*((4)+(5))))",
        ];
        for (test, answer) in tests.into_iter().zip(answers) {
            let (_, result) = primary_parser(false).parse(Scanner::new(test.chars())).unwrap();
            assert_eq!(result.to_string(), answer);
        }
    }
}
