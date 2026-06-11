use crate::ast::{expression::Expression, *};

impl<R: BufRead> Parser<R> {
    fn next_element(&mut self, skip_newline: bool) -> Result<Option<Element>> {
        let Some(star) = self.next_symbol("*", skip_newline)? else {
            return Ok(self.next_expression(skip_newline)?.map(Element::Regular));
        };
        Ok(self
            .next_expression(skip_newline)?
            .map(|expr| Element::Unpacking(SpanOf(star.concat(expr.span()), expr))))
    }
    pub fn next_elements(&mut self, skip_newline: bool) -> Result<Vec<Element>> {
        let Some(mut elements) = self.next_element(skip_newline)?.map(|expr| vec![expr]) else {
            return Ok(vec![]);
        };
        while self.next_symbol(",", skip_newline)?.is_some() {
            let Some(element) = self.next_element(skip_newline)? else {
                return Ok(elements);
            };
            elements.push(element);
        }
        Ok(elements)
    }
    fn next_group(&mut self, skip_newline: bool) -> Result<Option<Expression>> {
        let Some(start) = self.next_symbol("(", skip_newline)? else {
            return Ok(None);
        };
        let Some(expr) = self.next_expression(skip_newline)? else {
            return Err(self.error(start, ErrorKind::ExpectedExpr));
        };
        let Some(_) = self.next_symbol(")", skip_newline)? else {
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
            Ok(Some(Pair::Index(
                SpanOf(start.concat(end), Box::new(expr)),
                Box::new(value),
            )))
        } else if let Some(key) = self.next_ident(true)? {
            let value = next_pair_value(self)?.unwrap_or(Expression::Ident(key.clone()));
            Ok(Some(Pair::Ident(key, Box::new(value))))
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
        Ok(Some(if let Some(tuple) = self.next_group(skip_newline)? {
            tuple
        } else if let Some(array) = self.next_array(skip_newline)? {
            array
        } else if let Some(object) = self.next_object(skip_newline)? {
            object
        } else if let Some(primitive) = self.next_primitive(skip_newline)? {
            primitive
        } else {
            return Ok(None);
        }))
    }
}

#[derive(Debug)]
pub enum Element {
    Regular(Expression),
    Unpacking(SpanOf<Expression>),
}
impl fmt::Display for Element {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Regular(expr) => write!(f, "{}", expr),
            Self::Unpacking(unpacking) => write!(f, "*{}", unpacking.1),
        }
    }
}
impl GetSpan for Element {
    fn span(&self) -> Span {
        match self {
            Self::Regular(expr) => expr.span(),
            Self::Unpacking(unpacking) => unpacking.0,
        }
    }
}

#[derive(Debug)]
pub enum Pair {
    Ident(SourceSpan, Box<Expression>),
    Index(SpanOf<Box<Expression>>, Box<Expression>),
}
impl fmt::Display for Pair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ident(ident, expr) => write!(f, "{ident}: {expr}"),
            Self::Index(key, value) => write!(f, "[{}]: {}", key.1, value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_array() {
        let mut parser = Parser::new(
            "
        [1,
        [2,
        3,
        4],*[5]]
        [[1, 2],
        [[(3)],
        [4,]], 5,]"
                .as_bytes(),
        );
        let answers = ["[1, [2, 3, 4], *[5]]", "[[1, 2], [[3], [4]], 5]"];
        for answer in answers {
            parser.skip_seperator().unwrap();
            let result = parser.next_array(false).unwrap().unwrap().to_string();
            assert_eq!(result, answer);
        }
    }
    #[test]
    fn parse_primary() {
        let mut parser = Parser::new(
            r#"[1, 2, 3]
            [4, 
            [5, 6],
            ]
            [1, 2, *a, *b, 3]
            { a: 1, 
             b: 2}
            {x,y,z,}
            { [0]: 0, [1]: 3,
                ["test"]: "no" ,}"#
                .as_bytes(),
        );
        let answers = [
            "[1, 2, 3]",
            "[4, [5, 6]]",
            "[1, 2, *a, *b, 3]",
            "{a: 1, b: 2}",
            "{x: x, y: y, z: z}",
            "{[0]: 0, [1]: 3, [\"test\"]: \"no\"}",
        ];
        for answer in answers {
            parser.skip_seperator().unwrap();
            let result = parser.next_primary(false).unwrap().unwrap().to_string();
            assert_eq!(answer, result);
        }
    }
}
