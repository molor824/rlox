use crate::ast::*;

#[derive(Debug, Clone)]
pub enum Assignee {
    Ident(SourceSpan),
    Array {
        elements: SpanOf<Vec<Assignee>>,
        split: Option<Split>,
    },
}
impl GetSpan for Assignee {
    fn span(&self) -> Span {
        match self {
            Self::Ident(s) => s.0,
            Self::Array { elements, .. } => elements.0,
        }
    }
}
impl fmt::Display for Assignee {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ident(i) => write!(f, "{}", i),
            Self::Array { elements, split } => {
                write!(f, "[")?;
                let mut splitted = false;
                for (i, elem) in elements.1.iter().enumerate() {
                    if let Some(split) = split {
                        if i == split.index.1 {
                            splitted = true;
                            if i != 0 {
                                write!(f, ",")?;
                            }
                            write!(f, "*")?;
                            if let Some(ident) = &split.ident {
                                write!(f, "{}", ident)?;
                            }
                        }
                    }
                    if i != 0 || splitted {
                        write!(f, ",")?;
                    }
                    write!(f, "{}", elem)?;
                }
                if let Some(split) = split {
                    if elements.1.len() == split.index.1 {
                        if elements.1.len() != 0 {
                            write!(f, ",")?;
                        }
                        write!(f, "*")?;
                        if let Some(ident) = &split.ident {
                            write!(f, "{}", ident)?;
                        }
                    }
                }
                write!(f, "]")
            }
        }
    }
}
#[derive(Debug, Clone)]
pub struct Split {
    pub index: SpanOf<usize>,
    pub ident: Option<SourceSpan>,
}
impl GetSpan for Split {
    fn span(&self) -> Span {
        match &self.ident {
            Some(i) => self.index.0.concat(i.0),
            None => self.index.0,
        }
    }
}

impl<R: BufRead> Parser<R> {
    pub fn next_assignment(&mut self, skip_newline: bool) -> Result<Option<Expression>> {
        let mut chain = vec![];

        loop {
            let prev = self.clone();
            let Ok(Some(assignee)) = self.next_assignee(skip_newline) else {
                *self = prev;
                break;
            };
            let Some(equal) = self.next_symbol("=", skip_newline)? else {
                *self = prev;
                break;
            };
            chain.push((assignee, equal));
        }
        let Some(mut expr) = self.next_binary(skip_newline)? else {
            if let Some((_, equal)) = chain.last() {
                return Err(self.error(*equal, ErrorKind::ExpectedExpr));
            } else {
                return Ok(None);
            }
        };
        while let Some((assignee, ..)) = chain.pop() {
            expr = Expression::Assign { assignee, assigner: expr.into() };
        }
        Ok(Some(expr))
    }
    pub fn next_assignee(&mut self, skip_newline: bool) -> Result<Option<Assignee>> {
        if let Some(ident) = self.next_ident(skip_newline)? {
            Ok(Some(Assignee::Ident(ident)))
        } else if let Some(start) = self.next_symbol("[", skip_newline)? {
            let mut elements: Vec<Assignee> = vec![];
            let mut split: Option<Split> = None;
            let mut span = start;
            loop {
                if let Some(assignee) = self.next_assignee(true)? {
                    span = span.concat(assignee.span());
                    elements.push(assignee);
                } else if let Some(unpack) = self.next_symbol("*", true)? {
                    if let Some(split) = split {
                        return Err(self.error(split.span(), ErrorKind::RepeatingSplit));
                    }
                    let ident = self.next_ident(true)?;
                    let split1 = Split {
                        index: SpanOf(unpack, elements.len()),
                        ident,
                    };
                    span = span.concat(split1.span());
                    split = Some(split1);
                } else {
                    break;
                }
                let Some(comma) = self.next_symbol(",", true)? else {
                    break;
                };
                span = span.concat(comma);
            }
            let Some(end) = self.next_symbol("]", true)? else {
                return Err(self.error(span, ErrorKind::ExpectedRightSquare));
            };
            Ok(Some(Assignee::Array {
                elements: SpanOf(span.concat(end), elements),
                split,
            }))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_assignment() {
        let question = "a = [b, c] = [d, *] = [*, e, f] = [g, h, *i, j] = [k, *l] = [1, 2, 3, 4, 5, 6]";
        let answer = "(= a (= [b,c] (= [d,*] (= [*,e,f] (= [g,h,*i,j] (= [k,*l] [1,2,3,4,5,6]))))))";
        let mut parser = Parser::new(question.as_bytes());
        let result = parser.next_expression(false).unwrap().unwrap().to_string();
        assert_eq!(result, answer);
    }
}
