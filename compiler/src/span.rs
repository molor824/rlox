#[derive(Debug, Clone, Copy)]
pub struct Span(pub usize, pub usize);

impl Span {
    pub fn fill(v: usize) -> Self {
        Self(v, v)
    }
    pub fn concat(self, other: Self) -> Self {
        Self(usize::min(self.0, other.0), usize::max(self.1, other.1))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SpanOf<T>(pub Span, pub T);
