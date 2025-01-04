use crate::{
    ast::primitive::{
        char_lit_parser, ident_parser, number_parser, skip_parser, string_lit_parser,
    },
    span::Span,
};

use super::{primitive::NumberToken, Parser};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Primary {
    Ident(String),
    CharLit(char),
    StrLit(String),
    Number(NumberToken),
}

pub fn primary_parser() -> Parser<Span<Primary>> {
    fn _primary() -> Parser<Span<Primary>> {
        number_parser()
            .map(|n| n.map(Primary::Number))
            .or_else(|_| char_lit_parser().map(|c| c.map(Primary::CharLit)))
            .or_else(|_| string_lit_parser().map(|s| s.map(Primary::StrLit)))
            .or_else(|_| ident_parser().map(|i| i.map(Primary::Ident)))
    }
    skip_parser()
        .and_then(|_| _primary())
        .or_else(|_| _primary())
}

#[cfg(test)]
mod tests {
    use crate::ast::scanner::Scanner;

    use super::*;

    #[test]
    fn primary_test() {
        let test = "/* comment */ 3.21 ident 0xff \"a string test!\\n\" 'p'";
        let answers = [
            Primary::Number(NumberToken {
                radix: 10,
                integer: 321_u32.into(),
                exponent: Some(-2),
            }),
            Primary::Ident("ident".to_string()),
            Primary::Number(NumberToken {
                radix: 16,
                integer: 0xff_u32.into(),
                exponent: None,
            }),
            Primary::StrLit("a string test!\n".to_string()),
            Primary::CharLit('p'),
        ];
        let mut scanner = Scanner::new(test);
        for answer in answers {
            let (next, result) = primary_parser().parse(scanner).unwrap();
            assert_eq!(result.value, answer);
            scanner = next;
        }
    }
}
