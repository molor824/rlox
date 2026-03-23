use num_bigint::BigInt;

use crate::ast::*;

#[derive(Debug, Eq, PartialEq)]
pub struct Integer {
    pub radix: u32,
    pub value: BigInt,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Number {
    pub radix: u32,
    pub integer: BigInt,
    pub exponent: Option<i64>,
}
impl Number {
    pub fn new(radix: u32, mut integer: BigInt, exponent: Option<i64>) -> Self {
        if let Some(mut exp) = exponent {
            // Perform zero trimming exponent optimization
            if integer == BigInt::ZERO {
                exp = 0
            } else {
                while &integer % radix == BigInt::ZERO {
                    integer /= radix;
                    exp += 1;
                }
            }
            Self {
                radix,
                integer,
                exponent: Some(exp),
            }
        } else {
            Self {
                radix,
                integer,
                exponent,
            }
        }
    }
}

impl<R: Read> Parser<R> {
    // Every following digit after the first can have one underscore
    // Every alphanumeric characters consequent after one and other, is considered part of number
    // If that said alphanumeric character is non-digit, then return an error
    fn next_digit(&mut self, radix: u32) -> Result<Option<SpanOf<u32>>> {
        let Some(SpanOf(span, ch)) = self.next_if(|ch| {
            if radix <= 10 {
                ch.1.is_ascii_digit()
            } else {
                ch.1.is_alphanumeric()
            }
        })?
        else {
            return Ok(None);
        };
        let Some(digit) = ch.to_digit(radix) else {
            return Err(self.error(span, ErrorKind::NotDigit(ch, radix)));
        };
        Ok(Some(SpanOf(span, digit)))
    }
    /// Parses partial integer. More specifically, it parses integer without the prefix part, sending the radix as a parameter
    fn next_partial_integer(&mut self, radix: u32) -> Result<Option<SpanOf<BigInt>>> {
        let Some(first_digit) = self.next_digit(radix)? else {
            return Ok(None);
        };
        let mut number = first_digit.map(BigInt::from);
        loop {
            let prev = self.clone();
            self.next_if(|ch| ch.1 == '_')?;
            let Some(digit) = self.next_digit(radix)? else {
                *self = prev;
                return Ok(Some(number))
            };
            number = number.concat(digit, |num, d| num * radix + d)
        }
    }
    fn next_integer(&mut self) -> Result<Option<SpanOf<Integer>>> {
        let prev = self.clone();
        if let Some(SpanOf(start, _)) = self.next_if(|ch| ch.1 == '0')? {
            if let Some(SpanOf(end, radix)) = self.next_and(|SpanOf(i, ch)| match ch {
                'x' | 'X' => Some(SpanOf(i, 16_u32)),
                'o' | 'O' => Some(SpanOf(i, 8)),
                'b' | 'B' => Some(SpanOf(i, 2)),
                _ => None,
            })? {
                let prefix = start.concat(end);
                return match self.next_partial_integer(radix)? {
                    Some(integer) => Ok(Some(
                        integer
                            .map(|value| Integer { radix, value })
                            .concat_span(prefix),
                    )),
                    None => Err(self.error(prefix, ErrorKind::MissingInteger)),
                };
            }
        }
        *self = prev;
        self.next_partial_integer(10)
            .map(|i| i.map(|i| i.map(|value| Integer { radix: 10, value })))
    }
    fn next_real(&mut self) -> Result<Option<SpanOf<Number>>> {
        let Some(mut integer) = self.next_integer()? else {
            return Ok(None);
        };
        let Some(dot) = self.next_if(|ch| ch.1 == '.')? else {
            return Ok(Some(integer.map(|i| Number::new(i.radix, i.value, None))));
        };
        let Some(mantissa) = self.next_partial_integer(integer.1.radix)? else {
            return Ok(Some(
                integer.concat(dot, |i, _| Number::new(i.radix, i.value, Some(0))),
            ));
        };
        let mantissa_slice = &self.buffer.borrow()[mantissa.start()..mantissa.end()];
        let mut exponent: i64 = 0;
        for ch in mantissa_slice.chars() {
            if ch == '_' {
                continue;
            }
            integer.1.value *= integer.1.radix;
            exponent -= 1;
        }
        Ok(Some(integer.concat(mantissa, |i, m| {
            Number::new(i.radix, i.value + m, Some(exponent))
        })))
    }
    fn next_exponent_integer(&mut self, radix: u32) -> Result<SpanOf<i64>> {
        let sign = self.next_if(|ch| matches!(ch.1, '+' | '-'))?;
        let Some(first_digit) = self.next_digit(radix)? else {
            return Err(match sign {
                Some(s) => self.error(s.0, ErrorKind::MissingExponent),
                None => self.error_here(ErrorKind::MissingExponent),
            });
        };
        let mut integer = first_digit.map(|d| d as i64);
        loop {
            let prev = self.clone();
            self.next_if(|ch| ch.1 == '_')?;
            let Some(digit) = self.next_digit(radix)? else {
                *self = prev;
                break;
            };
            integer = integer.concat(digit, |i, d| i * (radix as i64) + (d as i64));
        }
        if let Some(SpanOf(_, '-')) = sign {
            integer.1 = -integer.1;
        }
        Ok(integer)
    }
    /// Parses full number
    pub fn next_number(&mut self) -> Result<Option<SpanOf<Number>>> {
        let Some(real) = self.next_real()? else {
            return Ok(None);
        };
        let Some(_) = self.next_if(|ch| if real.1.radix > 10 {
            matches!(ch.1, 'p' | 'P')
        } else {
            matches!(ch.1, 'e' | 'E')
        })? else {
            return Ok(Some(real));
        };
        let exponent = self.next_exponent_integer(real.1.radix).map_err(|mut e| {
            e.span = e.span.concat(real.0.clone());
            e
        })?;
        Ok(Some(real.concat(exponent, |mut r, e| {
            r.exponent = Some(r.exponent.unwrap_or(0) + e);
            r
        })))
    }
}

#[cfg(test)]
mod tests {
    use num_bigint::BigInt;

    use crate::ast::{
        primary::{Integer, Number},
        Parser,
    };

    #[test]
    fn integer_parsing() {
        // TODO: Implement ability to read suffixes soon
        let questions = [
            "3_34_21'km",
            "0xDEAD_beef'ft",
            "0b1001_0110'cm",
            "0o0_1_2_3_4_5_6_7'mm",
        ];
        let answers = [
            (10_u32, 3_34_21_i64),
            (16, 0xDEAD_beef),
            (2, 0b1001_0110),
            (8, 0o0_1_2_3_4_5_6_7),
        ];
        for (question, (radix, answer)) in questions.into_iter().zip(answers) {
            let mut parser = Parser::new(question.as_bytes());
            assert_eq!(
                parser.next_integer().unwrap().unwrap().1,
                Integer {
                    radix,
                    value: BigInt::from(answer)
                }
            );
        }
    }
    #[test]
    fn real_parsing() {
        let qna = [
            ("314", (10_u32, 314_i64, None)),
            ("314.", (10, 314, Some(0_i64))),
            ("314.1", (10, 314100, Some(-3))),
            ("3.1410000", (10, 3141, Some(-3))),
            ("0b0.01000", (2, 0b001, Some(-2))),
            ("0o76.54_32_100000", (8, 0o76543210, Some(-6))),
            ("0xdead.BEEF00000000", (16, 0xDEADBEEF, Some(-4))),
        ];
        for (q, a) in qna {
            let mut parser = Parser::new(q.as_bytes());
            assert_eq!(
                parser.next_real().unwrap().unwrap().1,
                Number::new(a.0, a.1.into(), a.2)
            );
        }
    }
    #[test]
    fn num_parsing() {
        let qna = [
            ("123", (10_u32, 123_i64, None)),
            ("123.", (10, 123, Some(0))),
            ("1.230", (10, 123, Some(-2))),
            ("0xdead_BEEF", (16, 0xdeadbeef, None)),
            ("0x10.1", (16, 0x101, Some(-1))),
            ("0o100.40", (8, 0o1004, Some(-1))),
            ("1e10", (10, 1, Some(10))),
            ("34.3e-3", (10, 343, Some(-4))),
            ("0b0.1e+101001", (2, 0b1, Some(0b101001 - 1))),
            ("0xD.eadPBeeF", (16, 0xdead, Some(0xbeef - 3))),
            ("0x1pdeadbeef", (16, 1, Some(0xdeadbeef_i64))),
        ];
        for (q, a) in qna {
            let mut parser = Parser::new(q.as_bytes());
            assert_eq!(
                parser.next_number().unwrap().unwrap().1,
                Number::new(a.0, a.1.into(), a.2)
            )
        }
    }
}
