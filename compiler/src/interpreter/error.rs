use std::{cell::RefCell, fmt, rc::Rc};

use crate::{
    ast::Span,
    interpreter::{string::ValueStr, value::Value},
};

#[derive(Debug, thiserror::Error)]
pub struct Error {
    buffer: Rc<RefCell<String>>,
    span: Option<Span>,
    source: ErrorKind,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ErrorKind {
    #[error("stack overflow")]
    StackOverflow,
    #[error("stack underflow")]
    StackUnderflow,
    #[error("local id out of range")]
    InvalidLocalId,
    #[error("binary operator `{0}` cannot be applied to value of type `{1}` and `{2}`")]
    InvalidBinary(&'static str, &'static str, &'static str),
    #[error("unary operator `{0}` cannot be applied to value of type `{1}`")]
    InvalidUnary(&'static str, &'static str),
    #[error("cannot convert `{0}` to type `{1}`")]
    InvalidType(Value, &'static str),
    #[error("cannot index with nil value")]
    NilIndexing,
    #[error("cannot index with nan value")]
    NanIndexing,
    #[error("attempted to write to read-only global `{0}`")]
    ReadonlyGlobalWrite(ValueStr),
    #[error("attempted to share a memory that is not initialized")]
    UninitCellShare,
    #[error("attempted to index array with non-number value")]
    InvalidArrayIndex,
    #[error("attempted to access property of value whose type is not object or array")]
    InvalidPropertyAccess,
    #[error("attempted to access non-existent upvalue")]
    InvalidUpvalueAccess,
}
