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
