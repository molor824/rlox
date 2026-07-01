use std::{cell::RefCell, fmt, io, rc::Rc};

use crate::{interpreter::string::ValueStr, span::Span};

#[derive(Debug, thiserror::Error)]
pub enum ErrorKind {
    #[error("IOError: {0}")]
    IoError(#[from] io::Error),
    #[error("Character {0} is not allowed in number with base {1}")]
    NotDigit(char, u32),
    #[error("Missing integer")]
    MissingInteger,
    #[error("Missing exponent")]
    MissingExponent,
    #[error("Invalid escape sequence")]
    InvalidEscape,
    #[error("Invalid unicode")]
    InvalidUnicode,
    #[error("String literal unterminated")]
    UnterminatedString,
    #[error("Expected `(`")]
    ExpectedLeftParen,
    #[error("Expected `)`")]
    ExpectedRightParen,
    #[error("Expected `]`")]
    ExpectedRightSquare,
    #[error("Expected `}}`")]
    ExpectedRightCurly,
    #[error("Expected `:`")]
    ExpectedColon,
    #[error("Expected `=`")]
    ExpectedEq,
    #[error("Cannot use unpacking operation here")]
    UnexpectedUnpacking,
    #[error("Expected identifier")]
    ExpectedIdent,
    #[error("Expected expression")]
    ExpectedExpr,
    #[error("Array splitting already used")]
    RepeatingSplit,
    #[error("Expected `else` or `end` terminator at the end of block")]
    ExpectedElse,
    #[error("Expected `end` terminator at the end of block")]
    ExpectedEnd,
    #[error("Expected `then`")]
    ExpectedThen,
    #[error("Expected `in`")]
    ExpectedIn,
    #[error("Expected `do ... end` block")]
    ExpectedDoBlock,
    #[error("Invalid expression behind `=` operator. Only variable, property and/or indexing is allowed.")]
    InvalidAssignee,
    #[error("Expeced function body `=> [expr]` or `do ... end`")]
    ExpectedFuncBody,
    #[error("Stack overflow")]
    StackOverflow,
    #[error("Stack underflow")]
    StackUnderflow,
    #[error("Local id out of range")]
    InvalidLocalId,
    #[error("Binary operator `{0}` cannot be applied to value of type `{1}` and `{2}`")]
    InvalidBinary(&'static str, &'static str, &'static str),
    #[error("Unary operator `{0}` cannot be applied to value of type `{1}`")]
    InvalidUnary(&'static str, &'static str),
    #[error("Cannot convert `{0}` to `{1}`")]
    InvalidType(&'static str, &'static str),
    #[error("Cannot index with nil value")]
    NilIndexing,
    #[error("Cannot index with nan value")]
    NanIndexing,
    #[error("Attempted to write to read-only global `{0}`")]
    ReadonlyGlobalWrite(ValueStr),
    #[error("Attempted to share a memory that is not initialized")]
    UninitCellShare,
    #[error("Attempted to index array with non-number value")]
    InvalidArrayIndex,
    #[error("Attempted to access property of value whose type is not object or array")]
    InvalidPropertyAccess,
    #[error("Attempted to access non-existent upvalue")]
    InvalidUpvalueAccess,
}

#[derive(thiserror::Error)]
pub struct Error {
    #[source]
    pub kind: ErrorKind,
    pub span: Span,
    pub source: Rc<RefCell<String>>,
}
impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Error")
            .field("kind", &self.kind)
            .field(
                "span",
                &(
                    self.span.start..self.span.end,
                    &self.source.borrow()[self.span.start..self.span.end],
                ),
            )
            .finish()
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut col = 0;
        let mut row = 1;
        let mut prev_ch = '\0';
        let source = self.source.borrow();
        for (i, ch) in source.char_indices() {
            if prev_ch == '\n' || prev_ch == '\r' {
                col = 1;
                if prev_ch == '\n' {
                    row += 1;
                }
            } else {
                col += 1;
            }
            if i == self.span.start {
                return write!(f, "Error [line:{}, col:{}]: {}", row, col, self.kind);
            }
            prev_ch = ch;
        }
        write!(f, "Error: {}", self.kind)
    }
}
pub type Result<T> = std::result::Result<T, Error>;
