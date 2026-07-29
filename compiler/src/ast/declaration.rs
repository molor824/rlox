use crate::ast::expression::Expression;

use crate::ast::statement::{print_indent, Statement};
use crate::{
    ast::{expression::*, *},
    span::{GetSpan, SpanOf},
};

#[derive(Debug)]
pub enum Declaration {
    VarDecl(VarDecl),
    FuncDecl(FuncDecl),
    Expression(Expression),
}
impl fmt::Display for Declaration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VarDecl(decl) => write!(f, "{}", decl),
            Self::FuncDecl(decl) => write!(f, "{}", decl),
            Self::Expression(expr) => write!(f, "{}", expr),
        }
    }
}
impl GetSpan for Declaration {
    fn span(&self) -> Span {
        match self {
            Self::VarDecl(decl) => decl.span(),
            Self::FuncDecl(decl) => decl.span(),
            Self::Expression(expr) => expr.span(),
        }
    }
}

#[derive(Debug)]
pub struct VarDecl {
    pub keyword: SourceSpan,
    pub ident: SourceSpan,
    pub assigner: Expression,
}
impl GetSpan for VarDecl {
    fn span(&self) -> Span {
        self.keyword.0.concat(self.assigner.span())
    }
}
impl fmt::Display for VarDecl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} = ({})", self.keyword, self.ident, self.assigner)
    }
}
#[derive(Debug)]
pub enum FunctionBody {
    Block(SpanOf<Vec<Statement>>), // span covers `do ... end`
    Expression(Expression),
}
impl fmt::Display for FunctionBody {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Block(block) => {
                writeln!(f, "do")?;
                print_indent(&block.1, f)?;
                write!(f, "end")
            }
            Self::Expression(expr) => write!(f, "{expr}"),
        }
    }
}
impl GetSpan for FunctionBody {
    fn span(&self) -> Span {
        match self {
            Self::Block(block) => block.0,
            Self::Expression(expr) => expr.span(),
        }
    }
}

#[derive(Debug)]
pub struct FuncDecl {
    pub fn_keyword: Span,
    pub ident: SourceSpan,
    pub closure: Closure,
}
impl GetSpan for FuncDecl {
    fn span(&self) -> Span {
        self.fn_keyword.concat(self.closure.span())
    }
}
impl fmt::Display for FuncDecl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "fn {}", self.ident)?;
        write!(f, "(")?;
        for (i, param) in self.closure.params.1.iter().enumerate() {
            if i != 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", param)?;
        }
        if let Some(variadic) = &self.closure.variadic {
            if !self.closure.params.1.is_empty() {
                write!(f, ", ")?;
            }
            write!(f, "*{}", variadic.1)?;
        }
        write!(f, ") {}", self.closure.body)
    }
}

impl<R: BufRead> Parser<R> {
    fn next_func_decl(&mut self, skip_newline: bool) -> Result<Option<Declaration>> {
        let Some(fn_kwd) = self.next_keyword("fn", skip_newline)? else {
            return Ok(None);
        };

        let Some(ident) = self.next_ident(skip_newline)? else {
            return Err(self.error(fn_kwd.0, ErrorKind::ExpectedIdent));
        };

        let Some(paren_start) = self.next_symbol("(", skip_newline)? else {
            return Err(self.error(fn_kwd.0, ErrorKind::ExpectedLeftParen));
        };
        let (params, variadic) = self.next_params(skip_newline)?;

        let Some(paren_end) = self.next_symbol(")", true)? else {
            return Err(self.error(paren_start, ErrorKind::ExpectedRightParen));
        };

        let Some(body) = self.next_body(skip_newline)? else {
            return Err(self.error(fn_kwd.0.concat(paren_end), ErrorKind::ExpectedFuncBody));
        };

        Ok(Some(Declaration::FuncDecl(FuncDecl {
            fn_keyword: fn_kwd.0,
            ident,
            closure: Closure {
                params: SpanOf(paren_start.concat(paren_end), params),
                variadic,
                body: Box::new(body),
            },
        })))
    }
    fn next_var_decl(&mut self, skip_newline: bool) -> Result<Option<Declaration>> {
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

        Ok(Some(Declaration::VarDecl(VarDecl {
            keyword: var_kwd,
            ident,
            assigner,
        })))
    }

    pub(crate) fn next_params(
        &mut self,
        skip_newline: bool,
    ) -> Result<(Vec<SourceSpan>, Option<SpanOf<SourceSpan>>)> {
        let mut params = vec![];
        let mut variadic = None;

        loop {
            let star = self.next_symbol("*", skip_newline)?;
            let Some(ident) = self.next_ident(skip_newline)? else {
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
            if self.next_symbol(",", skip_newline)?.is_none() {
                break;
            }
        }
        Ok((params, variadic))
    }
    pub(crate) fn next_body(&mut self, skip_newline: bool) -> Result<Option<FunctionBody>> {
        if let Some(block) = self.next_do_block(skip_newline)? {
            Ok(Some(FunctionBody::Block(block)))
        } else if let Some(expr) = self.next_expression(skip_newline)? {
            Ok(Some(FunctionBody::Expression(expr)))
        } else {
            Ok(None)
        }
    }
    pub fn next_decl(&mut self, skip_newline: bool) -> Result<Option<Declaration>> {
        if let Some(decl) = self.next_func_decl(skip_newline)? {
            return Ok(Some(decl));
        }
        if let Some(decl) = self.next_var_decl(skip_newline)? {
            return Ok(Some(decl));
        }
        self.next_expression(skip_newline)
            .map(|expr| expr.map(Declaration::Expression))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_declaration() {
        let question = r"
        a = b = c
        a.x = b.y = 2
        a[0] = b[1] = c[2] + d[3] + e[4]
        let a = 3
        fn add(a, b) a + b
        fn sum(*values) do
            let total = 0
            let i = 0
            while i < values:len() do
                total = total + values[i]
                i = i + 1
            end
            return total
        end
        let a = { x: 0, y: 0 }
        a.magnitude = \self -> sqrt(self.x * self.x + self.y * self.y)
        a.magnitude = \self -> do
            let sqr = self.x * self.x + self.y * self.y
            return sqrt(sqr)
        end
        print(a:magnitude())
        ";
        let answers = [
            "(a) = ((b) = (c))",
            "((a).x) = (((b).y) = (2))",
            "((a)[0]) = (((b)[1]) = ((((c)[2]) + ((d)[3])) + ((e)[4])))",
            "let a = (3)",
            "fn add(a, b) (a) + (b)",
            "fn sum(*values) do
. let total = (0)
. let i = (0)
. while (i) < (((values):len)()) do
. . (total) = ((total) + ((values)[i]))
. . (i) = ((i) + (1))
. end
. return total
end",
            "let a = ({x: 0, y: 0})",
            "((a).magnitude) = (\\self -> (sqrt)((((self).x) * ((self).x)) + (((self).y) * ((self).y))))",
            "((a).magnitude) = (\\self -> do\n. let sqr = ((((self).x) * ((self).x)) + (((self).y) * ((self).y)))\n. return (sqrt)(sqr)\nend)",
            "(print)(((a):magnitude)())"
        ];

        let mut parser = Parser::new(question.as_bytes());
        for answer in answers {
            parser.skip_seperator().unwrap();
            let result = parser.next_decl(false).unwrap().unwrap().to_string();
            assert_eq!(result, answer);
        }
    }
}
