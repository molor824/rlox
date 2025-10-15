use crate::span::{Span, SpanOf};
use error::Error;
use scanner::Scanner;
use std::ops::Range;

mod binary;
mod error;
pub mod expression;
mod primary;
mod primitive;
pub mod scanner;
pub mod statement;
mod unary;

pub type ParseResult<T> = Result<(Scanner, T), SpanOf<Error>>;

pub struct Parser<T>(Box<dyn FnOnce(Scanner) -> ParseResult<T>>);
impl<T> Parser<T> {
    pub fn new(f: impl FnOnce(Scanner) -> ParseResult<T> + 'static) -> Self {
        Self(Box::new(f))
    }
    pub fn parse(self, scanner: Scanner) -> ParseResult<T> {
        (self.0)(scanner)
    }
}
impl<T: 'static> Parser<T> {
    pub fn span(range: Range<usize>) -> Parser<Span> {
        Parser::new(move |scanner| {
            let source = scanner.source.clone();
            Ok((scanner, Span { range, source }))
        })
    }
    pub fn span_of(range: Range<usize>, value: T) -> Parser<SpanOf<T>> {
        Self::span(range).map(move |span| span.add_value(value))
    }
    pub fn new_ok(result: T) -> Self {
        Self::new(move |scanner| Ok((scanner, result)))
    }
    pub fn new_ok_with(f: impl FnOnce(Scanner) -> T + 'static) -> Self {
        Self::new(move |scanner| Ok((scanner.clone(), f(scanner))))
    }
    pub fn new_err(error: SpanOf<Error>) -> Self {
        Self::new(move |_| Err(error))
    }
    pub fn new_err_range(range: Range<usize>, error: Error) -> Self {
        Self::new(move |scanner| Err(SpanOf::new(scanner.source, range, error)))
    }
    pub fn new_err_current(error: Error) -> Self {
        Parser::new_err_with(move |scanner| {
            SpanOf::new(scanner.source, scanner.offset..scanner.offset, error)
        })
    }
    pub fn new_err_with(f: impl FnOnce(Scanner) -> SpanOf<Error> + 'static) -> Self {
        Self::new(move |scanner| Err(f(scanner)))
    }
    pub fn map<U>(self, f: impl FnOnce(T) -> U + 'static) -> Parser<U> {
        Parser::new(move |scanner| self.parse(scanner).map(|(next, result)| (next, f(result))))
    }
    pub fn map_err(self, f: impl FnOnce(SpanOf<Error>) -> SpanOf<Error> + 'static) -> Parser<T> {
        Parser::new(move |scanner| self.parse(scanner).map_err(f))
    }
    pub fn and_then<U>(self, f: impl FnOnce(T) -> Parser<U> + 'static) -> Parser<U> {
        Parser::new(move |scanner| match self.parse(scanner.clone()) {
            Ok((next, result)) => f(result).parse(next),
            Err(err) => Err(err),
        })
    }
    pub fn then_or<U>(
        self,
        ok: impl FnOnce(T) -> Parser<U> + 'static,
        error: impl FnOnce(SpanOf<Error>) -> Parser<U> + 'static,
    ) -> Parser<U> {
        Parser::new(move |scanner| match self.parse(scanner.clone()) {
            Ok((next, result)) => ok(result).parse(next),
            Err(err) => error(err).parse(scanner),
        })
    }
    pub fn or_else(self, f: impl FnOnce(SpanOf<Error>) -> Parser<T> + 'static) -> Parser<T> {
        Parser::new(move |scanner| match self.parse(scanner.clone()) {
            Err(e) => f(e).parse(scanner),
            n => n,
        })
    }
    pub fn optional(self) -> Parser<Option<T>> {
        Parser::new(move |scanner| match self.parse(scanner.clone()) {
            Ok((next, result)) => Ok((next, Some(result))),
            Err(_) => Ok((scanner, None)),
        })
    }
    pub fn fold<U: 'static>(
        self,
        mut parser: impl FnMut() -> Parser<U> + 'static,
        mut accumulator: impl FnMut(T, U) -> T + 'static,
    ) -> Self {
        self.and_then(|mut init| {
            Parser::new(move |mut scanner| loop {
                match parser().parse(scanner.clone()) {
                    Ok((next, value)) => {
                        init = accumulator(init, value);
                        scanner = next;
                    }
                    Err(_) => return Ok((scanner, init)),
                }
            })
        })
    }
}
fn next_char_parser() -> Parser<SpanOf<char>> {
    Parser::new(|scanner| match scanner.clone().next() {
        Some((next, ch, offset)) => {
            Parser::span_of(offset..(offset + ch.len_utf8()), ch).parse(next)
        }
        None => Parser::new_err_current(Error::Eof).parse(scanner),
    })
}
fn string_eq_parser(string: &'static str) -> Parser<SpanOf<&'static str>> {
    Parser::new(move |mut scanner| {
        let offset = scanner.offset;
        let end_offset = scanner.offset + string.len();
        while scanner.source.borrow().len() < end_offset {
            let Some(ch) = scanner.iter.borrow_mut().next() else {
                break;
            };
            scanner.source.borrow_mut().push(ch);
        }
        if scanner.source.borrow()[offset..].starts_with(string) {
            scanner.offset += string.len();
            let source = scanner.source.clone();
            Ok((scanner, SpanOf::new(source, offset..end_offset, string)))
        } else {
            Parser::new_err_current(Error::ExpectedString(string.to_string())).parse(scanner)
        }
    })
}
fn strings_eq_parser(strings: &'static [&'static str]) -> Parser<SpanOf<&'static str>> {
    Parser::new(move |scanner| {
        for &str in strings {
            if let Ok(a) = string_eq_parser(str).parse(scanner.clone()) {
                return Ok(a);
            }
        }
        Parser::new_err_current(Error::ExpectedStrings(
            strings.into_iter().map(|s| s.to_string()).collect(),
        ))
        .parse(scanner)
    })
}
fn char_eq_parser(ch: char) -> Parser<SpanOf<char>> {
    next_char_parser().and_then(move |char| {
        if char.value == ch {
            Parser::new_ok(char)
        } else {
            Parser::new_err(char.map(|_| Error::ExpectedChar(ch)))
        }
    })
}
fn chars_eq_parser(chars: &'static [char]) -> Parser<SpanOf<char>> {
    next_char_parser().and_then(move |char| {
        if chars.contains(&char.value) {
            Parser::new_ok(char)
        } else {
            Parser::new_err(char.map(|_| Error::ExpectedChars(chars.iter().cloned().collect())))
        }
    })
}
fn char_match_parser(f: impl FnOnce(char) -> bool + 'static) -> Parser<SpanOf<char>> {
    next_char_parser().and_then(move |ch| {
        if f(ch.value) {
            Parser::new_ok(ch)
        } else {
            Parser::new_err(ch.map(|_| Error::CharNotMatch))
        }
    })
}
