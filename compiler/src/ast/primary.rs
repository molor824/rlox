use crate::ast::*;

impl<R: BufRead> Parser<R> {
    fn next_element(&mut self, skip_newline: bool) -> Result<Option<Element>> {
        let Some(star) = self.next_symbol("*", skip_newline)? else {
            return Ok(self.next_expression(skip_newline)?.map(Element::Regular));
        };
        Ok(self
            .next_expression(skip_newline)?
            .map(|expr| Element::Unpacking(SpanOf(star.concat(expr.span()), expr))))
    }
    pub fn next_elements(&mut self, skip_newline: bool) -> Result<Option<SpanOf<Vec<Element>>>> {
        let Some(mut elements) = self
            .next_element(skip_newline)?
            .map(|expr| SpanOf(expr.span(), vec![expr]))
        else {
            return Ok(None);
        };
        while self.next_symbol(",", skip_newline)?.is_some() {
            let Some(element) = self.next_element(skip_newline)? else {
                return Ok(Some(elements));
            };
            elements.0 = elements.0.concat(element.span());
            elements.1.push(element);
        }
        Ok(Some(elements))
    }

    /// Returns either tuple or group expression. (a) - group expression, (a,) - tuple
    fn next_group(&mut self, skip_newline: bool) -> Result<Option<Expression>> {
        let Some(start) = self.next_symbol("(", skip_newline)? else {
            return Ok(None);
        };
        let Some(expr) = self.next_expression(skip_newline)? else {
            return Err(self.error(start, ErrorKind::ExpectedExpr));
        };
        let Some(end) = self.next_symbol(")", skip_newline)? else {
            return Err(self.error(expr.span(), ErrorKind::ExpectedRightParen));
        };
        Ok(Some(Expression::Group(SpanOf(start.concat(end), expr.into()))))
    }
    fn next_array(&mut self, skip_newline: bool) -> Result<Option<Expression>> {
        let Some(start) = self.next_symbol("[", skip_newline)? else {
            return Ok(None);
        };
        let elements = self
            .next_elements(skip_newline)?
            .map(|expr| expr.1)
            .unwrap_or(vec![]);
        let Some(end) = self.next_symbol("]", skip_newline)? else {
            return Err(self.error(start, ErrorKind::ExpectedRightSquare));
        };
        Ok(Some(Expression::Array(SpanOf(start.concat(end), elements))))
    }
    pub fn next_primary(&mut self, skip_newline: bool) -> Result<Option<Expression>> {
        Ok(Some(if let Some(tuple) = self.next_group(skip_newline)? {
            tuple
        } else if let Some(array) = self.next_array(skip_newline)? {
            array
        } else if let Some(primitive) = self.next_primitive(skip_newline)? {
            primitive
        } else {
            return Ok(None);
        }))
    }
}

#[derive(Debug, Clone)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tuple() {
        let mut parser = Parser::new(
            "(1, 2, 3) ((1, 2), (3, (4,))) ((1), (2,), (3, 4))(*(1, 2,), 3, *(4,))".as_bytes(),
        );
        let answers = [
            "t[1,2,3]",
            "t[t[1,2],t[3,t[4]]]",
            "t[1,t[2],t[3,4]]",
            "t[*t[1,2],3,*t[4]]",
        ];
        for answer in answers {
            let result = parser.next_group(true).unwrap().unwrap().to_string();
            assert_eq!(result, answer);
        }
    }
    #[test]
    fn parse_array() {
        let mut parser = Parser::new(
            "
        [1,
        (2,
        3,
        4),*[5]]
        [[1, 2],
        [[(3)],
        (4,)], 5,]"
                .as_bytes(),
        );
        let answers = ["[1,t[2,3,4],*[5]]", "[[1,2],[[3],t[4]],5]"];
        for answer in answers {
            let result = parser.next_array(true).unwrap().unwrap().to_string();
            assert_eq!(result, answer);
        }
    }
}
