use std::rc::Rc;

use thiserror::Error;

use crate::span::Span;

#[derive(Clone, Debug)]
pub struct Error {
    pub source: Rc<str>,
    pub code: Span<ErrorCode>,
}
impl Error {
    pub const fn new(source: Rc<str>, code: Span<ErrorCode>) -> Self {
        Self { source, code }
    }
    pub fn map(self, f: impl FnOnce(Span<ErrorCode>) -> Span<ErrorCode>) -> Self {
        Self {
            source: self.source,
            code: f(self.code),
        }
    }
}
#[derive(Error, Debug, Clone)]
pub enum ErrorCode {
    #[error("reached end of file.")]
    Eof,
    #[error("expected token {0:?}")]
    ExpectedToken(&'static str),
    #[error("expected character {0:?}")]
    ExpectedChar(char),
    #[error("expected base prefix (one of b, o, x)")]
    ExpectedBase,
    #[error("expected integer")]
    ExpectedInt,
    #[error("character does not match")]
    CharNotMatch,
    #[error("character is not a digit")]
    CharNotDigit,
    #[error("exponent overflow")]
    ExponentOverflow,
    #[error("missing exponent")]
    MissingExponent,
    #[error("string literal is not terminated")]
    StringLiteralIncomplete,
    #[error("character literal is not terminated")]
    CharLiteralIncomplete,
    #[error("expected character in character literal")]
    CharLiteralEmpty,
    #[error("missing escape character '\\'")]
    MissingEscape,
    #[error("invalid escape character")]
    InvalidEscape,
    #[error("unicode overflow")]
    UnicodeOverflow,
    #[error("invalid unicode")]
    InvalidUnicode,
}
