use std::{cell::RefCell, io::BufRead, ops::Range, rc::Rc};

use crate::{ast, span::{Span, SpanOf}};

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
    pub fn ast_error(&self, kind: ast::ErrorKind, range: Range<usize>) -> ast::Error {
        ast::Error {
            kind,
            span: self.span(range),
        }
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
    pub fn next_if(
        &mut self,
        condition: impl FnOnce((usize, char)) -> bool,
    ) -> ast::Result<Option<(usize, char)>> {
        let offset = self.offset;
        match self.next()? {
            Some(ch) if condition(ch) => Ok(Some(ch)),
            _ => {
                self.offset = offset;
                Ok(None)
            }
        }
    }
    pub fn next_and<U>(
        &mut self,
        next: impl FnOnce((usize, char)) -> Option<U>,
    ) -> ast::Result<Option<U>> {
        let offset = self.offset;
        match self.next()?.and_then(next) {
            Some(v) => Ok(Some(v)),
            _ => {
                self.offset = offset;
                Ok(None)
            }
        }
    }
    pub fn next(&mut self) -> ast::Result<Option<(usize, char)>> {
        loop {
            let mut buffer = self.buffer.borrow_mut();
            let mut reader = self.reader.borrow_mut();
            match buffer.get(self.offset..).and_then(|str| str.chars().next()) {
                Some(ch) => {
                    let index = self.offset;
                    self.offset += ch.len_utf8();
                    return Ok(Some((index, ch)));
                }
                None => {
                    let len = buffer.len();
                    if reader
                        .read_line(&mut buffer)
                        .map_err(|e| self.ast_error(e.into(), len..(buffer.len())))?
                        == 0
                    {
                        return Ok(None);
                    }
                }
            }
        }
    }
}
