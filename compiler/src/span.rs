#[derive(Debug, Clone, Copy)]
pub struct Span<T> {
    pub start: usize,
    pub end: usize,
    pub value: T,
}
impl<T> Span<T> {
    pub const fn new(start: usize, end: usize, value: T) -> Self {
        Self { start, end, value }
    }
    pub fn combine<U, O>(self, other: Span<U>, f: impl FnOnce(T, U) -> O) -> Span<O> {
        Span::new(
            usize::min(self.start, other.start),
            usize::max(self.end, other.end),
            f(self.value, other.value),
        )
    }
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Span<U> {
        Span::new(self.start, self.end, f(self.value))
    }
}
