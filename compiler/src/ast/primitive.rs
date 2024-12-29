use num_bigint::{BigInt, BigUint};

use crate::span::Span;

use super::{
    error::{Error, ErrorCode},
    scanner::Scanner,
    Parser,
};

fn next_char_parser() -> Parser<Span<char>> {
    Parser::new(|scanner| match scanner.clone().next() {
        Some((next, ch, offset)) => Ok((next, Span::new(offset, offset + ch.len_utf8(), ch))),
        None => Err(Error::new(
            scanner.source,
            Span::from_len(scanner.offset, 0, ErrorCode::Eof),
        )),
    })
}
fn string_eq_parser(string: &'static str) -> Parser<Span<&'static str>> {
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
fn char_eq_parser(ch: char) -> Parser<Span<char>> {
    next_char_parser().and_then(move |char| {
        if char.value == ch {
            Parser::new_ok(char)
        } else {
            Parser::new_err(char.map(|_| ErrorCode::CharNotEq(ch)))
        }
    })
}
fn char_match_parser(f: impl FnOnce(char) -> bool + 'static) -> Parser<Span<char>> {
    next_char_parser().and_then(move |ch| {
        if f(ch.value) {
            Parser::new_ok(ch)
        } else {
            Parser::new_err(ch.map(|_| ErrorCode::CharNotMatch))
        }
    })
}

fn digit_parser(radix: u32) -> Parser<Span<u8>> {
    next_char_parser().and_then(move |ch| match ch.value.to_digit(radix) {
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
        char_eq_parser('.')
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
            char_match_parser(|ch| matches!(ch, 'e' | 'E'))
        } else {
            char_match_parser(|ch| matches!(ch, 'p' | 'P'))
        }
        .map(move |exp| decimal.combine(exp, |decimal, _| decimal))
        .and_then(move |decimal| {
            char_eq_parser('+')
                .or_else(move |_| char_eq_parser('-'))
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
fn string_parser() -> Parser<Span<String>> {
    char_eq_parser('"')
        .map(|q| q.map(|_| String::new()))
        .fold(
            move || {
                string_eq_parser(r#"\""#)
                    .map(|s| s.map(|_| '"'))
                    .or_else(|_| char_match_parser(|ch| ch != '"' && ch != '\n'))
            },
            move |str, ch| {
                str.combine(ch, |mut str, ch| {
                    str.push(ch);
                    str
                })
            },
        )
        .and_then(move |str| {
            char_eq_parser('"')
                .or_else({
                    let str = str.clone();
                    move |_| Parser::new_err(str.map(|_| ErrorCode::StringNotTerminated))
                })
                .map(move |q| str.combine(q, |str, _| str))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::scanner::Scanner;

    #[test]
    fn text_string() {
        assert_eq!(
            string_parser()
                .parse(Scanner::new(r#""foo""#))
                .unwrap()
                .1
                .value,
            "foo"
        );
        assert_eq!(
            string_parser()
                .parse(Scanner::new(r#""i say \"foo\"""#))
                .unwrap()
                .1
                .value,
            "i say \"foo\""
        );
        assert_eq!(
            string_parser()
                .parse(Scanner::new(r#""""#))
                .unwrap()
                .1
                .value,
            ""
        );
        assert!(string_parser()
            .parse(Scanner::new("\"unterminated string!\n\n"))
            .is_err())
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
