mod primary;
mod primitive;

use std::{
    cell::RefCell,
    fmt,
    io::{self, BufRead, BufReader, Read},
    rc::Rc,
};

use crate::{
    ast::primitive::{CachedString, Number},
    cache::Cache,
};

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
    UnterminatedString,
    #[error("Expected `)`")]
    ExpectedRightParen,
    #[error("Expected `]`")]
    ExpectedRightSquare,
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
            .field(
                "span",
                &(
                    self.span.start..self.span.end,
                    &self.source.borrow()[self.span.start..self.span.end],
                ),
            )
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
    reader: Rc<RefCell<BufReader<R>>>,
    buffer: Rc<RefCell<String>>,
    ident_cache: Rc<RefCell<Cache<str>>>,
    string_cache: Rc<RefCell<Cache<str>>>,
    offset: usize,
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
    pub fn get_ident_id(&self, ident: &str) -> usize {
        let mut ident_cache = self.ident_cache.borrow_mut();
        if let Some(id) = ident_cache.get_id(ident) {
            id
        } else {
            ident_cache.insert_rc(Rc::from(ident))
        }
    }
    pub fn get_string_id(&self, string: &str) -> usize {
        let mut string_cache = self.string_cache.borrow_mut();
        if let Some(id) = string_cache.get_id(string) {
            id
        } else {
            string_cache.insert_rc(Rc::from(string))
        }
    }
    pub fn error(&self, span: Span, kind: ErrorKind) -> Error {
        Error {
            kind,
            span,
            source: self.buffer.clone(),
        }
    }
    pub fn error_to_here(&self, from: usize, kind: ErrorKind) -> Error {
        self.error(Span::new(from, self.offset), kind)
    }
    pub fn next_and<T>(
        &mut self,
        then: impl FnOnce(SpanOf<char>) -> Option<T>,
    ) -> Result<Option<T>> {
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
    pub fn next_if(
        &mut self,
        condition: impl FnOnce(SpanOf<char>) -> bool,
    ) -> Result<Option<SpanOf<char>>> {
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
                    return Ok(Some(SpanOf(Span::from_len(index, ch.len_utf8()), ch)));
                }
                None => {
                    if reader
                        .read_line(&mut buffer)
                        .map_err(|e| self.error(Span::from_len(self.offset, 0), e.into()))?
                        == 0
                    {
                        return Ok(None);
                    }
                }
            }
        }
    }
    // Is used for recursive expressions
    // NOTE: Update when the top most expression implementation changes
    pub fn next_expression(&mut self, skip_newline: bool) -> Result<Option<Expression>> {
        self.next_primary(skip_newline)
    }
}

#[derive(Debug, Clone)]
pub enum Expression {
    Ident(SpanOf<CachedString>),
    String(SpanOf<CachedString>),
    Number(SpanOf<Number>),
    Group(SpanOf<Box<Expression>>),
    Tuple(SpanOf<Vec<Expression>>),
    Array(SpanOf<Vec<Expression>>),
    Boolean(SpanOf<bool>),
}
impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Group(group) => write!(f, "({})", group.1),
            Self::Ident(ident) => write!(f, "{}", ident.1),
            Self::Number(number) => write!(f, "{}", number.1),
            Self::String(string) => write!(f, "{:?}", string.1.get_str()),
            Self::Tuple(tuple) => write!(
                f,
                "({})",
                tuple
                    .1
                    .iter()
                    .map(|expr| format!("{},", expr))
                    .collect::<String>()
            ),
            Self::Boolean(boolean) => write!(f, "{}", boolean.1),
            Self::Array(arr) => write!(
                f,
                "[{}]",
                arr.1
                    .iter()
                    .map(|expr| format!("{},", expr))
                    .collect::<String>()
            ),
        }
    }
}
impl GetSpan for Expression {
    fn span(&self) -> Span {
        match self {
            Self::Group(group) => group.0,
            Self::Ident(ident) => ident.0,
            Self::Number(number) => number.0,
            Self::String(string) => string.0,
            Self::Tuple(tuple) => tuple.0,
            Self::Boolean(boolean) => boolean.0,
            Self::Array(array) => array.0,
        }
    }
}

pub trait GetSpan {
    fn span(&self) -> Span;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}
impl Span {
    pub const fn from_len(start: usize, len: usize) -> Self {
        Self {
            start,
            end: start + len,
        }
    }
    pub const fn from_char_offset(ch: (usize, char)) -> Self {
        Self::from_len(ch.0, ch.1.len_utf8())
    }
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
    pub const fn len(&self) -> usize {
        self.end - self.start
    }
    pub fn with_end(self, new_end: usize) -> Self {
        Self::new(self.start, new_end)
    }
    pub fn concat(self, other: Span) -> Span {
        let start = self.start.min(other.start);
        let end = self.end.max(other.end);
        Span::new(start, end)
    }
}
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct SpanOf<T>(pub Span, pub T);
impl<T> SpanOf<T> {
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> SpanOf<U> {
        SpanOf(self.0, f(self.1))
    }
    pub fn concat<U, Q>(self, other: SpanOf<U>, f: impl FnOnce(T, U) -> Q) -> SpanOf<Q> {
        SpanOf(self.0.concat(other.0), f(self.1, other.1))
    }
    pub fn concat_span(mut self, other: Span) -> Self {
        self.0 = self.0.concat(other);
        self
    }
}
