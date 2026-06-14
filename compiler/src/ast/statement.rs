use crate::ast::{expression::*, *};

#[derive(Debug)]
pub enum Statement {
    Expression(Box<Expression>),
    If {
        condition: SpanOf<Box<Expression>>, // span covers `if <expr>`
        met_block: SpanOf<Vec<Statement>>, // span covers `then ... end` or `then ...` if else block exists
        else_block: Option<SpanOf<Vec<Statement>>>, // span covers `else <block> end`
    },
    While {
        condition: SpanOf<Box<Expression>>, // span covers `while <expr>`
        block: SpanOf<Vec<Statement>>,      // span covers `do ... end`
    },
    For {
        initial: SpanOf<Option<Box<Expression>>>, // span covers `for <expr>` or `for`
        condition: Option<SpanOf<Box<Expression>>>, // span covers `where <expr>`
        repeat: Option<SpanOf<Box<Expression>>>,  // span covers `repeat <expr>`
        block: SpanOf<Vec<Statement>>,            // span covers `do ... end`
    },
}

pub fn print_indent(statements: &[Statement], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for statement in statements {
        writeln!(f, ". {}", statement.to_string().replace("\n", "\n. "))?;
    }
    Ok(())
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
                writeln!(f, "if {} then", condition.1)?;
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
                writeln!(f, "while {} do", condition.1)?;
                print_indent(&block.1, f)?;
                write!(f, "end")
            }
            Self::For {
                initial,
                condition,
                repeat,
                block,
                ..
            } => {
                write!(f, "for ")?;
                if let Some(i) = &initial.1 {
                    write!(f, "{}", i)?;
                }
                if let Some(c) = condition {
                    write!(f, "where {}", c.1)?;
                }
                if let Some(r) = repeat {
                    write!(f, "repeat {}", r.1)?;
                }
                writeln!(f, " do")?;
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
            Self::For { initial, block, .. } => initial.0.concat(block.0),
            Self::While { condition, block } => condition.0.concat(block.0),
            Self::If {
                condition,
                met_block,
                else_block,
            } => match else_block {
                Some(else_block) => condition.0.concat(else_block.0),
                None => condition.0.concat(met_block.0),
            },
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
        let initial = self.next_expression(true)?.map(Box::new);
        let condition = match self.next_keyword("where", true)? {
            Some(kwd) => {
                let Some(expr) = self.next_expression(true)? else {
                    return Err(self.error(kwd.0, ErrorKind::ExpectedExpr));
                };
                Some(SpanOf(kwd.0, Box::new(expr)))
            }
            None => None,
        };
        let repeat = match self.next_keyword("repeat", true)? {
            Some(kwd) => {
                let Some(expr) = self.next_expression(true)? else {
                    return Err(self.error(kwd.0, ErrorKind::ExpectedExpr));
                };
                Some(SpanOf(kwd.0, Box::new(expr)))
            }
            None => None,
        };
        let Some(block) = self.next_do_block(true)? else {
            return Err(self.error(for_keyword.0, ErrorKind::ExpectedDoBlock));
        };
        Ok(Some(Statement::For {
            initial: SpanOf(for_keyword.0, initial),
            condition,
            repeat,
            block,
        }))
    }
    fn next_while_statement(&mut self) -> Result<Option<Statement>> {
        let Some(while_keyword) = self.next_keyword("while", true)? else {
            return Ok(None);
        };
        let Some(condition) = self
            .next_expression(true)?
            .map(|expr| SpanOf(while_keyword.0, Box::new(expr)))
        else {
            return Err(self.error(while_keyword.0, ErrorKind::ExpectedExpr));
        };
        let Some(block) = self.next_do_block(true)? else {
            return Err(self.error(condition.0, ErrorKind::ExpectedDoBlock));
        };
        Ok(Some(Statement::While { condition, block }))
    }
    fn next_if_statement(&mut self) -> Result<Option<Statement>> {
        let Some(if_keyword) = self.next_keyword("if", true)? else {
            return Ok(None);
        };
        let Some(condition) = self
            .next_expression(true)?
            .map(|expr| SpanOf(if_keyword.0, Box::new(expr)))
        else {
            return Err(self.error(if_keyword.0, ErrorKind::ExpectedExpr));
        };
        let Some(then_keyword) = self.next_keyword("then", true)? else {
            return Err(self.error(condition.0, ErrorKind::ExpectedKeyword("then")));
        };
        let (met_block, Some(terminator)) = self.next_block()? else {
            return Err(self.error(if_keyword.0.concat(then_keyword.0), ErrorKind::ExpectedElse));
        };

        let keyword = terminator.get_str();

        match &*keyword {
            "end" => Ok(Some(Statement::If {
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
            .map(|expr| expr.map(|expr| Statement::Expression(Box::new(expr))))
    }
    pub fn next_statement(&mut self) -> Result<Option<Statement>> {
        let order = [
            Self::next_if_statement,
            Self::next_while_statement,
            Self::next_for_statement,
            Self::next_expr_statement,
        ];
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
    fn test_if() {
        let question = r#"
        if true then
            print("Hello, world!");
            print("Semicolon is unnecessary, although it is optional!")

            ;;;;; -- Some weird fuck decided to spam semicolons, but it's technically valid code anyways.

        else if false then print("Inlining!") end
        else print("Semicolon is necessary"); print("In this case!") end
        "#;
        let answer = r#"if true then
. (print)("Hello, world!")
. (print)("Semicolon is unnecessary, although it is optional!")
else
. if false then
. . (print)("Inlining!")
. end
end"#;

        let mut parser = Parser::new(question.as_bytes());
        let result = parser.next_statement().unwrap().unwrap().to_string();
        assert_eq!(result, answer);
    }
}
