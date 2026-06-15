use crate::ast::{expression::*, *};

pub fn print_indent(statements: &[Statement], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for statement in statements {
        writeln!(f, ". {}", statement.to_string().replace("\n", "\n. "))?;
    }
    Ok(())
}

#[derive(Debug)]
pub enum Statement {
    Expression(Expression),
    If {
        span: Span,
        condition: Expression,
        met_block: SpanOf<Vec<Statement>>,
        else_block: Option<SpanOf<Vec<Statement>>>,
    },
    While {
        span: Span,
        condition: Expression,
        block: SpanOf<Vec<Statement>>,
    },
    For {
        span: Span,
        ident: SourceSpan,
        expr: Expression,
        block: SpanOf<Vec<Statement>>,
    },
}
impl fmt::Display for Statement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Expression(expr) => write!(f, "{expr}"),
            Self::If {
                condition,
                met_block,
                else_block,
                ..
            } => {
                writeln!(f, "if {condition} then")?;
                print_indent(&met_block.1, f)?;
                if let Some(else_block) = else_block {
                    writeln!(f, "else")?;
                    print_indent(&else_block.1, f)?;
                }
                write!(f, "end")
            }
            Self::While {
                condition, block, ..
            } => {
                writeln!(f, "while {condition} do")?;
                print_indent(&block.1, f)?;
                write!(f, "end")
            }
            Self::For {
                ident, expr, block, ..
            } => {
                writeln!(f, "for {ident} in {expr} do")?;
                print_indent(&block.1, f)?;
                write!(f, "end")
            }
        }
    }
}
impl GetSpan for Statement {
    fn span(&self) -> Span {
        match self {
            Self::Expression(expr) => expr.span(),
            Self::While { span, .. } => *span,
            Self::If { span, .. } => *span,
            Self::For { span, .. } => *span,
        }
    }
}

impl<R: BufRead> Parser<R> {
    fn next_terminators(&mut self) -> Result<Option<SourceSpan>> {
        const BLOCK_TERMINATORS: &[&str] = &["end", "else"];
        self.next_keywords(BLOCK_TERMINATORS.iter().copied(), true)
    }
    fn next_block(&mut self) -> Result<(Vec<Statement>, Option<SourceSpan>)> {
        let mut statements = vec![];

        loop {
            if let Some(terminator) = self.next_terminators()? {
                return Ok((statements, Some(terminator)));
            }
            if let Some(statement) = self.next_statement()? {
                statements.push(statement);
            }
            if !self.skip_seperator()? {
                return Ok((statements, self.next_terminators()?));
            }
        }
    }
    pub fn next_do_block(&mut self, skip_newline: bool) -> Result<Option<SpanOf<Vec<Statement>>>> {
        let Some(do_keyword) = self.next_keyword("do", skip_newline)? else {
            return Ok(None);
        };
        let (statements, Some(terminator)) = self.next_block()? else {
            return Err(self.error(do_keyword.0, ErrorKind::ExpectedEnd));
        };
        if &*terminator.get_str() != "end" {
            return Err(self.error(terminator.0, ErrorKind::ExpectedEnd));
        }
        Ok(Some(SpanOf(do_keyword.0.concat(terminator.0), statements)))
    }
    fn next_for_statement(&mut self) -> Result<Option<Statement>> {
        let Some(for_keyword) = self.next_keyword("for", true)? else {
            return Ok(None);
        };

        let Some(ident) = self.next_ident(true)? else {
            return Err(self.error(for_keyword.0, ErrorKind::ExpectedIdent));
        };

        let Some(in_keyword) = self.next_keyword("in", true)? else {
            return Err(self.error(ident.0, ErrorKind::ExpectedIn));
        };

        let Some(expr) = self.next_expression(true)? else {
            return Err(self.error(in_keyword.0, ErrorKind::ExpectedExpr));
        };

        let Some(block) = self.next_do_block(true)? else {
            return Err(self.error(expr.span(), ErrorKind::ExpectedDoBlock));
        };

        Ok(Some(Statement::For {
            span: for_keyword.0.concat(block.0),
            ident,
            expr,
            block,
        }))
    }
    fn next_while_statement(&mut self) -> Result<Option<Statement>> {
        let Some(while_keyword) = self.next_keyword("while", true)? else {
            return Ok(None);
        };
        let Some(condition) = self.next_expression(true)? else {
            return Err(self.error(while_keyword.0, ErrorKind::ExpectedExpr));
        };
        let Some(block) = self.next_do_block(true)? else {
            return Err(self.error(while_keyword.0, ErrorKind::ExpectedDoBlock));
        };
        Ok(Some(Statement::While {
            span: while_keyword.0.concat(block.0),
            condition,
            block,
        }))
    }
    fn next_if_statement(&mut self) -> Result<Option<Statement>> {
        let Some(if_keyword) = self.next_keyword("if", true)? else {
            return Ok(None);
        };
        let Some(condition) = self.next_expression(true)? else {
            return Err(self.error(if_keyword.0, ErrorKind::ExpectedExpr));
        };
        let Some(then_keyword) = self.next_keyword("then", true)? else {
            return Err(self.error(
                if_keyword.0.concat(condition.span()),
                ErrorKind::ExpectedThen,
            ));
        };
        let (met_block, Some(terminator)) = self.next_block()? else {
            return Err(self.error(if_keyword.0.concat(then_keyword.0), ErrorKind::ExpectedElse));
        };

        let keyword = terminator.get_str();

        match &*keyword {
            "end" => Ok(Some(Statement::If {
                span: if_keyword.0.concat(terminator.0),
                condition,
                met_block: SpanOf(then_keyword.0.concat(terminator.0), met_block),
                else_block: None,
            })),
            "else" => {
                drop(keyword); // self.next_if_statement will borrow the RefCell
                let met_block = SpanOf(
                    match met_block.last() {
                        // span covers `then ...` without covering else keyword
                        Some(stmt) => then_keyword.0.concat(stmt.span()),
                        None => then_keyword.0,
                    },
                    met_block,
                );
                if let Some(elif) = self.next_if_statement()? {
                    Ok(Some(Statement::If {
                        span: if_keyword.0.concat(elif.span()),
                        condition,
                        met_block,
                        else_block: Some(SpanOf(terminator.0.concat(elif.span()), vec![elif])),
                    }))
                } else {
                    let (else_block, Some(else_terminator)) = self.next_block()? else {
                        return Err(self.error(terminator.0, ErrorKind::ExpectedEnd));
                    };
                    if &*else_terminator.get_str() != "end" {
                        return Err(self.error(else_terminator.0, ErrorKind::ExpectedEnd));
                    }
                    Ok(Some(Statement::If {
                        span: if_keyword.0.concat(else_terminator.0),
                        condition,
                        met_block,
                        else_block: Some(SpanOf(
                            terminator.0.concat(else_terminator.0),
                            else_block,
                        )),
                    }))
                }
            }
            _ => unreachable!(),
        }
    }
    fn next_expr_statement(&mut self) -> Result<Option<Statement>> {
        self.next_expression(false)
            .map(|expr| expr.map(Statement::Expression))
    }
    pub fn next_statement(&mut self) -> Result<Option<Statement>> {
        let order = [
            Self::next_if_statement,
            Self::next_while_statement,
            Self::next_for_statement,
            Self::next_expr_statement,
        ];
        self.skip_seperator()?;
        for method in order {
            if let Some(stmt) = method(self)? {
                return Ok(Some(stmt));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_statements() {
        let question = r#"
        if true then
            print("Hello, world!");
            print("Semicolon is unnecessary, although it is optional!")

            ;;;;; -- Some weird fuck decided to spam semicolons, but it's technically valid code anyways.

        else if false then print("Inlining!")
        else print("Semicolon is necessary"); print("In this case!") end
        let i = 1
        while i < 100 do
            print(i)
            i = i * 2
        end
        for i in range(0, 100) do
            print(i)
        end
        "#;
        let answers = [
            r#"if true then
. (print)("Hello, world!")
. (print)("Semicolon is unnecessary, although it is optional!")
else
. if false then
. . (print)("Inlining!")
. else
. . (print)("Semicolon is necessary")
. . (print)("In this case!")
. end
end"#,
            "let i = (1)",
            "while (i) < (100) do
. (print)(i)
. (i) = ((i) * (2))
end",
            "for i in (range)(0, 100) do
. (print)(i)
end",
        ];

        let mut parser = Parser::new(question.as_bytes());

        for answer in answers {
            let result = parser.next_statement().unwrap().unwrap().to_string();
            assert_eq!(result, answer);
        }
    }
}
