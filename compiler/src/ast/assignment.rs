use crate::ast::{expression::*, *};

impl<R: BufRead> Parser<R> {
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
        } else if let Some(block) = self.next_do_block(skip_newline)? {
            Ok(Some(FunctionBody::Block(block)))
        } else {
            Ok(None)
        }
    }
    fn next_function_decl(&mut self, skip_newline: bool) -> Result<Option<Expression>> {
        let Some(fn_kwd) = self.next_keyword("fn", skip_newline)? else {
            return Ok(None);
        };

        let ident = self.next_ident(skip_newline)?;

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
            ident,
            params,
            variadic,
            body,
        }))
    }
    pub fn next_var_decl(&mut self, skip_newline: bool) -> Result<Option<Expression>> {
        let Some(var_kwd) = self.next_keywords(["let", "const"], skip_newline)? else {
            return Ok(None);
        };

        let Some(ident) = self.next_ident(skip_newline)? else {
            return Err(self.error(var_kwd.0, ErrorKind::ExpectedIdent));
        };

        let Some(eq) = self.next_symbol("=", skip_newline)? else {
            return Err(self.error(ident.0, ErrorKind::ExpectedEq));
        };

        let Some(assigner) = self.next_expression(skip_newline)? else {
            return Err(self.error(eq, ErrorKind::ExpectedExpr));
        };

        Ok(Some(Expression::VarDecl {
            keyword: var_kwd,
            ident,
            assigner: Box::new(assigner),
        }))
    }
    pub fn expr_to_assignee(&self, expression: Expression) -> Result<Assignee> {
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
    pub fn next_assignment(&mut self, skip_newline: bool) -> Result<Option<Expression>> {
        if let Some(decl) = self.next_function_decl(skip_newline)? {
            return Ok(Some(decl));
        } else if let Some(decl) = self.next_var_decl(skip_newline)? {
            return Ok(Some(decl));
        }

        let Some(assignee) = self.next_binary(skip_newline)? else {
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
        a = b = c
        a.x = b.y = 2
        a[0] = b[1] = c[2] + d[3] + e[4]
        let a = const b = 3
        fn add(a, b) => a + b
        fn sum(*values) do
            let total = 0
            let i = 0
            while i < values:len() do
                total = total + values[i]
                i = i + 1
            end
        end
        let a = { x: 0, y: 0 }
        a.magnitude = fn(self) => sqrt(self.x * self.x + self.y * self.y)
        print(a:magnitude())
        ";
        let answers = [
            "(a) = ((b) = (c))",
            "((a).x) = (((b).y) = (2))",
            "((a)[0]) = (((b)[1]) = ((((c)[2]) + ((d)[3])) + ((e)[4])))",
            "let a = (const b = (3))",
            "fn add(a, b) => (a) + (b)",
            "fn sum(*values) do
. let total = (0)
. let i = (0)
. while (i) < (((values):len)()) do
. . (total) = ((total) + ((values)[i]))
. . (i) = ((i) + (1))
. end
end",
            "let a = ({x: 0, y: 0})",
            "((a).magnitude) = (fn(self) => (sqrt)((((self).x) * ((self).x)) + (((self).y) * ((self).y))))",
            "(print)(((a):magnitude)())"
        ];

        let mut parser = Parser::new(question.as_bytes());
        for answer in answers {
            parser.skip_seperator().unwrap();
            let result = parser.next_assignment(false).unwrap().unwrap().to_string();
            assert_eq!(result, answer);
        }
    }
}
