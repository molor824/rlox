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
    fn next_prefix_operators(&mut self, skip_newline: bool) -> Result<Option<Expression>> {
        let mut operators = vec![];
        loop {
            if let Some(ch) = self.next_symbols(["-", "~", "!"], skip_newline)? {
                operators.push(PrefixOperator(ch));
            } else if let Some(not) = self.next_keyword("not", skip_newline)? {
                operators.push(PrefixOperator(not.map(|_| "!")));
            } else {
                break;
            }
        }

        let Some(mut expr) = self.next_postfix_operators(skip_newline)? else {
            if let Some(op) = operators.last() {
                return Err(self.error(op.span(), ErrorKind::ExpectedExpr));
            } else {
                return Ok(None);
            }
        };

        while let Some(op) = operators.pop() {
            expr = Expression::Prefix {
                operator: op,
                operand: Box::new(expr),
            };
        }
        Ok(Some(expr))
    }
    pub fn next_unary(&mut self, skip_newline: bool) -> Result<Option<Expression>> {
        self.next_prefix_operators(skip_newline)
    }
}

#[derive(Debug, Clone)]
pub struct PrefixOperator(pub SpanOf<&'static str>);
impl fmt::Display for PrefixOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0 .1)
    }
}
impl GetSpan for PrefixOperator {
    fn span(&self) -> Span {
        self.0 .0
    }
}
#[derive(Debug, Clone)]
pub enum PostfixOperator {
    Property(SpanOf<CachedString>),
    Call(SpanOf<Vec<Element>>),
    Index(SpanOf<Vec<Element>>),
}
impl fmt::Display for PostfixOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Property(property) => write!(f, ".{}", property.1),
            Self::Call(args) => write!(
                f,
                "({})",
                args.1
                    .iter()
                    .map(|arg| arg.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            Self::Index(args) => write!(
                f,
                "[{}]",
                args.1
                    .iter()
                    .map(|arg| arg.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        }
    }
}
impl GetSpan for PostfixOperator {
    fn span(&self) -> Span {
        match self {
            Self::Property(p) => p.0,
            Self::Call(c) => c.0,
            Self::Index(i) => i.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_postfix() {
        let question = "a.b.c[1,](2) (3 , 4)[5, 6, 7, ] (*a)";
        let answer = "(((((((a).b).c)[1])(2))(3,4))[5,6,7])(*a)";
        let mut parser = Parser::new(question.as_bytes());
        let result = parser
            .next_postfix_operators(false)
            .unwrap()
            .unwrap()
            .to_string();
        assert_eq!(result, answer);
    }
    #[test]
    fn parse_unary() {
        let question = "\t-- -  !~ ~ !  \t\t !! a . b . c (    d , e ) [ f , ] ";
        let answer = "-(-(-(!(~(~(!(!(!(((((a).b).c)(d,e))[f])))))))))";
        let mut parser = Parser::new(question.as_bytes());
        let result = parser.next_unary(false).unwrap().unwrap().to_string();
        assert_eq!(result, answer)
    }
}
