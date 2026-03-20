use std::{cell::RefCell, rc::Rc};

#[derive(Clone)]
/// Struct that handles iteration over chars and storing accumulated chars.
/// 
/// It's always mutably referenced in parser methods to advance.
/// If parser method returns None, it's expected for the Source to be rolled back to the previous state by the parsing function
/// However if it returns Err, it's expected for the Source to be at the location where the error occured.
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
    pub fn next_if(&mut self, condition: impl FnOnce(char) -> bool) -> Option<char> {
        let offset = self.offset;
        match self.next() {
            Some(ch) if condition(ch) => Some(ch),
            _ => {
                self.offset = offset;
                None
            }
        }
    }
    pub fn next_and<U>(&mut self, next: impl FnOnce(char) -> Option<U>) -> Option<U> {
        let offset = self.offset;
        match self.next().and_then(next) {
            Some(v) => Some(v),
            _ => {
                self.offset = offset;
                None
            }
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