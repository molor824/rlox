use super::*;

impl<B: BufRead> Parser<B> {
    fn next_postfix_operators(&mut self, skip_newline: bool) -> Result<Option<Expression>> {
        let Some(operand) = self.next_primary(skip_newline)? else {
            return Ok(None);
        };
        let mut expr = operand;

        loop {
            let operator = if let Some(dot) = self.next_symbol(".", skip_newline)? {
                // Property
                let Some(ident) = self.next_ident(skip_newline)? else {
                    return Err(self.error(dot, ErrorKind::ExpectedIdent));
                };
                PostfixOperator::Property(ident)
            } else if let Some(left) = self.next_symbol("(", skip_newline)? {
                // Call
                let elements = self
                    .next_elements(skip_newline)?
                    .map(|expr| expr.1)
                    .unwrap_or(vec![]);
                let Some(right) = self.next_symbol(")", skip_newline)? else {
                    return Err(self.error(left, ErrorKind::ExpectedRightParen));
                };
                PostfixOperator::Call(SpanOf(left.concat(right), elements))
            } else if let Some(left) = self.next_symbol("[", skip_newline)? {
                // Indexing
                let elements = self
                    .next_elements(skip_newline)?
                    .map(|expr| expr.1)
                    .unwrap_or(vec![]);
                let Some(right) = self.next_symbol("]", skip_newline)? else {
                    return Err(self.error(left, ErrorKind::ExpectedRightSquare));
                };
                PostfixOperator::Index(SpanOf(left.concat(right), elements))
            } else {
                return Ok(Some(expr));
            };
            expr = Expression::Postfix {
                operator,
                operand: Box::new(expr),
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_postfix() {
        let question = "a.b.c[1,](2) (3 , 4)[5, 6, 7, ] (*a)";
        let answer = "a.b.c[1](2)(3,4)[5,6,7](*a)";
        let mut parser = Parser::new(question.as_bytes());
        let result = parser
            .next_postfix_operators(false)
            .unwrap()
            .unwrap()
            .to_string();
        assert_eq!(result, answer);
    }
}
