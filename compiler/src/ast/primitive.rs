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
    Parser::new(move |Scanner { source, offset }| {
        if source[offset..].starts_with(string) {
            Ok((
                Scanner {
                    offset: offset + string.len(),
                    source,
                },
                Span::new(offset, offset + string.len(), string),
            ))
        } else {
            Err(Error::new(
                source,
                Span::from_len(offset, 0, ErrorCode::ExpectedToken(string)),
            ))
        }
    })
}
fn char_eq_parser(ch: char) -> Parser<Span<char>> {
    next_char_parser().and_then(move |char| {
        if char.value == ch {
            Parser::new_ok(char)
        } else {
            Parser::new_err(char.map(|_| ErrorCode::ExpectedChar(ch)))
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
        .map_err(|e| e.map(|c| c.map(|_| ErrorCode::ExpectedInt)))
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
            .map_err(|err| err.map(|c| c.map(|_| ErrorCode::ExpectedBase)))
    })
}
fn number_parser() -> Parser<Span<NumberToken>> {
    radix_parser()
        .and_then(|radix| exponent_parser(radix.value).map(move |n| radix.combine(n, |_, n| n)))
        .or_else(|_| exponent_parser(10))
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
                        return Parser::new_err(hex.map(|_| ErrorCode::UnicodeOverflow));
                    };
                    let Some(character) = char::from_u32(hex_value) else {
                        return Parser::new_err(hex.map(|_| ErrorCode::InvalidUnicode));
                    };
                    Parser::new_ok(hex.map(|_| character))
                }),
            'x' | 'X' => digit_parser(16).and_then(move |first| {
                digit_parser(16)
                    .map(move |second| first.combine(second, |f, s| (f * 16 + s) as char))
            }),
            _ => Parser::new_err(ch.map(|_| ErrorCode::InvalidEscape)),
        })
        .map_err(|err| err.map(|code| code.map(|_| ErrorCode::MissingEscape)))
}
fn string_lit_parser() -> Parser<Span<String>> {
    char_eq_parser('"')
        .map(|q| q.map(|_| String::new()))
        .fold(
            move || {
                escape_char_parser().or_else(|_| char_match_parser(|ch| ch != '"' && ch != '\n'))
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
                    move |_| Parser::new_err(str.map(|_| ErrorCode::StringLiteralIncomplete))
                })
                .map(move |q| str.combine(q, |str, _| str))
        })
}
fn char_lit_parser() -> Parser<Span<char>> {
    char_eq_parser('\'')
        .and_then(move |q| {
            escape_char_parser()
                .or_else(move |_| char_match_parser(|ch| ch != '\'' && ch != '\n'))
                .map_err(move |err| err.map(|e| q.combine(e, |_, _| ErrorCode::CharLiteralEmpty)))
                .map(move |ch| q.combine(ch, |_, ch| ch))
        })
        .and_then(move |ch| {
            char_eq_parser('\'')
                .map_err(move |err| err.map(|_| ch.map(|_| ErrorCode::CharLiteralIncomplete)))
                .map(move |q| ch.combine(q, |ch, _| ch))
        })
}
fn string_not_eq_parser(string: &'static str) -> Parser<Span<&'static str>> {
    Parser::new(move |Scanner { source, offset }| {
        if source[offset..].starts_with(string) {
            let len = source.len();
            Err(Error::new(
                source,
                Span::from_len(offset, len, ErrorCode::UnexpectedToken(string)),
            ))
        } else {
            Ok((
                Scanner { source, offset },
                Span::from_len(offset, 0, string),
            ))
        }
    })
}
fn whitespace_parser() -> Parser<Span<()>> {
    char_match_parser(|ch| ch.is_whitespace()).map(|ch| ch.map(|_| ()))
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
fn skip_parser() -> Parser<Span<()>> {
    fn one_of() -> Parser<Span<()>> {
        whitespace_parser()
            .or_else(|_| line_comment_parser())
            .or_else(|_| block_comment_parser())
    }
    one_of()
        .fold(one_of, |a, b| Span::new(a.start, b.end, ()))
        .or_else(|err| Parser::new_ok(Span::from_len(err.code.start, 0, ())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::scanner::Scanner;

    #[test]
    fn test_skip() {
        let tests = [
            "  ",
            " \n\t\r",
            "// line comment\nsomething else",
            "/* block\n comment */something else",
            "/* incomplete comment",
            "// incomplete comment",
        ];
        let answers = [
            "  ",
            " \n\t\r",
            "// line comment\n",
            "/* block\n comment */",
            "/* incomplete comment",
            "// incomplete comment",
        ];
        for (test, answer) in tests.into_iter().zip(answers) {
            let (next, result) = skip_parser().parse(Scanner::new(test)).unwrap();
            assert_eq!(&next.source[result.start..result.end], answer);
        }
    }
    #[test]
    fn test_character() {
        let tests = [r"'a'", r"'\n'", r"'字'", r"'\u{5B57}'", r"'\x34'"];
        let answers = ['a', '\n', '字', '\u{5B57}', '\x34'];
        for (test, answer) in tests.into_iter().zip(answers) {
            assert_eq!(
                char_lit_parser().parse(Scanner::new(test)).unwrap().1.value,
                answer
            );
        }
    }
    #[test]
    fn test_string() {
        let tests = [
            r#""foo""#,
            r#""i say \"foo\"""#,
            r#""""#,
            r#""\n\t\r\0\'\"\u{32}\x45""#,
        ];
        let answers = ["foo", "i say \"foo\"", "", "\n\t\r\0\'\"\u{32}\x45"];
        for (test, answer) in tests.into_iter().zip(answers) {
            assert_eq!(
                string_lit_parser()
                    .parse(Scanner::new(test))
                    .unwrap()
                    .1
                    .value,
                answer
            );
        }
        assert!(string_lit_parser()
            .parse(Scanner::new("\"unterminated string!\n\n"))
            .is_err())
    }
    #[test]
    fn test_number() {
        let tests = [
            "3",
            "0xf4",
            "0b1_001",
            "0o123",
            "3.14",
            "0x3.f",
            "0b0.1",
            "0o1.7",
            "314e-2",
            "0.314E1",
            "0x3.fp-f",
            "0b0.1e+10",
            "0o1.7e-10",
        ];
        let answers = [
            NumberToken {
                radix: 10,
                integer: 3_u32.into(),
                exponent: None,
            },
            NumberToken {
                radix: 16,
                integer: 0xf4_u32.into(),
                exponent: None,
            },
            NumberToken {
                radix: 2,
                integer: 0b1001_u32.into(),
                exponent: None,
            },
            NumberToken {
                radix: 8,
                integer: 0o123_u32.into(),
                exponent: None,
            },
            NumberToken {
                radix: 10,
                integer: 314_u32.into(),
                exponent: Some(-2),
            },
            NumberToken {
                radix: 16,
                integer: 0x3f_u32.into(),
                exponent: Some(-1),
            },
            NumberToken {
                radix: 2,
                integer: 0b0_1_u32.into(),
                exponent: Some(-1),
            },
            NumberToken {
                radix: 8,
                integer: 0o1_7_u32.into(),
                exponent: Some(-1),
            },
            NumberToken {
                radix: 10,
                integer: 314_u32.into(),
                exponent: Some(-2),
            },
            NumberToken {
                radix: 10,
                integer: 314_u32.into(),
                exponent: Some(-2),
            },
            NumberToken {
                radix: 16,
                integer: 0x3f_u32.into(),
                exponent: Some(-1 - 0xf),
            },
            NumberToken {
                radix: 2,
                integer: 0b0_1_u32.into(),
                exponent: Some(-1 + 0b10),
            },
            NumberToken {
                radix: 8,
                integer: 0o1_7_u32.into(),
                exponent: Some(-1 - 0o10),
            },
        ];
        for (test, answer) in tests.into_iter().zip(answers) {
            assert_eq!(
                number_parser().parse(Scanner::new(test)).unwrap().1.value,
                answer
            );
        }
    }
}
