pub mod primary;

use std::{
    cell::RefCell,
    fmt,
    io::{self, BufRead, BufReader, Read},
    rc::Rc,
};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum ErrorKind {
    #[error("IOError: {0}")]
    IoError(#[from] io::Error),
    #[error("Character {0} is not allowed in number with base {1}")]
    NotDigit(char, u32),
    #[error("Missing integer")]
    IntegerMissing,
}
#[derive(thiserror::Error)]
pub struct Error {
    #[source]
    pub kind: ErrorKind,
    pub span: Span,
    pub source: Rc<RefCell<String>>,
}
impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

/// Struct that handles iteration over chars and storing accumulated chars.
///
/// It's always mutably referenced in parser methods to advance.
/// If parser method returns None, it's expected for the Source to be rolled back to the previous state by the parsing function
/// However if it returns Err, it's expected for the Source to be at the location where the error occured.
pub struct Parser<R: ?Sized> {
    pub reader: Rc<RefCell<BufReader<R>>>,
    pub buffer: Rc<RefCell<String>>,
    pub offset: usize,
}
impl<R> Clone for Parser<R> {
    fn clone(&self) -> Self {
        Self {
            reader: self.reader.clone(),
            buffer: self.buffer.clone(),
            offset: self.offset,
        }
    }
}
impl<R: Read> Parser<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader: Rc::new(RefCell::new(BufReader::new(reader))),
            buffer: Rc::new(RefCell::new(String::new())),
            offset: 0,
        }
    }
    pub fn error(&self, span: Span, kind: ErrorKind) -> Error {
        Error {
            kind,
            span,
            source: self.buffer.clone(),
        }
    }
    pub fn error_here(&self, kind: ErrorKind) -> Error {
        self.error(Span::new(self.offset, 0), kind)
    }
    pub fn next_and<T>(&mut self, then: impl FnOnce(SpanOf<char>) -> Option<T>) -> Result<Option<T>> {
        let prev = self.clone();
        let Some(ch) = self.next()? else {
            return Ok(None);
        };
        let Some(v) = then(ch) else {
            *self = prev;
            return Ok(None);
        };
        Ok(Some(v))
    }
    pub fn next_if(&mut self, condition: impl FnOnce(SpanOf<char>) -> bool) -> Result<Option<SpanOf<char>>> {
        let prev = self.clone();
        let Some(ch) = self.next()? else {
            return Ok(None);
        };
        if !condition(ch.clone()) {
            *self = prev;
            return Ok(None);
        }
        Ok(Some(ch))
    }
    pub fn next(&mut self) -> Result<Option<SpanOf<char>>> {
        let mut buffer = self.buffer.borrow_mut();
        let mut reader = self.reader.borrow_mut();
        loop {
            match buffer.get(self.offset..).and_then(|str| str.chars().next()) {
                Some(ch) => {
                    let index = self.offset;
                    self.offset += ch.len_utf8();
                    return Ok(Some(SpanOf(Span::new(index, ch.len_utf8()), ch)));
                }
                None => {
                    if reader.read_line(&mut buffer).map_err(|e| self.error_here(e.into()))? == 0 {
                        return Ok(None);
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub len: usize
}
impl Span {
    pub const fn new(start: usize, len: usize) -> Self {
        Self { start, len }
    }
    pub const fn from_end(start: usize, end: usize) -> Self {
        Self { start, len: end - start }
    }
    pub const fn end(&self) -> usize {
        self.start + self.len
    }
    pub fn concat(self, other: Span) -> Span {
        let start = self.start.min(other.start);
        let end = self.end().max(other.end());
        Span::from_end(start, end)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanOf<T>(pub Span, pub T);
impl<T> SpanOf<T> {
    pub const fn start(&self) -> usize {
        self.0.start
    }
    pub const fn len(&self) -> usize {
        self.0.len
    }
    pub const fn end(&self) -> usize {
        self.0.end()
    }
    pub fn map<U>(self, map: impl FnOnce(T) -> U) -> SpanOf<U> {
        SpanOf(self.0, map(self.1))
    }
    pub fn concat<U, R>(self, other: SpanOf<U>, concat: impl FnOnce(T, U) -> R) -> SpanOf<R> {
        SpanOf(self.0.concat(other.0), concat(self.1, other.1))
    }
    pub fn concat_span(self, other: Span) -> Self {
        Self(self.0.concat(other), self.1)
    }
}
