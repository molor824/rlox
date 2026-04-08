pub mod primitive;

use std::{
    cell::RefCell,
    fmt,
    io::{self, BufRead, BufReader, Read},
    rc::Rc,
};

use crate::cache::Cache;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum ErrorKind {
    #[error("IOError: {0}")]
    IoError(#[from] io::Error),
    #[error("Character {0} is not allowed in number with base {1}")]
    NotDigit(char, u32),
    #[error("Missing integer")]
    MissingInteger,
    #[error("Missing exponent")]
    MissingExponent,
    #[error("Invalid escape sequence")]
    InvalidEscape,
    #[error("Invalid unicode")]
    InvalidUnicode,
    #[error("String literal unterminated")]
    UnterminatedString
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
        f.debug_struct("Error")
            .field("kind", &self.kind)
            .field("span", &&self.source.borrow()[self.span.start..self.span.end()])
            .finish()
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut col = 0;
        let mut row = 1;
        let mut prev_ch = '\0';
        let source = self.source.borrow();
        for (i, ch) in source.char_indices() {
            if prev_ch == '\n' || prev_ch == '\r' {
                col = 1;
                if prev_ch == '\n' {
                    row += 1;
                }
            } else {
                col += 1;
            }
            if i == self.span.start {
                return write!(f, "Error [line:{}, col:{}]: {}", row, col, self.kind);
            }
            prev_ch = ch;
        }
        write!(f, "Error: {}", self.kind)
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
    pub ident_cache: Rc<RefCell<Cache<String>>>,
    pub string_cache: Rc<RefCell<Cache<String>>>,
    pub offset: usize,
}
impl<R> Clone for Parser<R> {
    fn clone(&self) -> Self {
        Self {
            reader: self.reader.clone(),
            buffer: self.buffer.clone(),
            ident_cache: self.ident_cache.clone(),
            string_cache: self.string_cache.clone(),
            offset: self.offset,
        }
    }
}
impl<R: Read> Parser<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader: Rc::new(RefCell::new(BufReader::new(reader))),
            buffer: Rc::new(RefCell::new(String::new())),
            ident_cache: Rc::new(RefCell::new(Cache::new())),
            string_cache: Rc::new(RefCell::new(Cache::new())),
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
    pub fn next_and<T>(&mut self, then: impl FnOnce((usize, char)) -> Option<T>) -> Result<Option<T>> {
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
    pub fn next_if(&mut self, condition: impl FnOnce((usize, char)) -> bool) -> Result<Option<(usize, char)>> {
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
    pub fn next(&mut self) -> Result<Option<(usize, char)>> {
        let mut buffer = self.buffer.borrow_mut();
        let mut reader = self.reader.borrow_mut();
        loop {
            match buffer.get(self.offset..).and_then(|str| str.chars().next()) {
                Some(ch) => {
                    let index = self.offset;
                    self.offset += ch.len_utf8();
                    return Ok(Some((index, ch)));
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub len: usize
}
impl Span {
    pub const fn new(start: usize, len: usize) -> Self {
        Self { start, len }
    }
    pub const fn from_char_offset(ch: (usize, char)) -> Self {
        Self { start: ch.0, len: ch.1.len_utf8() }
    }
    pub const fn from_end(start: usize, end: usize) -> Self {
        Self { start, len: end - start }
    }
    pub const fn end(&self) -> usize {
        self.start + self.len
    }
    pub fn with_end(self, new_end: usize) -> Self {
        Self::from_end(self.start, new_end)
    }
    pub fn concat(self, other: Span) -> Span {
        let start = self.start.min(other.start);
        let end = self.end().max(other.end());
        Span::from_end(start, end)
    }
}
