use crate::span::Span;

use super::{
    error::{Error, ErrorCode},
    scanner::Scanner,
    Parser,
};

fn next_char() -> Parser<Span<char>> {
    Parser::new(|scanner| match scanner.clone().next() {
        Some((next, ch, offset)) => Ok((next, Span::new(offset, offset + ch.len_utf8(), ch))),
        None => Err(Error::new(
            scanner.source,
            Span::from_len(scanner.offset, 0, ErrorCode::Eof),
        )),
    })
}
fn string_eq(string: &'static str) -> Parser<Span<&'static str>> {
    Parser::new(move |scanner| {
        if scanner.source[scanner.offset..].starts_with(string) {
            Ok((
                Scanner {
                    offset: scanner.offset + string.len(),
                    source: scanner.source,
                },
                Span::new(scanner.offset, scanner.offset + string.len(), string),
            ))
        } else {
            Err(Error::new(
                scanner.source,
                Span::from_len(scanner.offset, 0, ErrorCode::StringNotEq(string)),
            ))
        }
    })
}
fn char_eq(ch: char) -> Parser<Span<char>> {
    next_char().and_then(move |char| {
        if char.value == ch {
            Parser::new_ok(char)
        } else {
            Parser::new_err(char.map(|_| ErrorCode::CharNotEq(ch)))
        }
    })
}
fn char_match(f: impl FnOnce(char) -> bool + 'static) -> Parser<Span<char>> {
    next_char().and_then(move |ch| {
        if f(ch.value) {
            Parser::new_ok(ch)
        } else {
            Parser::new_err(ch.map(|_| ErrorCode::CharNotMatch))
        }
    })
}

fn digit_parser(radix: u32) -> Parser<Span<u8>> {
    next_char().and_then(move |ch| match ch.value.to_digit(radix) {
        Some(d) => Parser::new_ok(ch.map(|_| d as u8)),
        None => Parser::new_err(ch.map(|_| ErrorCode::CharNotDigit)),
    })
}
fn integer_parser(radix: u32) -> Parser<Span<Vec<u8>>> {
    digit_parser(radix).map(|d| d.map(|d| vec![d])).fold(
        move || digit_parser(radix),
        move |acc, digit| {
            acc.combine(digit, |mut acc, d| {
                acc.push(d);
                acc
            })
        },
    )
}
// if dot_index is some and 0
// all digits are whole
// if dot_index is some, then it indicates how many digits are before the dot
// 35.2 would have dot_index of 2 as 3 and 5 is before the dot
#[derive(Debug, Clone, PartialEq, Eq)]
struct NumberToken {
    pub radix: u32,
    pub digits: Vec<u8>,
    pub dot_index: Option<i32>,
}
fn decimal_parser(radix: u32) -> Parser<Span<NumberToken>> {
    integer_parser(radix).and_then(move |whole| {
        char_eq('.')
            .and_then(move |dot| {
                integer_parser(radix)
                    .map(move |frac| dot.combine(frac, |_, frac| frac))
                    .or_else(move |_| Parser::new_ok(dot.map(|_| vec![])))
            })
            .map({
                let whole = whole.clone();
                move |frac| {
                    frac.combine(whole, |frac, whole| NumberToken {
                        radix,
                        dot_index: Some(whole.len() as i32),
                        digits: [whole, frac].concat(),
                    })
                }
            })
            .or_else(move |_| {
                Parser::new_ok(whole.map(|whole| NumberToken {
                    radix,
                    digits: whole,
                    dot_index: None,
                }))
            })
    })
}
fn exponent_parser(radix: u32) -> Parser<Span<NumberToken>> {
    decimal_parser(radix).and_then(move |decimal| {
        if radix <= 10 {
            char_match(|ch| matches!(ch, 'e' | 'E'))
        } else {
            char_match(|ch| matches!(ch, 'p' | 'P'))
        }
        .map(move |exp| decimal.combine(exp, |decimal, _| decimal))
        .and_then(move |decimal| {
            char_eq('+')
                .or_else(move |_| char_eq('-'))
                .map({
                    let decimal = decimal.clone();
                    move |sign| (decimal, Some(sign))
                })
                .or_else(move |_| Parser::new_ok((decimal, None)))
        })
        .and_then(move |(decimal, sign)| {
            integer_parser(radix)
                .and_then({
                    let decimal = decimal.clone();
                    move |exp| {
                        let mut exponent: i32 = 0;
                        for digit in &exp.value {
                            exponent = match exponent
                                .checked_mul(radix as i32)
                                .and_then(|e| e.checked_add(*digit as i32))
                            {
                                Some(e) => e,
                                None => {
                                    return Parser::new_err(
                                        exp.map(|_| ErrorCode::ExponentOverflow),
                                    )
                                }
                            }
                        }
                        exponent = match sign {
                            Some(s) if s.value == '+' => exponent,
                            None => exponent,
                            _ => -exponent,
                        };
                        exponent = match decimal.value.dot_index {
                            Some(dot_index) => match dot_index.checked_add(exponent) {
                                Some(e) => e,
                                None => {
                                    return Parser::new_err(
                                        exp.map(|_| ErrorCode::ExponentOverflow),
                                    )
                                }
                            },
                            None => exponent,
                        };

                        Parser::new_ok(decimal.combine(exp, |decimal, _| NumberToken {
                            radix: decimal.radix,
                            digits: decimal.digits,
                            dot_index: Some(exponent),
                        }))
                    }
                })
                .or_else(move |_| Parser::new_err(decimal.map(|_| ErrorCode::MissingExponent)))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::scanner::Scanner;

    #[test]
    fn test_integer_parser() {
        assert_eq!(
            integer_parser(10)
                .parse(Scanner::new("351"))
                .unwrap()
                .1
                .value,
            [3, 5, 1]
        );
        assert_eq!(
            integer_parser(16)
                .parse(Scanner::new("Ff3"))
                .unwrap()
                .1
                .value,
            [0xf, 0xf, 3]
        );
    }

    #[test]
    fn test_decimal_parser() {
        assert_eq!(
            decimal_parser(10)
                .parse(Scanner::new("335."))
                .unwrap()
                .1
                .value,
            NumberToken {
                radix: 10,
                digits: vec![3, 3, 5],
                dot_index: Some(3)
            }
        );
        assert_eq!(
            decimal_parser(16)
                .parse(Scanner::new("Ae.fF"))
                .unwrap()
                .1
                .value,
            NumberToken {
                radix: 16,
                digits: vec![0xA, 0xe, 0xF, 0xF],
                dot_index: Some(2)
            }
        );
    }
    #[test]
    fn test_exponent() {
        assert_eq!(
            exponent_parser(10)
                .parse(Scanner::new("3.5E-4"))
                .unwrap()
                .1
                .value,
            NumberToken {
                radix: 10,
                digits: vec![3, 5],
                dot_index: Some(1 - 4),
            }
        );
        assert_eq!(
            exponent_parser(10)
                .parse(Scanner::new("0.53e+2"))
                .unwrap()
                .1
                .value,
            NumberToken {
                radix: 10,
                digits: vec![0, 5, 3],
                dot_index: Some(1 + 2),
            }
        );
        assert_eq!(
            exponent_parser(16)
                .parse(Scanner::new("E.3p+FF"))
                .unwrap()
                .1
                .value,
            NumberToken {
                radix: 16,
                digits: vec![0xE, 3],
                dot_index: Some(1 + 0xFF),
            }
        );
    }
}
