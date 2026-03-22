pub mod primary;

use std::{
    cell::{Ref, RefCell},
    fmt,
    io::{self, BufRead},
    ops::Range,
    rc::Rc,
};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum ErrorKind {
    #[error("IOError: {0}")]
    IoError(#[from] io::Error),
    #[error("Character {0} is not allowed in number with base {1}")]
    NotDigit(char, u32),
}
#[derive(thiserror::Error, Debug)]
pub struct Error {
    #[source]
    pub kind: ErrorKind,
    pub span: Span,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

#[derive(Clone)]
/// Struct that handles iteration over chars and storing accumulated chars.
///
/// It's always mutably referenced in parser methods to advance.
/// If parser method returns None, it's expected for the Source to be rolled back to the previous state by the parsing function
/// However if it returns Err, it's expected for the Source to be at the location where the error occured.
pub struct Source {
    pub reader: Rc<RefCell<dyn BufRead>>,
    pub buffer: Rc<RefCell<String>>,
    pub offset: usize,
}
impl Source {
    pub fn new(reader: Rc<RefCell<dyn BufRead>>) -> Self {
        Self {
            reader,
            buffer: Rc::new(RefCell::new(String::new())),
            offset: 0,
        }
    }
    pub fn error(&self, kind: ErrorKind, range: Range<usize>) -> Error {
        Error {
            kind,
            span: self.span(range),
        }
    }
    pub fn error_here(&self, kind: ErrorKind) -> Error {
        self.error(kind, self.offset..self.offset)
    }
    pub fn span(&self, range: Range<usize>) -> Span {
        Span {
            source: self.buffer.clone(),
            range,
        }
    }
    pub fn span_of<T>(&self, range: Range<usize>, of: T) -> SpanOf<T> {
        SpanOf(self.span(range), of)
    }
    pub fn next_and<T>(&mut self, then: impl FnOnce((usize, char)) -> Option<T>) -> Result<Option<T>> {
        let prev = self.offset;
        let Some(ch) = self.next()? else {
            return Ok(None);
        };
        let Some(v) = then(ch) else {
            self.offset = prev;
            return Ok(None);
        };
        Ok(Some(v))
    }
    pub fn next_if(&mut self, condition: impl FnOnce((usize, char)) -> bool) -> Result<Option<(usize, char)>> {
        let prev = self.offset;
        let Some(ch) = self.next()? else {
            return Ok(None);
        };
        if !condition(ch) {
            self.offset = prev;
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

#[derive(Clone, PartialEq, Eq)]
pub struct Span {
    pub range: Range<usize>,
    pub source: Rc<RefCell<String>>,
}
impl Span {
    pub fn as_slice<'a>(&'a self) -> Ref<'a, str> {
        Ref::map(self.source.borrow(), |s| &s[self.range.clone()])
    }
    pub const fn start(&self) -> usize {
        self.range.start
    }
    pub const fn end(&self) -> usize {
        self.range.end
    }
    pub fn concat(self, other: Span) -> Span {
        let start = self.start().min(other.start());
        let end = self.end().max(other.end());
        Span {
            range: start..end,
            source: self.source,
        }
    }
}
impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Span").field(&self.as_slice()).finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct SpanOf<T>(pub Span, pub T);
impl<T> SpanOf<T> {
    pub const fn start(&self) -> usize {
        self.0.start()
    }
    pub const fn end(&self) -> usize {
        self.0.end()
    }
    pub fn concat<U, R>(self, other: SpanOf<U>, concat: impl FnOnce(T, U) -> R) -> SpanOf<R> {
        SpanOf(self.0.concat(other.0), concat(self.1, other.1))
    }
}
