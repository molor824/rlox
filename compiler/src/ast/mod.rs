use error::Error;
use scanner::Scanner;

use crate::span::Span;

mod binary;
mod error;
pub mod expression;
mod primary;
mod primitive;
mod scanner;
mod unary;

pub type ParseResult<T> = Result<(Scanner, T), Span<Error>>;

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
    pub fn new_err(error: Span<Error>) -> Self {
        Self::new(move |_| Err(error))
    }
    pub fn new_err_with(f: impl FnOnce(Scanner) -> Span<Error> + 'static) -> Self {
        Self::new(move |scanner| Err(f(scanner)))
    }
    pub fn map<U>(self, f: impl FnOnce(T) -> U + 'static) -> Parser<U> {
        Parser::new(move |scanner| self.parse(scanner).map(|(next, result)| (next, f(result))))
    }
    pub fn map_err(self, f: impl FnOnce(Span<Error>) -> Span<Error> + 'static) -> Parser<T> {
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
        error: impl FnOnce(Span<Error>) -> Parser<U> + 'static,
    ) -> Parser<U> {
        Parser::new(move |scanner| match self.parse(scanner.clone()) {
            Ok((next, result)) => ok(result).parse(next),
            Err(err) => error(err).parse(scanner),
        })
    }
    pub fn or_else(self, f: impl FnOnce(Span<Error>) -> Parser<T> + 'static) -> Parser<T> {
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
        None => Err(Span::from_len(scanner.offset, 0, Error::Eof)),
    })
}
fn string_eq_parser(string: &'static str) -> Parser<Span<&'static str>> {
    Parser::new(move |Scanner { source, offset }| {
        if source[offset..].starts_with(&string) {
            Ok((
                Scanner {
                    offset: offset + string.len(),
                    source,
                },
                Span::new(offset, offset + string.len(), string),
            ))
        } else {
            Err(Span::from_len(
                offset,
                0,
                Error::ExpectedString(string.into()),
            ))
        }
    })
}
fn strings_eq_parser(strings: &'static [&'static str]) -> Parser<Span<&'static str>> {
    Parser::new(move |Scanner { source, offset }| {
        match strings
            .into_iter()
            .find(|&s| source[offset..].starts_with(s))
        {
            Some(&str) => Ok((
                Scanner {
                    source,
                    offset: offset + str.len(),
                },
                Span::new(offset, offset + str.len(), str),
            )),
            None => Err(Span::from_len(
                offset,
                0,
                Error::ExpectedStrings(strings.iter().map(|s| s.to_string()).collect()),
            )),
        }
    })
}
fn char_eq_parser(ch: char) -> Parser<Span<char>> {
    next_char_parser().and_then(move |char| {
        if char.value == ch {
            Parser::new_ok(char)
        } else {
            Parser::new_err(char.map(|_| Error::ExpectedChar(ch)))
        }
    })
}
fn chars_eq_parser(chars: &'static [char]) -> Parser<Span<char>> {
    next_char_parser().and_then(move |char| {
        if chars.contains(&char.value) {
            Parser::new_ok(char)
        } else {
            Parser::new_err(char.map(|_| Error::ExpectedChars(chars.iter().cloned().collect())))
        }
    })
}
fn char_match_parser(f: impl FnOnce(char) -> bool + 'static) -> Parser<Span<char>> {
    next_char_parser().and_then(move |ch| {
        if f(ch.value) {
            Parser::new_ok(ch)
        } else {
            Parser::new_err(ch.map(|_| Error::CharNotMatch))
        }
    })
}
