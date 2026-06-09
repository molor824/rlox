use crate::ast::{assignment::Assignee, *};

impl<R: BufRead> Parser<R> {
    // Is used for recursive expressions
    // NOTE: Update when the top most expression implementation changes
    pub fn next_expression(&mut self, skip_newline: bool) -> Result<Option<Expression>> {
        self.next_assignment(skip_newline)
    }
}

#[derive(Debug)]
pub enum Expression {
    Ident(SourceSpan),
    String(SpanOf<String>),
    Number(SpanOf<Number>),
    Array(SpanOf<Vec<Element>>),
    Boolean(SpanOf<bool>),
    Postfix {
        operator: PostfixOperator,
        operand: Box<Expression>,
    },
    Prefix {
        operator: PrefixOperator,
        operand: Box<Expression>,
    },
    Binary {
        left_operand: Box<Expression>,
        operator: SpanOf<&'static str>,
        right_operand: Box<Expression>,
    },
    Assign {
        assignee: Assignee,
        assigner: Box<Expression>,
    },
}
impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ident(ident) => write!(f, "{}", ident),
            Self::Number(number) => write!(f, "{}", number.1),
            Self::String(string) => write!(f, "{:?}", string.1),
            Self::Boolean(boolean) => write!(f, "{}", boolean.1),
            Self::Array(arr) => write!(
                f,
                "[{}]",
                arr.1
                    .iter()
                    .map(|expr| expr.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            Self::Postfix { operand, operator } => write!(f, "({} {})", operator, operand),
            Self::Prefix { operator, operand } => write!(f, "({} {})", operator, operand),
            Self::Binary {
                left_operand,
                operator,
                right_operand,
            } => write!(f, "({} {} {})", operator.1, left_operand, right_operand),
            Self::Assign { assignee, assigner } => write!(f, "(= {assignee} {assigner})"),
        }
    }
}
impl GetSpan for Expression {
    fn span(&self) -> Span {
        match self {
            Self::Ident(ident) => ident.0,
            Self::Number(number) => number.0,
            Self::String(string) => string.0,
            Self::Boolean(boolean) => boolean.0,
            Self::Array(array) => array.0,
            Self::Postfix { operator, operand } => operator.span().concat(operand.span()),
            Self::Prefix { operator, operand } => operator.span().concat(operand.span()),
            Self::Binary {
                left_operand,
                operator,
                right_operand,
            } => left_operand
                .span()
                .concat(right_operand.span())
                .concat(operator.0),
            Self::Assign { assignee, assigner } => assignee.span().concat(assigner.span()),
        }
    }
}
