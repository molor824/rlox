use std::cell::Ref;

use num_bigint::BigUint;

use crate::{
    ast::{declaration::FunctionBody, *},
    span::{GetSpan, SpanOf},
};

impl<R: BufRead> Parser<R> {
    // Is used for recursive expressions
    // NOTE: Update when the top most expression implementation changes
    pub fn next_expression(&mut self, skip_newline: bool) -> Result<Option<Expression>> {
        self.next_binary(skip_newline)
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
        operator: SpanOf<&'static str>,
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
    Closure {
        params: SpanOf<Vec<SourceSpan>>,      // Covers \params ->
        variadic: Option<SpanOf<SourceSpan>>, // Covers *a
        body: Box<FunctionBody>,
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
            Self::Prefix { operator, operand } => write!(f, "{}({operand})", operator.1),
            Self::Binary {
                left_operand,
                operator,
                right_operand,
            } => write!(f, "({left_operand}) {} ({right_operand})", operator.1),
            Self::Assign { assignee, assigner } => write!(f, "({assignee}) = ({assigner})"),
            Self::Closure {
                params,
                body,
                variadic,
            } => {
                write!(f, "\\")?;
                let mut first = true;
                for p in params.1.iter() {
                    if !first {
                        write!(f, ", ")?;
                    }
                    first = false;
                    write!(f, "{p}")?;
                }
                if let Some(v) = variadic {
                    if !first {
                        write!(f, ", ")?;
                    }
                    write!(f, "*{}", v.1)?;
                }
                write!(f, " -> {body}")
            }
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
            Self::Prefix { operator, operand } => operator.0.concat(operand.span()),
            Self::Binary {
                left_operand,
                operator,
                right_operand,
            } => left_operand
                .span()
                .concat(right_operand.span())
                .concat(operator.0),
            Self::Assign { assignee, assigner } => assignee.span().concat(assigner.span()),
            Self::Closure { params, body, .. } => params.0.concat(body.span()),
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
    pub integer: BigUint,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Number {
    pub radix: u32,
    pub integer: BigUint,
    pub exponent: Option<i64>,
}
impl Number {
    pub fn to_f64(&self) -> f64 {
        let mut integer = self.integer.clone();
        let mut value = 0.0;
        while integer != BigUint::ZERO {
            value *= self.radix as f64;
            value += (&integer % self.radix)
                .to_u32_digits()
                .get(0)
                .copied()
                .unwrap_or(0) as f64;
            integer /= self.radix;
        }
        let exponent = self.exponent.unwrap_or(0);
        if exponent < 0 {
            for _ in exponent..0 {
                value /= self.radix as f64;
            }
        } else {
            for _ in 0..exponent {
                value /= self.radix as f64;
            }
        }
        value
    }
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
    pub fn new(radix: u32, mut integer: BigUint, mut exponent: Option<i64>) -> Self {
        if let Some(mut exp) = exponent {
            // Perform zero trimming exponent optimization
            if integer == BigUint::ZERO {
                exp = 0
            } else {
                while &integer % radix == BigUint::ZERO {
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
