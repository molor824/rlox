use super::*;

#[derive(Debug, Clone)]
pub struct BinaryOperator(pub SpanOf<&'static str>);
impl fmt::Display for BinaryOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0 .1)
    }
}
impl GetSpan for BinaryOperator {
    fn span(&self) -> Span {
        self.0 .0
    }
}

impl<R: BufRead> Parser<R> {
    pub fn next_binary(&mut self, skip_newline: bool) -> Result<Option<Expression>> {
        self.next_logical_or(skip_newline)
    }
    fn next_binary_operator(
        &mut self,
        operators: impl IntoIterator<Item = &'static str>,
        skip_newline: bool,
    ) -> Result<Option<BinaryOperator>> {
        let prev = self.clone();
        let mut operator: Option<(BinaryOperator, Self)> = None;
        for op in operators.into_iter() {
            if let Some(span) = self.next_symbol(op, skip_newline)? {
                operator = match operator {
                    Some((op1, _)) if op1.0 .0.len() < span.len() => {
                        Some((BinaryOperator(SpanOf(span, op)), self.clone()))
                    }
                    None => Some((BinaryOperator(SpanOf(span, op)), self.clone())),
                    o => o,
                };
                *self = prev.clone();
            }
        }
        match operator {
            Some((op, next)) => {
                *self = next;
                Ok(Some(op))
            }
            None => Ok(None),
        }
    }
    fn next_left_binary(
        &mut self,
        mut lower: impl FnMut(&mut Self) -> Result<Option<Expression>>,
        mut operator: impl FnMut(&mut Self) -> Result<Option<BinaryOperator>>,
    ) -> Result<Option<Expression>> {
        let Some(mut expr) = lower(self)? else {
            return Ok(None);
        };

        while let Some(op) = operator(self)? {
            let Some(right_operand) = lower(self)? else {
                return Err(self.error_to_here(op.span().start, ErrorKind::ExpectedExpr));
            };
            expr = Expression::Binary {
                left_operand: Box::new(expr),
                operator: op,
                right_operand: Box::new(right_operand),
            };
        }
        Ok(Some(expr))
    }
    fn next_logical_or(&mut self, skip_newline: bool) -> Result<Option<Expression>> {
        self.next_left_binary(
            |parser| parser.next_logical_and(skip_newline),
            |parser| {
                if let Some(op) = parser
                    .next_keyword("or", skip_newline)
                    .map(|i| i.map(|i| BinaryOperator(SpanOf(i.0, "||"))))?
                {
                    Ok(Some(op))
                } else {
                    parser.next_binary_operator(["||"], skip_newline)
                }
            },
        )
    }
    fn next_logical_and(&mut self, skip_newline: bool) -> Result<Option<Expression>> {
        self.next_left_binary(
            |parser| parser.next_bitwise_or(skip_newline),
            |parser| {
                if let Some(op) = parser
                    .next_keyword("and", skip_newline)
                    .map(|i| i.map(|i| BinaryOperator(SpanOf(i.0, "&&"))))?
                {
                    Ok(Some(op))
                } else {
                    parser.next_binary_operator(["&&"], skip_newline)
                }
            },
        )
    }
    fn next_bitwise_or(&mut self, skip_newline: bool) -> Result<Option<Expression>> {
        self.next_left_binary(
            |parser| parser.next_bitwise_xor(skip_newline),
            |parser| {
                let mut peek = parser.clone();
                if let Ok(Some(..)) = peek.next_symbol("||", skip_newline) {
                    return Ok(None);
                }
                parser.next_binary_operator(["|"], skip_newline)
            },
        )
    }
    fn next_bitwise_xor(&mut self, skip_newline: bool) -> Result<Option<Expression>> {
        self.next_left_binary(
            |parser| parser.next_bitwise_and(skip_newline),
            |parser| parser.next_binary_operator(["^"], skip_newline),
        )
    }
    fn next_bitwise_and(&mut self, skip_newline: bool) -> Result<Option<Expression>> {
        self.next_left_binary(
            |parser| parser.next_equality(skip_newline),
            |parser| {
                let mut peek = parser.clone();
                if let Ok(Some(..)) = peek.next_symbol("&&", skip_newline) {
                    return Ok(None);
                }
                parser.next_binary_operator(["&"], skip_newline)
            },
        )
    }
    fn next_equality(&mut self, skip_newline: bool) -> Result<Option<Expression>> {
        self.next_left_binary(
            |parser| parser.next_comparison(skip_newline),
            |parser| parser.next_binary_operator(["==", "!="], skip_newline),
        )
    }
    fn next_comparison(&mut self, skip_newline: bool) -> Result<Option<Expression>> {
        self.next_left_binary(
            |parser| parser.next_shift(skip_newline),
            |parser| parser.next_binary_operator(["<", ">", "<=", ">="], skip_newline),
        )
    }
    fn next_shift(&mut self, skip_newline: bool) -> Result<Option<Expression>> {
        self.next_left_binary(
            |parser| parser.next_addition(skip_newline),
            |parser| parser.next_binary_operator([">>", "<<", ">>>"], skip_newline),
        )
    }
    fn next_addition(&mut self, skip_newline: bool) -> Result<Option<Expression>> {
        self.next_left_binary(
            |parser| parser.next_multiplication(skip_newline),
            |parser| parser.next_binary_operator(["+", "-"], skip_newline),
        )
    }
    fn next_multiplication(&mut self, skip_newline: bool) -> Result<Option<Expression>> {
        self.next_left_binary(
            |parser| parser.next_power(skip_newline),
            |parser| parser.next_binary_operator(["*", "/"], skip_newline),
        )
    }
    fn next_power(&mut self, skip_newline: bool) -> Result<Option<Expression>> {
        self.next_left_binary(
            |parser| parser.next_unary(skip_newline),
            |parser| parser.next_binary_operator(["**"], skip_newline),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_binary() {
        let question =
            "-(3).add(1) + 1 * 6 / 2\n1 + 2 + 3\n+ (\t4 + 5\t) * 6\n1!=0 and 3 <= 3 or 3>2";
        let answers = [
            "(+ (- ((1) (.add 3))) (/ (* 1 6) 2))",
            "(+ (+ (+ 1 2) 3) (* (+ 4 5) 6))",
            "(|| (&& (!= 1 0) (<= 3 3)) (> 3 2))",
        ];
        let mut parser = Parser::new(question.as_bytes());

        for answer in answers {
            let result = parser.next_expression(true).unwrap().unwrap().to_string();
            assert_eq!(answer, result);
        }
    }
}
