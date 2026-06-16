use crate::ast::{expression::*, *};

impl<R: BufRead> Parser<R> {
    fn next_element(&mut self, skip_newline: bool) -> Result<Option<Element>> {
        let Some(star) = self.next_symbol("*", skip_newline)? else {
            return Ok(self.next_expression(skip_newline)?.map(Element::Regular));
        };
        Ok(self
            .next_expression(skip_newline)?
            .map(|expr| Element::Unpack(SpanOf(star.concat(expr.span()), expr))))
    }
    pub fn next_elements(&mut self, skip_newline: bool) -> Result<Vec<Element>> {
        let mut elements = vec![];
        while let Some(element) = self.next_element(skip_newline)? {
            elements.push(element);
            if self.next_symbol(",", skip_newline)?.is_none() {
                break;
            }
        }
        Ok(elements)
    }
    fn next_group(&mut self, skip_newline: bool) -> Result<Option<Expression>> {
        let Some(start) = self.next_symbol("(", skip_newline)? else {
            return Ok(None);
        };
        let Some(expr) = self.next_expression(true)? else {
            return Err(self.error(start, ErrorKind::ExpectedExpr));
        };
        let Some(_) = self.next_symbol(")", true)? else {
            return Err(self.error(expr.span(), ErrorKind::ExpectedRightParen));
        };
        Ok(Some(expr))
    }
    fn next_array(&mut self, skip_newline: bool) -> Result<Option<Expression>> {
        let Some(start) = self.next_symbol("[", skip_newline)? else {
            return Ok(None);
        };
        let elements = self.next_elements(true)?;
        let Some(end) = self.next_symbol("]", true)? else {
            return Err(self.error(start, ErrorKind::ExpectedRightSquare));
        };
        Ok(Some(Expression::Array(SpanOf(start.concat(end), elements))))
    }
    fn next_pair(&mut self) -> Result<Option<Pair>> {
        let next_pair_value = |parser: &mut Self| -> Result<Option<Expression>> {
            let Some(colon) = parser.next_symbol(":", true)? else {
                return Ok(None);
            };
            match parser.next_expression(true)? {
                Some(expr) => Ok(Some(expr)),
                None => Err(parser.error(colon, ErrorKind::ExpectedExpr)),
            }
        };
        if let Some(start) = self.next_symbol("[", true)? {
            let Some(expr) = self.next_expression(true)? else {
                return Err(self.error(start, ErrorKind::ExpectedExpr));
            };
            let Some(end) = self.next_symbol("]", true)? else {
                return Err(self.error(start, ErrorKind::ExpectedRightSquare));
            };
            let Some(value) = next_pair_value(self)? else {
                return Err(self.error(start.concat(end), ErrorKind::ExpectedColon));
            };
            Ok(Some(Pair::Index(SpanOf(start.concat(end), expr), value)))
        } else if let Some(star) = self.next_symbol("*", true)? {
            let Some(expr) = self.next_expression(true)? else {
                return Err(self.error(star, ErrorKind::ExpectedExpr));
            };
            Ok(Some(Pair::Unpack(SpanOf(star.concat(expr.span()), expr))))
        } else if let Some(key) = self.next_ident(true)? {
            let value = next_pair_value(self)?.unwrap_or(Expression::Ident(key.clone()));
            Ok(Some(Pair::Ident(key, value)))
        } else {
            Ok(None)
        }
    }
    fn next_object(&mut self, skip_newline: bool) -> Result<Option<Expression>> {
        let Some(start) = self.next_symbol("{", skip_newline)? else {
            return Ok(None);
        };
        let mut pairs = vec![];
        while let Some(pair) = self.next_pair()? {
            pairs.push(pair);
            if self.next_symbol(",", true)?.is_none() {
                break;
            }
        }
        let Some(end) = self.next_symbol("}", true)? else {
            return Err(self.error(start, ErrorKind::ExpectedRightCurly));
        };
        Ok(Some(Expression::Object(SpanOf(start.concat(end), pairs))))
    }
    pub fn next_primary(&mut self, skip_newline: bool) -> Result<Option<Expression>> {
        let methods = [
            Self::next_group,
            Self::next_array,
            Self::next_object,
            Self::next_primitive,
        ];
        for method in methods {
            if let Some(expr) = method(self, skip_newline)? {
                return Ok(Some(expr));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_primary() {
        let mut parser = Parser::new(
            r#"[1, 2, 3]
            [4,
            [5, 6],
            ]
            { *obj, a: 1,
             b: 2}
            [1, 2, *a, *b, 3]
            {x,y,z, *{x: 2, y: 3}}
            { [0]: 0, [1]: 3,
                ["test"]: "no", ident: x, *unpack ,}"#
                .as_bytes(),
        );
        let answers = [
            "[1, 2, 3]",
            "[4, [5, 6]]",
            "{*obj, a: 1, b: 2}",
            "[1, 2, *a, *b, 3]",
            "{x: x, y: y, z: z, *{x: 2, y: 3}}",
            "{[0]: 0, [1]: 3, [\"test\"]: \"no\", ident: x, *unpack}",
        ];
        for answer in answers {
            parser.skip_seperator().unwrap();
            let result = parser.next_primary(false).unwrap().unwrap().to_string();
            assert_eq!(answer, result);
        }
    }
}
