use num_bigint::BigUint;

use crate::source::Source;

fn parse_digit(source: &mut Source, radix: u32, underscore: bool) -> Option<u32> {
    if underscore { // Skip over underscore character is permitted
        source.next_if(|ch| ch == '_');
    }
    source.next_and(|ch| ch.to_digit(radix))
}
// Every following digit after the first can have one underscore
fn parse_integer(source: &mut Source, radix: u32) -> Option<BigUint> {
    let Some(first_digit) = parse_digit(source, radix, false) else {
        return None;
    };
    let mut number = BigUint::from(first_digit);
    while let Some(digit) = parse_digit(source, radix, true) {
        number = number * radix + digit;
    }
    Some(number)
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use num_bigint::BigUint;

    use crate::{ast::parse_integer, source::Source};

    #[test]
    fn integer_parsing() {
        let mut source = Source::new(Rc::new(RefCell::new("4_15_4".chars())));
        assert_eq!(parse_integer(&mut source, 10), Some(BigUint::from(4154_u32)));
    }
}