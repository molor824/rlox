use std::{cell::RefCell, rc::Rc};

#[derive(Clone)]
pub struct Source {
    pub iter: Rc<RefCell<dyn Iterator<Item = char>>>,
    pub accumulated: Rc<RefCell<String>>,
    pub offset: usize,
}
impl Source {
    pub fn new(iter: Rc<RefCell<dyn Iterator<Item = char>>>) -> Self {
        Self {
            iter,
            accumulated: Rc::new(RefCell::new(String::new())),
            offset: 0,
        }
    }
}
impl Iterator for Source {
    type Item = char;
    fn next(&mut self) -> Option<Self::Item> {
        let Some(ch) = self.accumulated.borrow().get(self.offset..).and_then(|str| str.chars().next()) else {
            let Some(ch) = self.iter.borrow_mut().next() else {
                return None;
            };
            let mut accumulated = self.accumulated.borrow_mut();
            accumulated.push(ch);
            self.offset = accumulated.len();
            return Some(ch);
        };
        self.offset += ch.len_utf8();
        Some(ch)
    }
}