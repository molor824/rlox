use error::{Error, ErrorCode};
use scanner::Scanner;

use crate::span::Span;

mod error;
mod expression;
mod primary;
mod primitive;
mod scanner;
mod unary;

pub type ParseResult<T> = Result<(Scanner, T), Error>;

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
    pub fn new_ok(result: T) -> Self {
        Self::new(move |scanner| Ok((scanner, result)))
    }
    pub fn new_ok_with(f: impl FnOnce(Scanner) -> T + 'static) -> Self {
        Self::new(move |scanner| Ok((scanner.clone(), f(scanner))))
    }
    pub fn new_err(code: Span<ErrorCode>) -> Self {
        Self::new(move |scanner| Err(Error::new(scanner.source, code)))
    }
    pub fn new_err_with(f: impl FnOnce(Scanner) -> Error + 'static) -> Self {
        Self::new(move |scanner| Err(f(scanner)))
    }
    pub fn map<U>(self, f: impl FnOnce(T) -> U + 'static) -> Parser<U> {
        Parser::new(move |scanner| self.parse(scanner).map(|(next, result)| (next, f(result))))
    }
    pub fn map_err(self, f: impl FnOnce(Error) -> Error + 'static) -> Parser<T> {
        Parser::new(move |scanner| self.parse(scanner).map_err(f))
    }
    pub fn and_then<U>(self, f: impl FnOnce(T) -> Parser<U> + 'static) -> Parser<U> {
        Parser::new(move |scanner| match self.parse(scanner.clone()) {
            Ok((next, result)) => f(result).parse(next),
            Err(err) => Err(err),
        })
    }
    pub fn or_else(self, f: impl FnOnce(Error) -> Parser<T> + 'static) -> Parser<T> {
        Parser::new(move |scanner| match self.parse(scanner.clone()) {
            Err(e) => f(e).parse(scanner),
            n => n,
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
