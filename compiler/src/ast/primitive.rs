use crate::span::{Span, SpanOf};

use super::Parser;

fn char_parser() -> Parser<SpanOf<char>> {
    Parser::new(|scanner| {
        Ok(scanner
            .clone()
            .next()
            .map(|(next, ch, i)| (next, SpanOf(Span(i, i + ch.len_utf8()), ch))))
    })
}

fn digit_parser(radix: u32) -> Parser<SpanOf<u8>> {
    char_parser().and_then(move |SpanOf(span, ch)| match ch.to_digit(radix) {
        Some(d) => Parser::new_ok(SpanOf(span, d as u8)),
        None => Parser::new_none(),
    })
}
fn integer_parser(radix: u32) -> Parser<SpanOf<Vec<u8>>> {
    Parser::fold::<Option<SpanOf<Vec<u8>>>>(
        move || digit_parser(radix),
        |acc, SpanOf(dspan, digit)| match acc {
            Some(SpanOf(span, mut vec)) => {
                vec.push(digit);
                Some(SpanOf(span.concat(dspan), vec))
            }
            None => Some(SpanOf(dspan, vec![digit])),
        },
        None,
    )
    .and_then(move |integer| match integer {
        Some(i) => Parser::new_ok(i),
        None => Parser::new_none(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::scanner::Scanner;

    #[test]
    fn test_integer_parser() {
        assert_eq!(
            integer_parser(10)
                .parse(Scanner::new("351"))
                .unwrap()
                .unwrap()
                .1
                 .1,
            [3, 5, 1]
        );
        assert_eq!(
            integer_parser(16)
                .parse(Scanner::new("Ff3"))
                .unwrap()
                .unwrap()
                .1
                 .1,
            [0xf, 0xf, 3]
        );
    }
}
