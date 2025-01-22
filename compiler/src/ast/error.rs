use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum Error {
    #[error("reached end of file.")]
    Eof,
    #[error("expected {0:?}")]
    ExpectedString(String),
    #[error("expected one of {0:?}")]
    ExpectedStrings(Vec<String>),
    #[error("unexpected {0:?}")]
    UnexpectedString(String),
    #[error("expected character {0:?}")]
    ExpectedChar(char),
    #[error("expected one of {0:?} characters")]
    ExpectedChars(Vec<char>),
    #[error("expected primary expression")]
    ExpectedPrimary,
    #[error("expected base prefix (one of b, o, x)")]
    ExpectedBase,
    #[error("expected integer")]
    ExpectedInt,
    #[error("character is not a digit")]
    CharNotDigit,
    #[error("character does not match")]
    CharNotMatch,
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
