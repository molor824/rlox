use std::cell::{Ref, RefCell};
use std::fmt::{Debug, Formatter};
use std::ops::Range;
use std::rc::Rc;

#[derive(Clone, PartialEq, Eq)]
pub struct Span {
    pub range: Range<usize>,
    pub source: Rc<RefCell<String>>,
}
impl Span {
    pub const fn new(source: Rc<RefCell<String>>, range: Range<usize>) -> Self {
        Self { range, source }
    }
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
    pub fn add_value<T>(self, value: T) -> SpanOf<T> {
        SpanOf { span: self, value }
    }
}
impl Debug for Span {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Span").field(&self.as_slice()).finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanOf<T> {
    pub span: Span,
    pub value: T,
}
impl<T> SpanOf<T> {
    pub const fn new(source: Rc<RefCell<String>>, range: Range<usize>, value: T) -> Self {
        Self {
            span: Span::new(source, range),
            value,
        }
    }
    pub const fn start(&self) -> usize {
        self.span.start()
    }
    pub const fn end(&self) -> usize {
        self.span.end()
    }
    pub fn combine<U, O>(self, other: SpanOf<U>, f: impl FnOnce(T, U) -> O) -> SpanOf<O> {
        SpanOf {
            span: self.span.concat(other.span),
            value: f(self.value, other.value),
        }
    }
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> SpanOf<U> {
        SpanOf {
            span: self.span,
            value: f(self.value),
        }
    }
    pub fn replace<U>(self, value: U) -> SpanOf<U> {
        SpanOf {
            span: self.span,
            value,
        }
    }
}
