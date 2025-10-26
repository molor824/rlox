use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
pub struct Scanner {
    pub source: Rc<RefCell<String>>,
    pub iter: Rc<RefCell<dyn Iterator<Item = char>>>,
    pub offset: usize,
}
impl Scanner {
    pub fn new(iter: impl IntoIterator<Item = char> + 'static) -> Self {
        let iter = iter.into_iter();
        let source = String::with_capacity(iter.size_hint().0);
        Self {
            iter: Rc::new(RefCell::new(iter)),
            source: Rc::new(RefCell::new(source)),
            offset: 0,
        }
    }
    pub fn next(mut self) -> Option<(Scanner, char, usize)> {
        while self.offset >= self.source.borrow().len() {
            let Some(ch) = self.iter.borrow_mut().next() else {
                break;
            };
            self.source.borrow_mut().push(ch);
        }
        let ch = self
            .source
            .borrow_mut()
            .get(self.offset..)
            .and_then(|s| s.chars().next());
        ch.map(|ch| {
            let offset = self.offset;
            self.offset += ch.len_utf8();
            (self, ch, offset)
        })
    }
}
