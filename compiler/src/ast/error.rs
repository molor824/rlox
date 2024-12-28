use std::rc::Rc;

use thiserror::Error;

use crate::span::Span;

#[derive(Clone, Debug)]
pub struct Error {
    source: Rc<str>,
    code: Span<ErrorCode>,
}
impl Error {
    pub const fn new(source: Rc<str>, code: Span<ErrorCode>) -> Self {
        Self { source, code }
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
    #[error("character does not match")]
    CharNotMatch,
    #[error("character is not a digit")]
    CharNotDigit,
    #[error("exponent overflow")]
    ExponentOverflow,
    #[error("missing exponent")]
    MissingExponent,
}
