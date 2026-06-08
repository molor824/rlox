use crate::ast::{
    expression::Expression,
    statement::Statement,
    *,
};

#[derive(Debug)]
pub enum Assigner {
    Declare(Box<Expression>),
    Reassign(Box<Expression>),
    Block(SpanOf<Vec<Statement>>),
}
impl GetSpan for Assigner {
    fn span(&self) -> Span {
        match self {
            Self::Block(block) => block.0,
            Self::Declare(expr) => expr.span(),
            Self::Reassign(expr) => expr.span(),
        }
    }
}

const DECLARE: &str = ":=";
const REASSIGN: &str = "=";

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
            let Some(equal) = self.next_symbols([DECLARE, REASSIGN], skip_newline)? else {
                *self = prev;
                break;
            };
            chain.push((assignee, equal));
        }
        let Some(mut expr) = lower(self)? else {
            if let Some((_, equal)) = chain.last() {
                return Err(self.error(equal.0, ErrorKind::ExpectedExpr));
            } else {
                return Ok(None);
            }
        };
        if let Some(block) = self.next_do_block()? {
            expr = Expression::Assign {
                assignee: Box::new(expr),
                assigner: Assigner::Block(block),
            };
        }
        while let Some((assignee, operator)) = chain.pop() {
            expr = Expression::Assign {
                assignee: Box::new(assignee),
                assigner: match operator.1 {
                    DECLARE => Assigner::Declare(Box::new(expr)),
                    REASSIGN => Assigner::Reassign(Box::new(expr)),
                    _ => unreachable!(),
                },
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
        a := b
        a.x = b(x, y) := 2
        a[0] = b[1] = c[2] + d[3] + e[4]
        b := a(x, y, z) do
            length := x + y + z
        end";
        let answers = [
            "(:= a b)",
            "(= (.x a) (:= ((x,y) b) 2))",
            "(= ([0] a) (= ([1] b) (+ (+ ([2] c) ([3] d)) ([4] e))))",
            "(:= b (= ((x,y,z) a) do\n. (:= length (+ (+ x y) z))\nend))",
        ];

        let mut parser = Parser::new(question.as_bytes());
        for answer in answers {
            parser.skip_seperator().unwrap();
            let result = parser.next_assignment(false).unwrap().unwrap().to_string();
            assert_eq!(result, answer);
        }
    }
}
