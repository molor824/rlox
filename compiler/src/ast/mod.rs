use error::{Error, ErrorCode};
use scanner::Scanner;

use crate::span::SpanOf;

mod error;
mod primitive;
mod scanner;

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
    pub fn new_err(code: SpanOf<ErrorCode>) -> Self {
        Self::new(move |scanner| Err(Error::new(scanner.source, code)))
    }
    pub fn new_err_with(func: impl FnOnce(Scanner) -> SpanOf<ErrorCode> + 'static) -> Self {
        Self::new(move |scanner| Err(Error::new(scanner.source.clone(), func(scanner))))
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
        mut parser: impl FnMut() -> Parser<T> + 'static,
        mut accumulator: impl FnMut(U, T) -> U + 'static,
        mut init: U,
    ) -> Parser<U> {
        Parser::new(move |mut scanner| loop {
            match parser().parse(scanner.clone()) {
                Ok((next, result)) => {
                    scanner = next;
                    init = accumulator(init, result);
                }
                Err(_) => return Ok((scanner, init)),
            }
        })
    }
    pub fn reduce(
        mut parser: impl FnMut() -> Parser<T> + 'static,
        accumulator: impl FnMut(T, T) -> T + 'static,
    ) -> Parser<T> {
        parser().and_then(move |init| Parser::fold(parser, accumulator, init))
    }
}
