use num_bigint::BigUint;

use crate::source::Source;

#[derive(PartialEq, Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("Character {0} is not allowed in number with base {1}")]
    NotDigit(char, u32),
}

// Every following digit after the first can have one underscore
// Every alphanumeric characters consequent after one and other, is considered part of number
// If that said alphanumeric character is non-digit, then return an error
fn parse_digit(source: &mut Source, radix: u32) -> Result<Option<u32>, Error> {
    let Some(ch) = source.next_if(|ch| ch.is_alphanumeric()) else {
        return Ok(None);
    };
    let Some(digit) = ch.to_digit(radix) else {
        return Err(Error::NotDigit(ch, radix));
    };
    Ok(Some(digit))
}
fn parse_integer(source: &mut Source, radix: u32) -> Result<Option<BigUint>, Error> {
    let Some(first_digit) = parse_digit(source, radix)? else {
        return Ok(None);
    };
    let mut number = BigUint::from(first_digit);
    loop {
        source.next_if(|ch| ch == '_');
        let Some(digit) = parse_digit(source, radix)? else {
            break;
        };
        number = number * radix + digit;
    }
    Ok(Some(number))
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use num_bigint::BigUint;

    use crate::{ast::parse_integer, source::Source};

    #[test]
    fn integer_parsing() {
        let mut source = Source::new(Rc::new(RefCell::new("4_15_4".chars())));
        assert_eq!(parse_integer(&mut source, 10), Ok(Some(BigUint::from(4154_u32))));
    }
}