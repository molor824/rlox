use std::{fmt, io};

use num_bigint::BigUint;

use crate::{source::Source, span::Span};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum ErrorKind {
    #[error("IOError: {0}")]
    IoError(#[from] io::Error),
    #[error("Character {0} is not allowed in number with base {1}")]
    NotDigit(char, u32),
}
#[derive(thiserror::Error, Debug)]
pub struct Error {
    #[source]
    pub kind: ErrorKind,
    pub span: Span
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

// Every following digit after the first can have one underscore
// Every alphanumeric characters consequent after one and other, is considered part of number
// If that said alphanumeric character is non-digit, then return an error
fn parse_digit(source: &mut Source, radix: u32) -> Result<Option<u32>> {
    let Some((i, ch)) = source.next_if(|(_, ch)| ch.is_alphanumeric())? else {
        return Ok(None);
    };
    let Some(digit) = ch.to_digit(radix) else {
        return Err(source.ast_error(ErrorKind::NotDigit(ch, radix), i..(i + ch.len_utf8())));
    };
    Ok(Some(digit))
}
fn parse_integer(source: &mut Source, radix: u32) -> Result<Option<BigUint>> {
    let Some(first_digit) = parse_digit(source, radix)? else {
        return Ok(None);
    };
    let mut number = BigUint::from(first_digit);
    loop {
        source.next_if(|(_, ch)| ch == '_')?;
        let Some(digit) = parse_digit(source, radix)? else {
            break;
        };
        number = number * radix + digit;
    }
    Ok(Some(number))
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, io::BufReader, rc::Rc};

    use num_bigint::BigUint;

    use crate::{ast::parse_integer, source::Source};

    #[test]
    fn integer_parsing() {
        let mut source = Source::new(Rc::new(RefCell::new(BufReader::new("3_34_21".as_bytes()))));
        assert_eq!(parse_integer(&mut source, 10).unwrap(), Some(BigUint::from(33421_u32)));
    }
}