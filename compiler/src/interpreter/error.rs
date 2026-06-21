use std::{cell::RefCell, fmt, rc::Rc};

#[derive(Debug, thiserror::Error)]
pub struct Error {
    buffer: Rc<RefCell<String>>,
    source: ErrorKind,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ErrorKind {
    #[error("Local id out of range")]
    InvalidLocalId,
}
