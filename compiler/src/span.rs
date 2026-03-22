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
impl Debug for Span {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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
