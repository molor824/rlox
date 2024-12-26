use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct Scanner {
    pub source: Rc<str>,
    pub offset: usize,
}
impl Scanner {
    pub fn new(source: impl Into<Rc<str>>) -> Self {
        Self {
            source: source.into(),
            offset: 0,
        }
    }
    pub fn next(self) -> Option<(Scanner, char, usize)> {
        let Scanner { source, offset } = self;
        source[offset..].chars().next().map(|ch| {
            (
                Scanner {
                    source,
                    offset: offset + ch.len_utf8(),
                },
                ch,
                offset,
            )
        })
    }
}
