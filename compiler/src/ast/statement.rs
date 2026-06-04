use crate::ast::*;

#[derive(Debug)]
pub enum Statement {
    Expression(Box<Expression>),
    If {
        condition: Box<Expression>,
        met_block: Vec<Statement>,
        else_block: Vec<Statement>,
    },
    While {
        condition: Box<Expression>,
        block: Vec<Statement>,
    },
    For {
        initial: Option<Box<Expression>>,
        condition: Option<Box<Expression>>,
        repeat: Option<Box<Expression>>,
        block: Vec<Statement>,
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
            Self::If {
                condition,
                met_block,
                else_block,
            } => {
                writeln!(f, "if {condition} then")?;
                print_indent(met_block, f)?;
                writeln!(f, "else")?;
                print_indent(else_block, f)?;
                write!(f, "end")
            }
            Self::While { condition, block } => {
                writeln!(f, "while {condition} do")?;
                print_indent(block, f)?;
                write!(f, "end")
            }
            Self::For {
                initial,
                condition,
                repeat,
                block,
            } => {
                write!(f, "for ")?;
                if let Some(i) = initial {
                    write!(f, "{i}")?;
                }
                if let Some(c) = condition {
                    write!(f, "where {c}")?;
                }
                if let Some(r) = repeat {
                    write!(f, "repeat {r}")?;
                }
                writeln!(f, " do")?;
                print_indent(block, f)?;
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

    fn next_do_block(&mut self) -> Result<Option<Vec<Statement>>> {
        let Some(do_keyword) = self.next_keyword("do", true)? else {
            return Ok(None);
        };
        let (statements, Some(terminator)) = self.next_block()? else {
            return Err(self.error(do_keyword.0, ErrorKind::ExpectedEnd));
        };
        if &*terminator.get_str() != "end" {
            return Err(self.error(terminator.0, ErrorKind::ExpectedEnd));
        }
        Ok(Some(statements))
    }

    fn next_for_statement(&mut self) -> Result<Option<Statement>> {
        let Some(for_keyword) = self.next_keyword("for", true)? else {
            return Ok(None);
        };
        let initial = self.next_expression(true)?;
        let condition = match self.next_keyword("where", true)? {
            Some(kwd) => {
                let Some(expr) = self.next_expression(true)? else {
                    return Err(self.error(kwd.0, ErrorKind::ExpectedExpr));
                };
                Some(expr)
            }
            None => None,
        };
        let repeat = match self.next_keyword("repeat", true)? {
            Some(kwd) => {
                let Some(expr) = self.next_expression(true)? else {
                    return Err(self.error(kwd.0, ErrorKind::ExpectedExpr));
                };
                Some(expr)
            }
            None => None,
        };
        let Some(block) = self.next_do_block()? else {
            let mut span = for_keyword.0;
            if let Some(expr) = initial {
                span = span.concat(expr.span());
            }
            if let Some(expr) = condition {
                span = span.concat(expr.span());
            }
            if let Some(expr) = repeat {
                span = span.concat(expr.span());
            }
            return Err(self.error(span, ErrorKind::ExpectedDoBlock));
        };
        Ok(Some(Statement::For {
            initial: initial.map(Box::new),
            condition: condition.map(Box::new),
            repeat: repeat.map(Box::new),
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
        let Some(block) = self.next_do_block()? else {
            return Err(self.error(condition.span(), ErrorKind::ExpectedDoBlock));
        };
        Ok(Some(Statement::While {
            condition: condition.into(),
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
            return Err(self.error(condition.span(), ErrorKind::ExpectedKeyword("then")));
        };
        let (met_block, Some(terminator)) = self.next_block()? else {
            return Err(self.error(if_keyword.0.concat(then_keyword.0), ErrorKind::ExpectedElse));
        };

        let keyword = terminator.get_str();

        match &*keyword {
            "end" => Ok(Some(Statement::If {
                condition: condition.into(),
                met_block,
                else_block: vec![],
            })),
            "else" => {
                drop(keyword); // self.next_if_statement will borrow the RefCell
                if let Some(elif) = self.next_if_statement()? {
                    Ok(Some(Statement::If {
                        condition: condition.into(),
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
                        condition: condition.into(),
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
            .map(|expr| expr.map(|expr| Statement::Expression(expr.into())))
    }

    pub fn next_statement(&mut self) -> Result<Option<Statement>> {
        if let Some(stmt) = self.next_if_statement()? {
            Ok(Some(stmt))
        } else if let Some(stmt) = self.next_while_statement()? {
            Ok(Some(stmt))
        } else if let Some(stmt) = self.next_for_statement()? {
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
        let answer = r#"if true then
. (("Hello, world!") print);
. (("Semicolon is unnecessary, although it is optional!") print);
else
. if false then
. . (("Inlining!") print);
. else
. end
end"#;

        let mut parser = Parser::new(question.as_bytes());
        let result = parser.next_statement().unwrap().unwrap().to_string();
        assert_eq!(result, answer);
    }
}
