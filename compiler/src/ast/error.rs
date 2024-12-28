use std::rc::Rc;

use crate::span::SpanOf;

use thiserror::Error;

#[derive(Clone, Debug)]
pub struct Error {
    source: Rc<str>,
    code: SpanOf<ErrorCode>,
}
impl Error {
    pub const fn new(source: Rc<str>, code: SpanOf<ErrorCode>) -> Self {
        Self { source, code }
    }
}
#[derive(Error, Debug, Clone)]
pub enum ErrorCode {
    #[error("reached end of file.")]
    Eof,
    #[error("unexpected character {0:?}")]
    UnexpectedChar(char),
}
