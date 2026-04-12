use std::io::Read;

use crate::ast::*;

impl<R: Read> Parser<R> {
    pub fn next_symbol(&mut self, symbol: &str, skip_newline: bool) -> Result<Option<Span>> {
        self.skip(skip_newline)?;
        self.next_sequence(symbol)
    }

    fn next_expressions(&mut self, skip_newline: bool) -> Result<Option<SpanOf<Vec<Expression>>>> {
        let Some(mut expressions) = self
            .next_expression(skip_newline)?
            .map(|expr| SpanOf(expr.span(), vec![expr]))
        else {
            return Ok(None);
        };
        while let Some(comma) = self.next_symbol(",", skip_newline)? {
            let Some(expression) = self.next_expression(skip_newline)? else {
                expressions.0 = expressions.0.concat(comma);
                return Ok(Some(expressions));
            };
            expressions.0 = expressions.0.concat(expression.span());
            expressions.1.push(expression);
        }
        Ok(Some(expressions))
    }

    /// Returns either tuple or group expression. (a) - group expression, (a,) - tuple
    fn next_tuple(&mut self, skip_newline: bool) -> Result<Option<Expression>> {
        let Some(start) = self.next_symbol("(", skip_newline)? else {
            return Ok(None);
        };
        let Some(expression) = self.next_expression(true)? else {
            // might be an empty tuple
            let Some(end) = self.next_symbol(")", true)? else {
                return Err(self.error(start, ErrorKind::ExpectedRightParen));
            };
            return Ok(Some(Expression::Tuple(SpanOf(start.concat(end), vec![]))));
        };
        // Check if its a tuple
        if self.next_symbol(",", true)?.is_some() {
            // Start tuple mode
            let mut expressions = self
                .next_expressions(true)?
                .map(|expr| expr.1)
                .unwrap_or(vec![]);
            let Some(end) = self.next_symbol(")", true)? else {
                return Err(self.error_to_here(start.start, ErrorKind::ExpectedRightParen));
            };
            expressions.insert(0, expression);
            Ok(Some(Expression::Tuple(SpanOf(
                start.concat(end),
                expressions,
            ))))
        } else {
            // Group mode
            let Some(end) = self.next_symbol(")", true)? else {
                return Err(self.error_to_here(start.start, ErrorKind::ExpectedRightParen));
            };
            Ok(Some(Expression::Group(SpanOf(
                start.concat(end),
                Box::new(expression),
            ))))
        }
    }
    fn next_array(&mut self, skip_newline: bool) -> Result<Option<Expression>> {
        let Some(start) = self.next_symbol("[", skip_newline)? else {
            return Ok(None);
        };
        let expressions = self.next_expressions(skip_newline)?;
        let Some(end) = self.next_symbol("]", skip_newline)? else {
            return Err(self.error_to_here(start.start, ErrorKind::ExpectedRightSquare));
        };
        Ok(Some(Expression::Array(SpanOf(
            start.concat(end),
            expressions.map(|expr| expr.1).unwrap_or(vec![]),
        ))))
    }
    pub fn next_primary(&mut self, skip_newline: bool) -> Result<Option<Expression>> {
        Ok(Some(if let Some(tuple) = self.next_tuple(skip_newline)? {
            tuple
        } else if let Some(primitive) = self.next_primitive(skip_newline)? {
            primitive
        } else if let Some(array) = self.next_array(skip_newline)? {
            array
        } else {
            return Ok(None);
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tuple() {
        let mut parser =
            Parser::new("(1, 2, 3) ((1, 2), (3, (4,))) ((1), (2,), (3, 4))".as_bytes());
        let answers = ["(1,2,3,)", "((1,2,),(3,(4,),),)", "((1),(2,),(3,4,),)"];
        for answer in answers {
            let result = parser.next_expression(true).unwrap().unwrap().to_string();
            assert_eq!(result, answer);
        }
    }
    #[test]
    fn parse_array() {
        let mut parser = Parser::new("
        [1,
        (2,
        3,
        4)]
        [[1, 2],
        [[(3)],
        (4,)], 5,]".as_bytes());
        let answers = ["[1,(2,3,4,),]", "[[1,2,],[[(3),],(4,),],5,]"];
        for answer in answers {
            let result = parser.next_array(true).unwrap().unwrap().to_string();
            assert_eq!(result, answer);
        }
    }
}
