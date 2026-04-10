use std::cell::Ref;

use num_bigint::BigInt;

use crate::ast::*;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Integer {
    pub radix: u32,
    pub integer: BigInt,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Number {
    pub radix: u32,
    pub integer: BigInt,
    pub exponent: Option<i64>,
}
impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.radix {
            2 => write!(f, "0b{:b}", self.integer),
            8 => write!(f, "0o{:o}", self.integer),
            10 => write!(f, "{}", self.integer),
            16 => write!(f, "0x{:X}", self.integer),
            _ => unreachable!(),
        }?;
        if let Some(exp) = self.exponent {
            let sign = if exp >= 0 { '+' } else { '-' };
            let exp = exp.abs();
            match self.radix {
                2 => write!(f, "e{sign}{:b}", exp),
                8 => write!(f, "e{sign}{:o}", exp),
                10 => write!(f, "e{sign}{}", exp),
                16 => write!(f, "p{sign}{:X}", exp),
                _ => unreachable!(),
            }?;
        }
        Ok(())
    }
}
impl Number {
    pub fn new(radix: u32, mut integer: BigInt, mut exponent: Option<i64>) -> Self {
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
            radix,
            integer,
            exponent,
        }
    }
}
#[derive(Clone)]
pub struct CachedString {
    id: usize,
    cache: Rc<RefCell<Cache<str>>>,
}
impl CachedString {
    pub fn get_str<'a>(&'a self) -> Ref<'a, str> {
        Ref::map(self.cache.borrow(), |cache| {
            cache.get_data(self.id).unwrap()
        })
    }
}
impl fmt::Debug for CachedString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CachedString")
            .field(&self.get_str())
            .finish()
    }
}
impl fmt::Display for CachedString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &*self.get_str())
    }
}

impl<R: Read> Parser<R> {
    // Every following digit after the first can have one underscore
    // Every alphanumeric characters consequent after one and other, is considered part of number
    // If alphanumeric character is non-digit, then return an error
    // with the exception of 'p' and 'P' characters, as these are used for exponents
    fn next_digit(&mut self, radix: u32) -> Result<Option<SpanOf<u32>>> {
        let Some(digit_ch) = self.next_if(|ch| {
            if radix <= 10 {
                ch.1.is_ascii_digit()
            } else {
                ch.1.is_alphanumeric() && !matches!(ch.1, 'p' | 'P')
            }
        })?
        else {
            return Ok(None);
        };
        let Some(digit) = digit_ch.1.to_digit(radix) else {
            return Err(self.error(digit_ch.0, ErrorKind::NotDigit(digit_ch.1, radix)));
        };
        Ok(Some(SpanOf(digit_ch.0, digit)))
    }
    fn next_sequence(&mut self, sequence: &str) -> Result<Option<Span>> {
        let prev = self.clone();
        let mut span: Option<Span> = None;
        for ch in sequence.chars() {
            let Some(ch1) = self.next()? else {
                break;
            };
            if ch1.1 != ch {
                break;
            }
            span = Some(match span {
                Some(s) => s.concat(ch1.0),
                None => ch1.0,
            });
        }
        if span.is_none() {
            *self = prev;
        }
        Ok(span)
    }
    pub fn next_symbol(&mut self, symbol: &str, skip_newline: bool) -> Result<Option<Span>> {
        self.skip(skip_newline)?;
        self.next_sequence(symbol)
    }
    fn skip_whitespace(&mut self, skip_newline: bool) -> Result<bool> {
        let mut skipped = false;
        while self
            .next_if(|ch| {
                if skip_newline {
                    ch.1.is_whitespace()
                } else {
                    ch.1.is_whitespace() && ch.1 != '\n' && ch.1 != '\r'
                }
            })?
            .is_some()
        {
            skipped = true;
        }
        Ok(skipped)
    }
    fn skip_comments(&mut self) -> Result<bool> {
        let mut skipped = false;
        if self.next_if(|ch| ch.1 == '#')?.is_some() {
            skipped = true;
            if self.next_if(|ch| ch.1 == '{')?.is_some() {
                loop {
                    if self.next_sequence("}#")?.is_some() || self.next()?.is_none() {
                        break;
                    }
                }
            } else {
                while self.next_if(|ch| ch.1 != '\n')?.is_some() {}
            }
        }
        Ok(skipped)
    }
    fn skip(&mut self, skip_newline: bool) -> Result<bool> {
        let mut skipped = false;
        loop {
            if self.skip_whitespace(skip_newline)? || self.skip_comments()? {
                skipped = true;
                continue;
            }
            return Ok(skipped);
        }
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
                return Ok(Some(number));
            };
            number = number.concat(digit, |n, d| n * radix + d);
        }
    }
    fn next_integer(&mut self) -> Result<Option<SpanOf<Integer>>> {
        let prev = self.clone();
        if let Some(start) = self.next_if(|ch| ch.1 == '0')? {
            if let Some(radix) = self.next_and(|ch| match ch.1 {
                'x' | 'X' => Some(16_u32),
                'o' | 'O' => Some(8),
                'b' | 'B' => Some(2),
                _ => None,
            })? {
                let prefix = SpanOf(start.0, radix);
                return match self.next_partial_integer(radix)? {
                    Some(integer) => {
                        Ok(Some(integer.concat(prefix, |integer, radix| Integer {
                            radix,
                            integer,
                        })))
                    }
                    None => Err(self.error(start.0, ErrorKind::MissingInteger)),
                };
            }
        }
        *self = prev;
        self.next_partial_integer(10)
            .map(|i| i.map(|i| i.map(|integer| Integer { integer, radix: 10 })))
    }
    fn next_real(&mut self) -> Result<Option<SpanOf<Number>>> {
        let Some(mut integer) = self.next_integer()? else {
            return Ok(None);
        };
        let Some(dot) = self.next_if(|ch| ch.1 == '.')? else {
            return Ok(Some(
                integer.map(|integer| Number::new(integer.radix, integer.integer, None)),
            ));
        };
        let Some(mantissa) = self.next_partial_integer(integer.1.radix)? else {
            return Ok(Some(
                integer.concat(dot, |i, _| Number::new(i.radix, i.integer, Some(0))),
            ));
        };
        let mantissa_slice = &self.buffer.borrow()[mantissa.0.start..mantissa.0.end];
        let mut exponent: i64 = 0;
        for ch in mantissa_slice.chars() {
            if ch == '_' {
                continue;
            }
            integer.1.integer *= integer.1.radix;
            exponent -= 1;
        }
        Ok(Some(integer.concat(mantissa, |i, m| {
            Number::new(i.radix, i.integer + m, Some(exponent))
        })))
    }
    fn next_exponent_integer(&mut self, radix: u32) -> Result<SpanOf<i64>> {
        let sign = self.next_if(|ch| matches!(ch.1, '+' | '-'))?;
        let Some(first_digit) = self.next_digit(radix)? else {
            return Err(match sign {
                Some(s) => self.error(s.0, ErrorKind::MissingExponent),
                None => self.error(Span::from_len(self.offset, 0), ErrorKind::MissingExponent),
            });
        };
        let mut integer = first_digit.map(|i| i as i64);
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
        let Some(_) = self.next_if(|ch| {
            if real.1.radix > 10 {
                matches!(ch.1, 'p' | 'P')
            } else {
                matches!(ch.1, 'e' | 'E')
            }
        })?
        else {
            return Ok(Some(real));
        };
        let exponent = self.next_exponent_integer(real.1.radix).map_err(|mut e| {
            e.span = e.span.concat(real.0);
            e
        })?;
        Ok(Some(real.concat(exponent, |r, e| {
            Number::new(r.radix, r.integer, Some(e + r.exponent.unwrap_or(0)))
        })))
    }
    pub fn next_ident(&mut self) -> Result<Option<SpanOf<CachedString>>> {
        let Some(first) = self.next_if(|ch| ch.1.is_alphabetic() || ch.1 == '_')? else {
            return Ok(None);
        };
        let mut span = first.0;
        while let Some(ch) = self.next_if(|ch| ch.1.is_alphanumeric() || ch.1 == '_')? {
            span = span.concat(ch.0);
        }
        while let Some(ch) = self.next_if(|ch| ch.1 == '\'')? {
            span = span.concat(ch.0);
        }
        Ok(Some(SpanOf(
            span,
            CachedString {
                cache: self.ident_cache.clone(),
                id: self.get_ident_id(&self.buffer.borrow()[span.start..span.end]),
            },
        )))
    }
    fn next_char(&mut self, raw: bool) -> Result<Option<SpanOf<char>>> {
        let next_hex_digits = |parser: &mut Self, count: usize| -> Result<Option<SpanOf<u32>>> {
            let mut number = None;
            for _ in 0..count {
                let Some(digit) = parser.next_and(|ch| match ch.1.to_digit(16) {
                    Some(hex) => Some(SpanOf(ch.0, hex)),
                    None => None,
                })?
                else {
                    return Ok(None);
                };
                let num = number.unwrap_or(SpanOf(digit.0, 0_u32));
                number = Some(num.concat(digit, |n, d| n * 16 + d))
            }
            Ok(number)
        };
        let Some(ch) = self.next()? else {
            return Ok(None);
        };
        match ch.1 {
            '\\' if !raw => {
                let escape = self
                    .next()?
                    .ok_or(self.error(ch.0, ErrorKind::InvalidEscape))?;
                let span = ch.0.concat(escape.0);
                Ok(Some(match escape.1 {
                    'a' => SpanOf(span, '\x07'),
                    'b' => SpanOf(span, '\x08'),
                    'n' => SpanOf(span, '\n'),
                    't' => SpanOf(span, '\t'),
                    'r' => SpanOf(span, '\r'),
                    'f' => SpanOf(span, '\x0c'),
                    '\'' => SpanOf(span, '\''),
                    '"' => SpanOf(span, '"'),
                    '\\' => SpanOf(span, '\\'),
                    '0' => SpanOf(span, '\0'),
                    'x' | 'u' | 'U' => {
                        let Some(hex_digits) = next_hex_digits(
                            self,
                            match escape.1 {
                                'x' => 2,
                                'u' => 4,
                                _ => 8,
                            },
                        )?
                        else {
                            return Err(self.error(span, ErrorKind::InvalidEscape));
                        };
                        let Some(ch) = char::from_u32(hex_digits.1) else {
                            return Err(self.error(hex_digits.0, ErrorKind::InvalidUnicode));
                        };
                        SpanOf(span.concat(hex_digits.0), ch)
                    }
                    _ => return Err(self.error(span, ErrorKind::InvalidEscape)),
                }))
            }
            _ => Ok(Some(ch)),
        }
    }
    pub fn next_literal_string(&mut self) -> Result<Option<SpanOf<CachedString>>> {
        let prev = self.clone();
        let raw_start = self.next_if(|ch| ch.1 == 'r')?;
        let depth = match raw_start {
            Some(_) => {
                let mut depth: Option<SpanOf<usize>> = None;
                while let Some(ch) = self.next_if(|ch| ch.1 == '(')? {
                    depth = Some(match depth {
                        Some(d) => d.concat(ch, |d, _| d + 1),
                        None => SpanOf(ch.0, 1),
                    });
                }
                depth
            }
            None => None,
        };
        let Some(quote_start) = self.next_if(|ch| matches!(ch.1, '\'' | '"'))? else {
            *self = prev;
            return Ok(None);
        };
        let mut span = quote_start.0;
        let mut string = String::new();
        if let Some(ch) = raw_start {
            span = span.concat(ch.0);
        }

        loop {
            let prev = self.clone();
            if let Some(end_quote) = self.next_if(|ch| ch.1 == quote_start.1)? {
                let mut span1 = end_quote.0;
                let mut satisfies = true;
                match depth {
                    Some(depth) => {
                        for _ in 0..depth.1 {
                            let Some(bracket) = self.next_if(|ch| ch.1 == ')')? else {
                                satisfies = false;
                                break;
                            };
                            span1 = span1.concat(bracket.0);
                        }
                    }
                    None => {}
                }
                if satisfies {
                    span = span.concat(span1);
                    break;
                }
            }
            *self = prev;
            let Some(ch) = self.next_char(raw_start.is_some())? else {
                return Err(self.error(span, ErrorKind::UnterminatedString));
            };
            span = span.concat(ch.0);
            string.push(ch.1);
        }

        Ok(Some(SpanOf(
            span,
            CachedString {
                cache: self.string_cache.clone(),
                id: self.get_string_id(&string),
            },
        )))
    }
    pub fn next_primitive(&mut self, skip_newline: bool) -> Result<Option<Expression>> {
        self.skip(skip_newline)?;
        Ok(Some(if let Some(n) = self.next_number()? {
            Expression::Number(n)
        } else if let Some(s) = self.next_literal_string()? {
            Expression::String(s)
        } else if let Some(i) = self.next_ident()? {
            Expression::Ident(i)
        } else {
            return Ok(None);
        }))
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
            let result = parser.next_number().unwrap().unwrap().1;
            assert_eq!(
                (result.radix, result.integer, result.exponent),
                (a.0, a.1.into(), a.2)
            );
        }
    }
    #[test]
    fn ident_parsing() {
        let questions = ["___", "_test", "test123", "x", "x'", "x''"];
        for q in questions {
            let mut parser = Parser::new(q.as_bytes());
            let result = parser.next_ident().unwrap().unwrap().1;
            assert_eq!(&*result.get_str(), q);
        }
    }
    #[test]
    fn string_parsing() {
        let qna = [
            (r#""test""#, "test"),
            (r#"'escape\n'"#, "escape\n"),
            (r#"'escape\''"#, "escape'"),
            ("\'new\nline\'", "new\nline"),
            (r#""w is \x77""#, "w is w"),
            (
                r#""Superman once said \"Thou shalt not pass\"""#,
                "Superman once said \"Thou shalt not pass\"",
            ),
            (r#""\u4f60\U0000597d""#, "你好"),
            (r#"'你好'"#, "\u{4f60}\u{597d}"),
            (r#"r"raw string\""#, r"raw string\"),
            (
                r#"r('raw string with 'quotes'')"#,
                r"raw string with 'quotes'",
            ),
            (
                r#"r(("raw string with ("quotes and brackets")"))"#,
                r#"raw string with ("quotes and brackets")"#,
            ),
        ];
        for (q, a) in qna {
            let mut parser = Parser::new(q.as_bytes());
            let result = parser.next_literal_string().unwrap().unwrap().1;
            assert_eq!(&*result.get_str(), a);
        }
    }
    #[test]
    fn primitive_parsing() {
        let mut parser = Parser::new(
            r#"ident 1 10 0xdEaD00 0o123123
# Must ignore comment!
0b1011_1101
            "string" "escape\nstring" "string
with newline"
            r"raw string
with newline"
r"should ignore this escape!\n"
r("let me "quote" this!")
r(("let me ("bracket and quote") this!"))
#{ Must be ignored!
"comment, instead of string"
        }#
        x1'
        _x2_''
        __x3__'''
        10.
        0.1
        0x0.F
        0b1.10
        123e-10
        12.3e10
        1.23E+10
        0xde.adP+beef
        0b1010e+1010
        0o77.E-71"\u4f60\U0000597d"
#{ Unfinished comment, cuz why not
"#
            .as_bytes(),
        );
        let answers = [
            "ident",
            "1",
            "10",
            "0xDEAD00",
            "0o123123",
            "0b10111101",
            r#""string""#,
            r#""escape\nstring""#,
            r#""string\nwith newline""#,
            r#""raw string\nwith newline""#,
            r#""should ignore this escape!\\n""#,
            r#""let me \"quote\" this!""#,
            r#""let me (\"bracket and quote\") this!""#,
            "x1'",
            "_x2_''",
            "__x3__'''",
            "1e+1",
            "1e-1",
            "0xFp-1",
            "0b11e-1",
            "123e-10",
            "123e+9",
            "123e+8",
            "0xDEADp+BEED",
            "0b101e+1011",
            "0o77e-71",
            "\"\u{4f60}\u{597d}\""
        ];
        for answer in answers {
            let result = parser.next_expression(true).unwrap().unwrap().to_string();
            assert_eq!(answer, result);
        }
    }
}
