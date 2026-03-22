use num_bigint::BigInt;

use crate::ast::*;

// Every following digit after the first can have one underscore
// Every alphanumeric characters consequent after one and other, is considered part of number
// If that said alphanumeric character is non-digit, then return an error
fn parse_digit(source: &mut Source, radix: u32) -> Result<Option<(usize, u32)>> {
    let Some((i, ch)) = source.next_if(|(_, ch)| ch.is_alphanumeric())? else {
        return Ok(None);
    };
    let Some(digit) = ch.to_digit(radix) else {
        return Err(source.error(ErrorKind::NotDigit(ch, radix), i..(i + ch.len_utf8())));
    };
    Ok(Some((i, digit)))
}
fn parse_integer(source: &mut Source, radix: u32) -> Result<Option<SpanOf<BigInt>>> {
    let Some((start, first_digit)) = parse_digit(source, radix)? else {
        return Ok(None);
    };
    let mut end = start + 1;
    let mut number = BigInt::from(first_digit);
    loop {
        source.next_if(|(_, ch)| ch == '_')?;
        let Some((current, digit)) = parse_digit(source, radix)? else {
            break;
        };
        number = number * radix + digit;
        end = current;
    }
    Ok(Some(source.span_of(start..end, number)))
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, io::BufReader, rc::Rc};

    use num_bigint::BigInt;

    use crate::ast::{Source, primary::parse_integer};

    #[test]
    fn integer_parsing() {
        let mut source = Source::new(Rc::new(RefCell::new(BufReader::new("3_34_21".as_bytes()))));
        assert_eq!(
            parse_integer(&mut source, 10).unwrap().unwrap().1,
            BigInt::from(3_34_21)
        );
    }
}
