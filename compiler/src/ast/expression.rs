use std::cell::Ref;

use num_bigint::BigInt;

use crate::{
    ast::{
        statement::{print_indent, Statement},
        *,
    },
    span::{GetSpan, SpanOf},
};

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
    Object(SpanOf<Vec<Pair>>),
    Boolean(SpanOf<bool>),
    Nil(Span),
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
    FunctionDecl {
        fn_keyword: Span,
        ident: Option<SourceSpan>, // Closure if None
        params: Vec<SourceSpan>,
        variadic: Option<SpanOf<SourceSpan>>, // span covers *ident
        body: FunctionBody,
    },
    VarDecl {
        keyword: SourceSpan,
        ident: SourceSpan,
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
            Self::Nil(_) => write!(f, "nil"),
            Self::Array(arr) => write!(
                f,
                "[{}]",
                arr.1
                    .iter()
                    .map(|elem| elem.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Object(obj) => write!(
                f,
                "{{{}}}",
                obj.1
                    .iter()
                    .map(|pair| pair.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Postfix { operand, operator } => write!(f, "({operand}){operator}"),
            Self::Prefix { operator, operand } => write!(f, "{operator}({operand})"),
            Self::Binary {
                left_operand,
                operator,
                right_operand,
            } => write!(f, "({left_operand}) {} ({right_operand})", operator.1),
            Self::Assign { assignee, assigner } => write!(f, "({assignee}) = ({assigner})"),
            Self::FunctionDecl {
                ident,
                params,
                variadic,
                body,
                ..
            } => {
                write!(f, "fn")?;
                if let Some(ident) = ident {
                    write!(f, " {}", ident)?;
                }
                write!(f, "(")?;
                for (i, param) in params.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", param)?;
                }
                if let Some(variadic) = variadic {
                    if !params.is_empty() {
                        write!(f, ", ")?;
                    }
                    write!(f, "*{}", variadic.1)?;
                }
                write!(f, ") {body}")
            }
            Self::VarDecl {
                ident,
                assigner,
                keyword,
            } => write!(f, "{keyword} {ident} = ({assigner})"),
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
            Self::Nil(span) => *span,
            Self::Array(array) => array.0,
            Self::Object(object) => object.0,
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
            Self::FunctionDecl {
                body, fn_keyword, ..
            } => fn_keyword.concat(body.span()),
            Self::VarDecl {
                keyword, assigner, ..
            } => keyword.0.concat(assigner.span()),
        }
    }
}

#[derive(Debug)]
pub enum FunctionBody {
    Block(SpanOf<Vec<Statement>>),       // span covers `do ... end`
    Expression(SpanOf<Box<Expression>>), // span covers `=> [expr]`
}
impl fmt::Display for FunctionBody {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Block(block) => {
                writeln!(f, "do")?;
                print_indent(&block.1, f)?;
                write!(f, "end")
            }
            Self::Expression(expr) => write!(f, "=> {}", expr.1),
        }
    }
}
impl GetSpan for FunctionBody {
    fn span(&self) -> Span {
        match self {
            Self::Block(block) => block.0,
            Self::Expression(expr) => expr.0,
        }
    }
}

#[derive(Clone)]
pub struct SourceSpan(pub Span, pub Rc<RefCell<String>>);
impl SourceSpan {
    pub fn get_str(&self) -> Ref<'_, str> {
        Ref::map(self.1.borrow(), |r| &r[self.0.start..self.0.end])
    }
}
impl fmt::Display for SourceSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get_str())
    }
}
impl fmt::Debug for SourceSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.get_str())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Integer {
    pub radix: u32,
    pub integer: BigInt,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Number {
    pub radix: u32,
    pub integer: BigInt,
    pub exponent: Option<i64>,
}
impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.radix {
            2 => write!(f, "0b{:b}", self.integer),
            8 => write!(f, "0o{:o}", self.integer),
            10 => write!(f, "{}", self.integer),
            16 => write!(f, "0x{:X}", self.integer),
            _ => unreachable!(),
        }?;
        if let Some(exp) = self.exponent {
            let sign = if exp >= 0 { '+' } else { '-' };
            let exp = exp.abs();
            match self.radix {
                2 => write!(f, "e{sign}{:b}", exp),
                8 => write!(f, "e{sign}{:o}", exp),
                10 => write!(f, "e{sign}{}", exp),
                16 => write!(f, "p{sign}{:X}", exp),
                _ => unreachable!(),
            }?;
        }
        Ok(())
    }
}
impl Number {
    pub fn new(radix: u32, mut integer: BigInt, mut exponent: Option<i64>) -> Self {
        if let Some(mut exp) = exponent {
            // Perform zero trimming exponent optimization
            if integer == BigInt::ZERO {
                exp = 0
            } else {
                while &integer % radix == BigInt::ZERO {
                    integer /= radix;
                    exp += 1;
                }
            }
            exponent = Some(exp);
        }
        Self {
            radix,
            integer,
            exponent,
        }
    }
}

#[derive(Debug)]
pub struct PrefixOperator(pub SpanOf<&'static str>);
impl fmt::Display for PrefixOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0 .1)
    }
}
impl GetSpan for PrefixOperator {
    fn span(&self) -> Span {
        self.0 .0
    }
}
#[derive(Debug)]
pub enum PostfixOperator {
    Property(SourceSpan),
    Method(SourceSpan),
    Call(SpanOf<Vec<Element>>),
    Index(SpanOf<Box<Expression>>),
}
impl fmt::Display for PostfixOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Property(property) => write!(f, ".{property}"),
            Self::Call(args) => write!(
                f,
                "({})",
                args.1
                    .iter()
                    .map(|arg| arg.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Method(method) => write!(f, ":{method}"),
            Self::Index(args) => write!(f, "[{}]", args.1),
        }
    }
}
impl GetSpan for PostfixOperator {
    fn span(&self) -> Span {
        match self {
            Self::Property(p) => p.0,
            Self::Call(c) => c.0,
            Self::Index(i) => i.0,
            Self::Method(m) => m.0,
        }
    }
}

#[derive(Debug)]
pub enum Assignee {
    Ident(SourceSpan),
    Property {
        ident: SourceSpan,
        operand: Box<Expression>,
    },
    Index {
        arg: SpanOf<Box<Expression>>,
        operand: Box<Expression>,
    },
}
impl fmt::Display for Assignee {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ident(ident) => write!(f, "{ident}"),
            Self::Property { ident, operand } => write!(f, "({operand}).{ident}"),
            Self::Index { arg, operand } => write!(f, "({operand})[{}]", arg.1),
        }
    }
}
impl GetSpan for Assignee {
    fn span(&self) -> Span {
        match self {
            Self::Ident(ident) => ident.0,
            Self::Property { ident, operand } => ident.0.concat(operand.span()),
            Self::Index { arg, operand } => arg.0.concat(operand.span()),
        }
    }
}

#[derive(Debug)]
pub enum Element {
    Regular(Expression),
    Unpack(SpanOf<Expression>),
}
impl fmt::Display for Element {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Regular(expr) => write!(f, "{}", expr),
            Self::Unpack(unpacking) => write!(f, "*{}", unpacking.1),
        }
    }
}

#[derive(Debug)]
pub enum Pair {
    Ident(SourceSpan, Expression),
    Index(SpanOf<Expression>, Expression),
    Unpack(SpanOf<Expression>),
}
impl fmt::Display for Pair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ident(ident, expr) => write!(f, "{ident}: {expr}"),
            Self::Index(key, value) => write!(f, "[{}]: {}", key.1, value),
            Self::Unpack(expr) => write!(f, "*{}", expr.1),
        }
    }
}
