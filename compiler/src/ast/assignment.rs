use crate::ast::{expression::*, *};

impl<R: BufRead> Parser<R> {
    fn expr_to_assignee(&self, expression: Expression) -> Result<Assignee> {
        let span = expression.span();
        match expression {
            Expression::Ident(ident) => Ok(Assignee::Ident(ident)),
            Expression::Postfix { operator, operand } => match operator {
                PostfixOperator::Property(ident) => Ok(Assignee::Property { ident, operand }),
                PostfixOperator::Index(arg) => Ok(Assignee::Index { arg, operand }),
                _ => Err(self.error(span, ErrorKind::InvalidAssignee)),
            },
            _ => Err(self.error(span, ErrorKind::InvalidAssignee)),
        }
    }
    fn next_params(&mut self) -> Result<(Vec<SourceSpan>, Option<SpanOf<SourceSpan>>)> {
        let mut params = vec![];
        let mut variadic = None;

        loop {
            let star = self.next_symbol("*", true)?;
            let Some(ident) = self.next_ident(true)? else {
                match star {
                    Some(star) => return Err(self.error(star, ErrorKind::ExpectedIdent)),
                    None => break,
                }
            };
            if let Some(star) = star {
                variadic = Some(SpanOf(star, ident));
                break;
            } else {
                params.push(ident);
            }
            if self.next_symbol(",", true)?.is_none() {
                break;
            }
        }
        Ok((params, variadic))
    }
    fn next_body(&mut self, skip_newline: bool) -> Result<Option<FunctionBody>> {
        if let Some(arrow) = self.next_symbol("=>", skip_newline)? {
            let Some(expr) = self.next_expression(skip_newline)? else {
                return Err(self.error(arrow, ErrorKind::ExpectedExpr));
            };
            Ok(Some(FunctionBody::Expression(SpanOf(
                arrow,
                Box::new(expr),
            ))))
        } else if let Some(do_block) = self.next_do_block(skip_newline)? {
            Ok(Some(FunctionBody::Block(do_block)))
        } else {
            Ok(None)
        }
    }
    fn next_function_decl(&mut self, skip_newline: bool) -> Result<Option<Expression>> {
        let Some(fn_kwd) = self.next_keyword("fn", skip_newline)? else {
            return Ok(None);
        };

        let assignee = match self.next_expression(skip_newline)? {
            Some(expr) => Some(self.expr_to_assignee(expr)?),
            None => None,
        };

        let Some(paren_start) = self.next_symbol("(", skip_newline)? else {
            return Err(self.error(fn_kwd.0, ErrorKind::ExpectedLeftParen));
        };
        let (params, variadic) = self.next_params()?;

        let Some(paren_end) = self.next_symbol(")", true)? else {
            return Err(self.error(paren_start, ErrorKind::ExpectedRightParen));
        };

        let Some(body) = self.next_body(skip_newline)? else {
            return Err(self.error(fn_kwd.0.concat(paren_end), ErrorKind::ExpectedFuncBody));
        };

        Ok(Some(Expression::FunctionDecl {
            fn_keyword: fn_kwd.0,
            assignee,
            params,
            variadic,
            body,
        }))
    }
    pub fn next_assignment(&mut self, skip_newline: bool) -> Result<Option<Expression>> {
        let lower = |parser: &mut Self| parser.next_binary(skip_newline);

        let Some(assignee) = lower(self)? else {
            return Ok(None);
        };

        let Some(equal) = self.next_symbol("=", skip_newline)? else {
            return Ok(Some(assignee));
        };

        let Some(assigner) = self.next_expression(skip_newline)? else {
            return Err(self.error(equal, ErrorKind::ExpectedExpr));
        };

        Ok(Some(Expression::Assign {
            assignee: self.expr_to_assignee(assignee)?,
            assigner: Box::new(assigner),
        }))
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
            "(a) = (b)",
            "((a).x) = (((b).y) = (2))",
            "((a)[0]) = (((b)[1]) = ((((c)[2]) + ((d)[3])) + ((e)[4])))",
        ];

        let mut parser = Parser::new(question.as_bytes());
        for answer in answers {
            parser.skip_seperator().unwrap();
            let result = parser.next_assignment(false).unwrap().unwrap().to_string();
            assert_eq!(result, answer);
        }
    }
}
