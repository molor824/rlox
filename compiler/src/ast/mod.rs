#![allow(clippy::len_without_is_empty)]

pub mod assignment;
pub mod binary;
pub mod expression;
pub mod primary;
pub mod primitive;
pub mod statement;
pub mod unary;

use std::{cell::RefCell, fmt, io::BufRead, rc::Rc};

use crate::{
    error::{Error, ErrorKind, Result},
    span::{Span, SpanOf},
};

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
