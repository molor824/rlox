use crate::span::{Span, SpanOf};

use super::{
    error::{Error, ErrorCode},
    Parser,
};

fn char_parser() -> Parser<SpanOf<char>> {
    Parser::new(|scanner| match scanner.clone().next() {
        Some((next, ch, offset)) => Ok((next, SpanOf(Span(offset, offset + ch.len_utf8()), ch))),
        None => Err(Error::new(
            scanner.source,
            SpanOf(Span::fill(scanner.offset), ErrorCode::Eof),
        )),
    })
}
fn char_match(f: impl FnOnce(char) -> bool + 'static) -> Parser<SpanOf<char>> {
    char_parser().and_then(move |SpanOf(span, ch)| {
        if f(ch) {
            Parser::new_ok(SpanOf(span, ch))
        } else {
            Parser::new_err(SpanOf(span, ErrorCode::UnexpectedChar(ch)))
        }
    })
}

fn digit_parser(radix: u32) -> Parser<SpanOf<u8>> {
    char_parser().and_then(move |SpanOf(span, ch)| match ch.to_digit(radix) {
        Some(d) => Parser::new_ok(SpanOf(span, d as u8)),
        None => Parser::new_err(SpanOf(span, ErrorCode::UnexpectedChar(ch))),
    })
}
fn integer_parser(radix: u32) -> Parser<SpanOf<Vec<u8>>> {
    Parser::fold::<Option<SpanOf<Vec<u8>>>>(
        move || digit_parser(radix),
        |acc, SpanOf(dspan, digit)| match acc {
            Some(SpanOf(span, mut vec)) => {
                vec.push(digit);
                Some(SpanOf(span.concat(dspan), vec))
            }
            None => Some(SpanOf(dspan, vec![digit])),
        },
        None,
    )
    .and_then(move |integer| match integer {
        Some(i) => Parser::new_ok(i),
        None => Parser::new_err_with(|scanner| SpanOf(Span::fill(scanner.offset), ErrorCode::Eof)),
    })
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
fn decimal_parser(radix: u32) -> Parser<SpanOf<NumberToken>> {
    integer_parser(radix).and_then(move |SpanOf(whole_span, whole)| {
        char_match(|ch| ch == '.')
            .and_then(move |SpanOf(dot, _)| {
                integer_parser(radix)
                    .map(move |SpanOf(frac_span, frac)| SpanOf(dot.concat(frac_span), frac))
                    .or_else(move |_| Parser::new_ok(SpanOf(dot, vec![])))
            })
            .map({
                let whole = whole.clone();
                move |SpanOf(frac_span, frac)| {
                    SpanOf(
                        frac_span.concat(whole_span),
                        NumberToken {
                            radix,
                            dot_index: Some(whole.len() as i32),
                            digits: [whole, frac].concat(),
                        },
                    )
                }
            })
            .or_else(move |_| {
                Parser::new_ok(SpanOf(
                    whole_span,
                    NumberToken {
                        radix,
                        digits: whole,
                        dot_index: None,
                    },
                ))
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
            integer_parser(10).parse(Scanner::new("351")).unwrap().1 .1,
            [3, 5, 1]
        );
        assert_eq!(
            integer_parser(16).parse(Scanner::new("Ff3")).unwrap().1 .1,
            [0xf, 0xf, 3]
        );
    }

    #[test]
    fn test_decimal_parser() {
        assert_eq!(
            decimal_parser(10)
                .parse(Scanner::new("3.135"))
                .unwrap()
                .1
                 .1,
            NumberToken {
                radix: 10,
                digits: [3, 1, 3, 5].to_vec(),
                dot_index: Some(1)
            }
        );
        assert_eq!(
            decimal_parser(16)
                .parse(Scanner::new("A.3FF"))
                .unwrap()
                .1
                 .1,
            NumberToken {
                radix: 16,
                digits: [0xA, 3, 0xF, 0xF].to_vec(),
                dot_index: Some(1)
            }
        );
    }
}
