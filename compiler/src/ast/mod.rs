#![allow(clippy::len_without_is_empty)]

mod assignment;
mod binary;
mod expression;
mod primary;
mod primitive;
mod statement;
mod unary;

use std::{
    cell::RefCell,
    fmt,
    io::{self, BufRead},
    rc::Rc,
};

pub type Result<T> = std::result::Result<T, Error>;

/// Struct that handles iteration over chars and storing accumulated chars.
///
/// It's always mutably referenced in parser methods to advance.
/// If parser method returns None, it's expected for the Source to be rolled back to the previous state by the parsing function
/// However if it returns Err, it's expected for the Source to be at the location where the error occured.
pub struct Parser<R> {
    reader: Rc<RefCell<R>>,
    buffer: Rc<RefCell<String>>,
    offset: usize,
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
impl<R: BufRead> Parser<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader: Rc::new(RefCell::new(reader)),
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
    pub fn error_to_here(&self, from: usize, kind: ErrorKind) -> Error {
        self.error(Span::new(from, self.offset), kind)
    }
    pub fn next_and<T>(
        &mut self,
        then: impl FnOnce(SpanOf<char>) -> Option<T>,
    ) -> Result<Option<T>> {
        let prev = self.clone();
        let Some(ch) = self.next_ch()? else {
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
        let Some(ch) = self.next_ch()? else {
            return Ok(None);
        };
        if !condition(ch) {
            *self = prev;
            return Ok(None);
        }
        Ok(Some(ch))
    }
    pub fn next_ch(&mut self) -> Result<Option<SpanOf<char>>> {
        let mut buffer = self.buffer.borrow_mut();
        loop {
            match buffer.get(self.offset..).and_then(|str| str.chars().next()) {
                Some(ch) => {
                    let index = self.offset;
                    self.offset += ch.len_utf8();
                    return Ok(Some(SpanOf(Span::from_len(index, ch.len_utf8()), ch)));
                }
                None => {
                    let mut reader = self.reader.borrow_mut();
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
}

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
    #[error("Expected `(`")]
    ExpectedLeftParen,
    #[error("Expected `)`")]
    ExpectedRightParen,
    #[error("Expected `]`")]
    ExpectedRightSquare,
    #[error("Expected `}}`")]
    ExpectedRightCurly,
    #[error("Expected `:`")]
    ExpectedColon,
    #[error("Expected `=`")]
    ExpectedEq,
    #[error("Cannot use unpacking operation here")]
    UnexpectedUnpacking,
    #[error("Expected identifier")]
    ExpectedIdent,
    #[error("Expected expression")]
    ExpectedExpr,
    #[error("Array splitting already used")]
    RepeatingSplit,
    #[error("Expected `else` or `end` terminator at the end of block")]
    ExpectedElse,
    #[error("Expected `end` terminator at the end of block")]
    ExpectedEnd,
    #[error("Expected `then`")]
    ExpectedThen,
    #[error("Expected `do ... end` block")]
    ExpectedDoBlock,
    #[error("Invalid expression behind `=` operator. Only variable, property and/or indexing is allowed.")]
    InvalidAssignee,
    #[error("Expeced function body `=> [expr]` or `do ... end`")]
    ExpectedFuncBody,
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
