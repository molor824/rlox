use num_bigint::{BigInt, BigUint};

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
fn integer_parser(radix: u32) -> Parser<Span<BigUint>> {
    digit_parser(radix)
        .map(|d| d.map(|d| BigUint::from(d)))
        .fold(
            move || digit_parser(radix),
            move |acc, digit| {
                acc.combine(digit, |mut acc, d| {
                    acc *= radix;
                    acc += d;
                    acc
                })
            },
        )
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct NumberToken {
    pub radix: u32,
    pub integer: BigUint,
    pub exponent: Option<i32>,
}
fn decimal_parser(radix: u32) -> Parser<Span<NumberToken>> {
    integer_parser(radix).and_then(move |whole| {
        char_eq('.')
            .and_then(move |dot| {
                integer_parser(radix)
                    .map(move |frac| dot.combine(frac, |_, frac| frac))
                    .or_else(move |_| Parser::new_ok(dot.map(|_| BigUint::ZERO)))
            })
            .map({
                let whole = whole.clone();
                move |frac| {
                    let frac_len = frac.end - frac.start - 1;
                    whole.combine(frac, |wh, fr| NumberToken {
                        radix,
                        integer: {
                            let mut integer = wh;
                            for _ in 0..frac_len {
                                integer *= radix;
                            }
                            integer + fr
                        },
                        exponent: Some(-(frac_len as i32)),
                    })
                }
            })
            .or_else(move |_| {
                Parser::new_ok(whole.map(|whole| NumberToken {
                    radix,
                    integer: whole,
                    exponent: None,
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
                    move |exponent| {
                        let mut exp = BigInt::from(exponent.value);
                        if let Some(Span { value: '-', .. }) = sign {
                            exp = -exp;
                        }
                        if let Some(old_exp) = decimal.value.exponent {
                            exp += old_exp;
                        }
                        let exp = match i32::try_from(exp) {
                            Ok(exp) => exp,
                            Err(_) => {
                                return Parser::new_err(Span::new(
                                    exponent.start,
                                    exponent.end,
                                    ErrorCode::ExponentOverflow,
                                ))
                            }
                        };
                        Parser::new_ok(Span::new(
                            decimal.start,
                            exponent.end,
                            NumberToken {
                                radix: decimal.value.radix,
                                integer: decimal.value.integer,
                                exponent: Some(exp),
                            },
                        ))
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
            BigUint::from(351_u32)
        );
        assert_eq!(
            integer_parser(16)
                .parse(Scanner::new("Ff3"))
                .unwrap()
                .1
                .value,
            BigUint::from(0xff3_u32)
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
                integer: BigUint::from(335_u32),
                exponent: Some(0),
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
                integer: BigUint::from(0xaeff_u32),
                exponent: Some(-2),
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
                integer: BigUint::from(35_u32),
                exponent: Some(-1 - 4),
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
                integer: BigUint::from(053_u32),
                exponent: Some(-2 + 2),
            }
        );
        assert_eq!(
            exponent_parser(16)
                .parse(Scanner::new("E.3pFF"))
                .unwrap()
                .1
                .value,
            NumberToken {
                radix: 16,
                integer: BigUint::from(0xe3_u32),
                exponent: Some(-1 + 0xff),
            }
        );
    }
}
