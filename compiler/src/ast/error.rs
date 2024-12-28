use std::rc::Rc;

use thiserror::Error;

#[derive(Clone, Debug)]
pub struct Error {
    source: Rc<str>,
    index: usize,
    code: ErrorCode,
}
impl Error {
    pub const fn new(source: Rc<str>, index: usize, code: ErrorCode) -> Self {
        Self {
            source,
            index,
            code,
        }
    }
}
#[derive(Error, Debug, Clone)]
pub enum ErrorCode {
    #[error("reached end of file.")]
    Eof,
    #[error("string does not match {0:?}")]
    StringNotEq(&'static str),
    #[error("character does not match {0:?}")]
    CharNotEq(char),
    #[error("{0:?} is not a digit")]
    CharNotDigit(char),
}
