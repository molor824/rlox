use num_bigint::BigInt;

use crate::ast::*;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Integer {
    pub span: Span,
    pub radix: u32,
    pub integer: BigInt,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Number {
    pub span: Span,
    pub radix: u32,
    pub integer: BigInt,
    pub exponent: Option<i64>,
}
impl Number {
    pub fn new(span: Span, radix: u32, mut integer: BigInt, mut exponent: Option<i64>) -> Self {
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
            exponent = Some(exp);
        }
        Self {
            span,
            radix,
            integer,
            exponent,
        }
    }
}
#[derive(Clone)]
pub struct Ident(pub Span, pub Rc<RefCell<String>>);
impl fmt::Debug for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", &self.1.borrow()[self.0.start..self.0.end()])
    }
}
impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.1.borrow()[self.0.start..self.0.end()])
    }
}

impl<R: Read> Parser<R> {
    // Every following digit after the first can have one underscore
    // Every alphanumeric characters consequent after one and other, is considered part of number
    // If alphanumeric character is non-digit, then return an error
    // with the exception of 'p' and 'P' characters, as these are used for exponents
    fn next_digit(&mut self, radix: u32) -> Result<Option<(usize, u32)>> {
        let Some((start, ch)) = self.next_if(|ch| {
            if radix <= 10 {
                ch.1.is_ascii_digit()
            } else {
                ch.1.is_alphanumeric() && !matches!(ch.1, 'p' | 'P')
            }
        })?
        else {
            return Ok(None);
        };
        let Some(digit) = ch.to_digit(radix) else {
            return Err(self.error(
                Span::new(start, ch.len_utf8()),
                ErrorKind::NotDigit(ch, radix),
            ));
        };
        Ok(Some((start, digit)))
    }
    /// Parses partial integer. More specifically, it parses integer without the prefix part, sending the radix as a parameter
    fn next_partial_integer(&mut self, radix: u32) -> Result<Option<(Span, BigInt)>> {
        let Some(first_digit) = self.next_digit(radix)? else {
            return Ok(None);
        };
        let mut number = (Span::new(first_digit.0, 1), BigInt::from(first_digit.1));
        loop {
            let prev = self.clone();
            self.next_if(|ch| ch.1 == '_')?;
            let Some(digit) = self.next_digit(radix)? else {
                *self = prev;
                return Ok(Some(number));
            };
            number.1 = number.1 * radix + digit.1;
            number.0 = number.0.concat(Span::new(digit.0, 1));
        }
    }
    fn next_integer(&mut self) -> Result<Option<Integer>> {
        let prev = self.clone();
        if let Some((start, _)) = self.next_if(|ch| ch.1 == '0')? {
            if let Some(radix) = self.next_and(|ch| match ch.1 {
                'x' | 'X' => Some(16_u32),
                'o' | 'O' => Some(8),
                'b' | 'B' => Some(2),
                _ => None,
            })? {
                let prefix = Span::new(start, 2);
                return match self.next_partial_integer(radix)? {
                    Some((span, integer)) => Ok(Some(Integer {
                        radix,
                        span: prefix.concat(span),
                        integer,
                    })),
                    None => Err(self.error(prefix, ErrorKind::MissingInteger)),
                };
            }
        }
        *self = prev;
        self.next_partial_integer(10).map(|i| {
            i.map(|(span, integer)| Integer {
                span,
                integer,
                radix: 10,
            })
        })
    }
    fn next_real(&mut self) -> Result<Option<Number>> {
        let Some(mut integer) = self.next_integer()? else {
            return Ok(None);
        };
        let Some(dot) = self.next_if(|ch| ch.1 == '.')? else {
            return Ok(Some(Number {
                span: integer.span,
                radix: integer.radix,
                integer: integer.integer,
                exponent: None,
            }));
        };
        let Some(mantissa) = self.next_partial_integer(integer.radix)? else {
            return Ok(Some(Number {
                span: integer.span.concat(Span::new(dot.0, 1)),
                radix: integer.radix,
                integer: integer.integer,
                exponent: Some(0),
            }));
        };
        let mantissa_slice = &self.buffer.borrow()[mantissa.0.start..mantissa.0.end()];
        let mut exponent: i64 = 0;
        for ch in mantissa_slice.chars() {
            if ch == '_' {
                continue;
            }
            integer.integer *= integer.radix;
            exponent -= 1;
        }
        Ok(Some(Number::new(
            integer.span.concat(mantissa.0),
            integer.radix,
            integer.integer + mantissa.1,
            Some(exponent),
        )))
    }
    fn next_exponent_integer(&mut self, radix: u32) -> Result<(Span, i64)> {
        let sign = self.next_if(|ch| matches!(ch.1, '+' | '-'))?;
        let Some(first_digit) = self.next_digit(radix)? else {
            return Err(match sign {
                Some(s) => self.error(Span::new(s.0, s.1.len_utf8()), ErrorKind::MissingExponent),
                None => self.error_here(ErrorKind::MissingExponent),
            });
        };
        let mut integer = first_digit.1 as i64;
        let mut end = first_digit.0 + 1;
        loop {
            let prev = self.clone();
            self.next_if(|ch| ch.1 == '_')?;
            let Some(digit) = self.next_digit(radix)? else {
                *self = prev;
                break;
            };
            integer = integer * (radix as i64) + (digit.1 as i64);
            end = digit.0 + 1;
        }
        if let Some((_, '-')) = sign {
            integer = -integer;
        }
        Ok((Span::from_end(first_digit.0, end), integer))
    }
    /// Parses full number
    pub fn next_number(&mut self) -> Result<Option<Number>> {
        let Some(real) = self.next_real()? else {
            return Ok(None);
        };
        let Some(_) = self.next_if(|ch| {
            if real.radix > 10 {
                matches!(ch.1, 'p' | 'P')
            } else {
                matches!(ch.1, 'e' | 'E')
            }
        })?
        else {
            return Ok(Some(real));
        };
        let exponent = self.next_exponent_integer(real.radix).map_err(|mut e| {
            e.span = e.span.concat(real.span.clone());
            e
        })?;
        Ok(Some(Number {
            span: real.span.concat(exponent.0),
            radix: real.radix,
            integer: real.integer,
            exponent: Some(exponent.1 + real.exponent.unwrap_or(0)),
        }))
    }
    pub fn next_ident(&mut self) -> Result<Option<Ident>> {
        let Some((start, ch)) = self.next_if(|ch| ch.1.is_alphabetic() || ch.1 == '_')? else {
            return Ok(None);
        };
        let mut end = start + ch.len_utf8();
        while let Some((current, ch)) = self.next_if(|(_, ch)| ch.is_alphanumeric() || ch == '_')? {
            end = current + ch.len_utf8();
        }
        while let Some((current, ch)) = self.next_if(|(_, ch)| ch == '\'')? {
            end = current + ch.len_utf8()
        }
        Ok(Some(Ident(Span::from_end(start, end), self.buffer.clone())))
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::Parser;

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
            let result = parser.next_number().unwrap().unwrap();
            assert_eq!((result.radix, result.integer, result.exponent), (a.0, a.1.into(), a.2));
        }
    }
    #[test]
    fn ident_parsing() {
        let questions = [
            "___", "_test", "test123", "x", "x'", "x''"
        ];
        for q in questions {
            let mut parser = Parser::new(q.as_bytes());
            let result = parser.next_ident().unwrap().unwrap();
            assert_eq!(result.to_string(), q);
        }
    }
}
