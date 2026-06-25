use std::{cell::RefCell, fmt, rc::Rc};

use crate::ast::Span;
use crate::interpreter::LocalId;

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
    #[error("local id `{0}` exceeds arity `{1}`")]
    ArityOverflow(LocalId, usize),
    #[error("index `{0}` exceeds stack capacity `{1}`")]
    StackOverflow(usize, usize),
    #[error("local id out of range")]
    InvalidLocalId,
    #[error("binary operator `{0}` cannot be applied to value of type `{1}` and `{2}`")]
    InvalidBinary(&'static str, &'static str, &'static str),
    #[error("unary operator `{0}` cannot be applied to value of type `{1}`")]
    InvalidUnary(&'static str, &'static str),
    #[error("cannot convert value to type `{0}`")]
    InvalidType(&'static str),
    #[error("cannot index with nil value")]
    NilIndexing,
    #[error("cannot index with nan value")]
    NanIndexing,
}
