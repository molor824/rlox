use super::{error::Error, expression::Number, scanner::Scanner, *};
use num_bigint::{BigInt, BigUint};
use std::fmt;
use std::rc::Rc;

fn digit_parser(radix: u32) -> Parser<Span<u8>> {
    next_char_parser().and_then(move |ch| match ch.value.to_digit(radix) {
        Some(d) => Parser::new_ok(ch.map(|_| d as u8)),
        None => Parser::new_err(ch.map(|_| Error::CharNotDigit)),
    })
}
fn integer_parser(radix: u32) -> Parser<Span<BigUint>> {
    digit_parser(radix)
        .map_err(|e| e.map(|_| Error::ExpectedInt))
        .map(|d| d.map(|d| BigUint::from(d)))
        .fold(
            move || {
                digit_parser(radix)
                    .or_else(move |_| char_eq_parser('_').and_then(move |_| digit_parser(radix)))
            },
            move |acc, digit| {
                acc.combine(digit, |mut acc, d| {
                    acc *= radix;
                    acc += d;
                    acc
                })
            },
        )
}
fn decimal_parser(radix: u32) -> Parser<Span<Number>> {
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
                    whole.combine(frac, |wh, fr| Number {
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
                Parser::new_ok(whole.map(|whole| Number {
                    radix,
                    integer: whole,
                    exponent: None,
                }))
            })
    })
}
fn exponent_parser(radix: u32) -> Parser<Span<Number>> {
    decimal_parser(radix).and_then(move |decimal| {
        if radix <= 10 {
            chars_eq_parser(&['e', 'E'])
        } else {
            chars_eq_parser(&['p', 'P'])
        }
        .map({
            let decimal = decimal.clone();
            move |exp| decimal.combine(exp, |decimal, _| decimal)
        })
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
                                    Error::ExponentOverflow,
                                ))
                            }
                        };
                        Parser::new_ok(Span::new(
                            decimal.start,
                            exponent.end,
                            Number {
                                radix: decimal.value.radix,
                                integer: decimal.value.integer,
                                exponent: Some(exp),
                            },
                        ))
                    }
                })
                .or_else(move |_| Parser::new_err(decimal.map(|_| Error::MissingExponent)))
        })
        .or_else(move |_| Parser::new_ok(decimal))
    })
}
fn radix_parser() -> Parser<Span<u32>> {
    char_eq_parser('0').and_then(move |zero| {
        char_eq_parser('b')
            .map(|ch| ch.map(|_| 2_u32))
            .or_else(|_| char_eq_parser('o').map(|ch| ch.map(|_| 8_u32)))
            .or_else(|_| char_eq_parser('x').map(|ch| ch.map(|_| 16_u32)))
            .map(move |radix| zero.combine(radix, |_, radix| radix))
            .map_err(|err| err.map(|_| Error::ExpectedBase))
    })
}
pub fn number_parser(skip_newline: bool) -> Parser<Span<Number>> {
    skip_parser(skip_newline).and_then(|_| {
        radix_parser()
            .and_then(|radix| exponent_parser(radix.value).map(move |n| radix.combine(n, |_, n| n)))
            .or_else(|_| exponent_parser(10))
    })
}
fn escape_char_parser() -> Parser<Span<char>> {
    char_eq_parser('\\')
        .and_then(|slash| next_char_parser().map(move |ch| slash.combine(ch, |_, ch| ch)))
        .and_then(move |ch| match ch.value {
            'n' | 't' | 'r' | '\\' | '\'' | '"' | '0' => Parser::new_ok(ch.map(|ch| match ch {
                'n' => '\n',
                't' => '\t',
                'r' => '\r',
                '\\' => '\\',
                '\'' => '\'',
                '"' => '"',
                '0' => '\0',
                _ => unreachable!(),
            })),
            'u' | 'U' => char_eq_parser('{')
                .and_then(move |brace| {
                    integer_parser(16).map(move |hex| brace.combine(hex, |_, hex| hex))
                })
                .and_then(move |hex| {
                    char_eq_parser('}').map(|brace| hex.combine(brace, |hex, _| hex))
                })
                .and_then(move |hex| {
                    let Ok(hex_value) = u32::try_from(hex.value.clone()) else {
                        return Parser::new_err(hex.map(|_| Error::UnicodeOverflow));
                    };
                    let Some(character) = char::from_u32(hex_value) else {
                        return Parser::new_err(hex.map(|_| Error::InvalidUnicode));
                    };
                    Parser::new_ok(hex.map(|_| character))
                }),
            'x' | 'X' => digit_parser(16).and_then(move |first| {
                digit_parser(16)
                    .map(move |second| first.combine(second, |f, s| (f * 16 + s) as char))
            }),
            _ => Parser::new_err(ch.map(|_| Error::InvalidEscape)),
        })
        .map_err(|err| err.map(|_| Error::MissingEscape))
}
pub fn string_lit_parser(skip_newline: bool) -> Parser<Span<String>> {
    skip_parser(skip_newline).and_then(|_| {
        char_eq_parser('"')
            .map(|q| q.map(|_| String::new()))
            .fold(
                move || {
                    escape_char_parser()
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
                        move |_| Parser::new_err(str.map(|_| Error::StringLiteralIncomplete))
                    })
                    .map(move |q| str.combine(q, |str, _| str))
            })
    })
}
pub fn char_lit_parser(skip_newline: bool) -> Parser<Span<char>> {
    skip_parser(skip_newline).and_then(|_| {
        char_eq_parser('\'')
            .and_then(move |q| {
                escape_char_parser()
                    .or_else(move |_| char_match_parser(|ch| ch != '\'' && ch != '\n'))
                    .map_err(move |err| q.combine(err, |_, _| Error::CharLiteralEmpty))
                    .map(move |ch| q.combine(ch, |_, ch| ch))
            })
            .and_then(move |ch| {
                char_eq_parser('\'')
                    .map_err(move |err| err.map(|_| Error::CharLiteralIncomplete))
                    .map(move |q| ch.combine(q, |ch, _| ch))
            })
    })
}
fn string_not_eq_parser(string: &'static str) -> Parser<Span<&'static str>> {
    Parser::new(move |Scanner { source, offset }| {
        if source[offset..].starts_with(&string) {
            let len = source.len();
            Err(Span::from_len(
                offset,
                len,
                Error::UnexpectedString(string.to_string()),
            ))
        } else {
            Ok((
                Scanner { source, offset },
                Span::from_len(offset, 0, string),
            ))
        }
    })
}
fn whitespace_parser(skip_newline: bool) -> Parser<Span<()>> {
    char_match_parser(move |ch| {
        if skip_newline {
            ch.is_whitespace()
        } else {
            ch.is_whitespace() && ch != '\n'
        }
    })
    .map(|ch| ch.map(|_| ()))
}
fn line_comment_parser() -> Parser<Span<()>> {
    string_eq_parser("//").and_then(|comment| {
        char_match_parser(|ch| ch != '\n')
            .map(move |ch| comment.combine(ch, |_, _| ()))
            .or_else(move |_| Parser::new_ok(comment.map(|_| ())))
            .fold(
                || char_match_parser(|ch| ch != '\n'),
                |ch, ch1| ch.combine(ch1, |_, _| ()),
            )
    })
}
fn block_comment_parser() -> Parser<Span<()>> {
    string_eq_parser("/*").and_then(|comment| {
        string_not_eq_parser("*/")
            .map(move |_| comment.map(|_| ()))
            .or_else(move |_| Parser::new_ok(comment.map(|_| ())))
            .fold(
                move || string_not_eq_parser("*/").and_then(move |_| next_char_parser()),
                |comment, ch| comment.combine(ch, |_, _| ()),
            )
            .and_then(|comment| {
                string_eq_parser("*/")
                    .map(move |end| comment.combine(end, |_, _| ()))
                    .or_else(move |_| Parser::new_ok(comment))
            })
    })
}
// The language uses newline as a seperator instead of ; or anything else
// However, if an expression is inside parenthesis, then until the parenthesis ends, newline won't be treated as a seperator
pub fn skip_parser(skip_newline: bool) -> Parser<Span<()>> {
    let one_of = move || {
        whitespace_parser(skip_newline)
            .or_else(|_| line_comment_parser())
            .or_else(|_| block_comment_parser())
    };
    one_of()
        .fold(one_of, |a, b| a.combine(b, |_, _| ()))
        .or_else(|e| Parser::new_ok(Span::from_len(e.start, 0, ())))
}
#[derive(Debug, Clone)]
pub struct Ident(pub Span<Rc<str>>);
impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.0.value[self.0.start..self.0.end])
    }
}
impl Ident {
    pub fn span(&self) -> Span<()> {
        Span::new(self.0.start, self.0.end, ())
    }
}
pub fn ident_parser(skip_newline: bool) -> Parser<Ident> {
    skip_parser(skip_newline).and_then(|_| {
        char_match_parser(|ch| ch.is_alphabetic() || ch == '_')
            .fold(
                || char_match_parser(|ch| ch.is_alphanumeric() || ch == '_'),
                |a, b| a.combine(b, |_, _| '\0'),
            )
            .and_then(|ident| {
                Parser::new_ok_with(move |scanner| Ident(ident.map(|_| scanner.source)))
            })
    })
}
