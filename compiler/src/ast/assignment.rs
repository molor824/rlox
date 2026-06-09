use crate::ast::{
    expression::Expression,
    *,
};

#[derive(Debug)]
pub enum Assignee {
    Ident(SourceSpan),
    Property {
        ident: SourceSpan,
        operand: Box<Expression>,
    },
    Index {
        arg: SpanOf<Box<Expression>>,
        operand: Box<Expression>,
    }
}

impl fmt::Display for Assignee {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ident(ident) => write!(f, "{ident}"),
            Self::Property { ident, operand } => write!(f, "(.{ident} {operand})"),
            Self::Index { arg, operand } => write!(f, "([{}] {operand})", arg.1),
        }
    }
}

impl GetSpan for Assignee {
    fn span(&self) -> Span {
        match self {
            Self::Ident(ident) => ident.0,
            Self::Property { ident, operand } => ident.0.concat(operand.span()),
            Self::Index { arg, operand } => arg.0.concat(operand.span()),
        }
    }
}

impl<R: BufRead> Parser<R> {
    pub fn next_assignment(&mut self, skip_newline: bool) -> Result<Option<Expression>> {
        let mut chain = vec![];
        let lower = |parser: &mut Self| parser.next_binary(skip_newline);

        loop {
            let prev = self.clone();
            let Ok(Some(assignee)) = lower(self) else {
                *self = prev;
                break;
            };

            let Some(equal) = self.next_symbol("=", skip_newline)? else {
                *self = prev;
                break;
            };
            chain.push((assignee, equal));
        }
        let Some(mut expr) = lower(self)? else {
            if let Some((_, equal)) = chain.last() {
                return Err(self.error(*equal, ErrorKind::ExpectedExpr));
            } else {
                return Ok(None);
            }
        };
        while let Some((assignee_expr, _)) = chain.pop() {
            let assignee_span = assignee_expr.span();
            let invalid_assignee_error = |parser: &mut Self| Err(
                parser.error(assignee_span, ErrorKind::InvalidAssignee)
            );
            let assignee = match assignee_expr {
                Expression::Ident(ident) => Assignee::Ident(ident),
                Expression::Postfix { operator, operand } => match operator {
                    PostfixOperator::Property(ident) => Assignee::Property { ident, operand },
                    PostfixOperator::Index(arg) => Assignee::Index { arg, operand },
                    _ => return invalid_assignee_error(self),
                },
                _ => return invalid_assignee_error(self),
            };

            expr = Expression::Assign {
                assignee,
                assigner: Box::new(expr),
            };
        }
        Ok(Some(expr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_assignment() {
        let question = r"
        a = b
        a.x = b.y = 2
        a[0] = b[1] = c[2] + d[3] + e[4]";
        let answers = [
            "(= a b)",
            "(= (.x a) (= (.y b) 2))",
            "(= ([0] a) (= ([1] b) (+ (+ ([2] c) ([3] d)) ([4] e))))",
        ];

        let mut parser = Parser::new(question.as_bytes());
        for answer in answers {
            parser.skip_seperator().unwrap();
            let result = parser.next_assignment(false).unwrap().unwrap().to_string();
            assert_eq!(result, answer);
        }
    }
}
