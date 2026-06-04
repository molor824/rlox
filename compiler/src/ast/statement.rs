use crate::ast::*;

#[derive(Debug)]
pub enum Statement {
    Expression(Expression),
    If {
        condition: Expression,
        met_block: Vec<Statement>,
        else_block: Vec<Statement>,
    },
}

impl fmt::Display for Statement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn print_indent(statements: &[Statement], f: &mut fmt::Formatter<'_>) -> fmt::Result {
            for statement in statements {
                writeln!(f, ". {}", statement.to_string().replace("\n", "\n. "))?;
            }
            Ok(())
        }

        match self {
            Self::Expression(expr) => write!(f, "{expr};"),
            Self::If { condition, met_block, else_block } => {
                writeln!(f, "if ({condition}) then")?;
                print_indent(met_block, f)?;
                writeln!(f, "else")?;
                print_indent(else_block, f)?;
                write!(f, "end")
            }
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
                break;
            }
        }

        Ok((statements, self.next_terminators()?))
    }

    fn next_if_statement(&mut self) -> Result<Option<Statement>> {
        let Some(if_keyword) = self.next_keyword("if", true)? else {
            return Ok(None);
        };
        let Some(condition) = self.next_expression(true)? else {
            return Err(self.error(if_keyword.0, ErrorKind::ExpectedExpr));
        };
        let Some(then_keyword) = self.next_keyword("then", true)? else {
            return Err(self.error(condition.span(), ErrorKind::ExpectedKeyword("then")));
        };
        let (met_block, Some(terminator)) = self.next_block()? else {
            return Err(self.error(if_keyword.0.concat(then_keyword.0), ErrorKind::ExpectedElse));
        };

        let keyword = terminator.get_str();

        match &*keyword {
            "end" => Ok(Some(Statement::If {
                condition,
                met_block,
                else_block: vec![],
            })),
            "else" => {
                drop(keyword); // self.next_if_statement will borrow the RefCell
                if let Some(elif) = self.next_if_statement()? {
                    Ok(Some(Statement::If {
                        condition,
                        met_block,
                        else_block: vec![elif],
                    }))
                } else {
                    let (else_block, Some(else_terminator)) = self.next_block()? else {
                        return Err(self.error(terminator.0, ErrorKind::ExpectedEnd));
                    };
                    if &*else_terminator.get_str() != "end" {
                        return Err(self.error(else_terminator.0, ErrorKind::ExpectedEnd));
                    }
                    Ok(Some(Statement::If {
                        condition,
                        met_block,
                        else_block,
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
        if let Some(stmt) = self.next_if_statement()? {
            Ok(Some(stmt))
        } else {
            self.next_expr_statement()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_if() {
        let question = r#"
        if true then
            print("Hello, world!");
            print("Semicolon is unnecessary, although it is optional!")

            ;;;;; # Some weird fuck decided to spam semicolons, but it's technically valid code anyways.

        else if false then print("Inlining!") end
        else print("Semicolon is necessary"); print("In this case!") end
        "#;
        let answer = r#"if (true) then
. (("Hello, world!") print);
. (("Semicolon is unnecessary, although it is optional!") print);
else
. if (false) then
. . (("Inlining!") print);
. else
. end
end"#;
        
        let mut parser = Parser::new(question.as_bytes());
        let result = parser.next_statement().unwrap().unwrap().to_string();
        assert_eq!(result, answer);
    }
}
