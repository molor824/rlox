use error::{Error, ErrorCode};
use scanner::Scanner;

use crate::span::SpanOf;

mod error;
mod primitive;
mod scanner;

pub type ParseResult<T> = Result<Option<(Scanner, T)>, Error>;

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
        Self::new(move |scanner| Ok(Some((scanner, result))))
    }
    pub fn new_none() -> Self {
        Self::new(move |_| Ok(None))
    }
    pub fn new_err(code: SpanOf<ErrorCode>) -> Self {
        Self::new(move |scanner| Err(Error::new(scanner.source, code)))
    }
    pub fn map<U>(self, f: impl FnOnce(T) -> U + 'static) -> Parser<U> {
        Parser::new(move |scanner| {
            self.parse(scanner)
                .map(|result| result.map(|(next, result)| (next, f(result))))
        })
    }
    pub fn map_err(self, f: impl FnOnce(Error) -> Error + 'static) -> Parser<T> {
        Parser::new(move |scanner| self.parse(scanner).map_err(f))
    }
    pub fn and_then<U>(self, f: impl FnOnce(T) -> Parser<U> + 'static) -> Parser<U> {
        Parser::new(move |scanner| match self.parse(scanner)? {
            Some((next, result)) => f(result).parse(next),
            None => Ok(None),
        })
    }
    pub fn or_else(self, f: impl FnOnce() -> Parser<T> + 'static) -> Parser<T> {
        Parser::new(move |scanner| match self.parse(scanner.clone())? {
            Some(result) => Ok(Some(result)),
            None => f().parse(scanner),
        })
    }
    pub fn fold<U: 'static>(
        mut parser: impl FnMut() -> Parser<T> + 'static,
        mut accumulator: impl FnMut(U, T) -> U + 'static,
        mut init: U,
    ) -> Parser<U> {
        Parser::new(move |mut scanner| loop {
            match parser().parse(scanner.clone())? {
                Some((next, result)) => {
                    scanner = next;
                    init = accumulator(init, result);
                }
                None => return Ok(Some((scanner, init))),
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
