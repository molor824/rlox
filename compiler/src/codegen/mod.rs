use crate::{
    ast::expression::Expression,
    interpreter::{
        bytecode::{Bytecode, Load},
        string::{IndexableStr, InternedStr},
    },
    span::SpanOf,
};

#[derive(Default)]
pub struct Codegen {
    bytecodes: Vec<SpanOf<Bytecode>>,
    accumulator: BytecodeBuilder,
}
impl Codegen {
    fn expr_gen(&mut self, expr: &Expression) {
        match expr {
            Expression::Nil(span) => self.accumulator.push_source(SpanOf(*span, Load::Nil)),
            Expression::Boolean(bool) => self.accumulator.push_source(bool.map(Load::Bool)),
            Expression::Number(n) => self
                .accumulator
                .push_source(n.as_ref().map(|n| Load::Number(n.to_f64()))),
            Expression::String(str) => self.accumulator.push_source(
                str.as_ref()
                    .map(|s| Load::String(InternedStr::from(IndexableStr::from(s)))),
            ),
            Expression::Array(arr) => self
                .accumulator
                .push_source(arr.as_ref().map(|a| Load::Array(a.len()))),
            _ => todo!(),
        }
    }
}

#[derive(Default, Clone)]
enum BytecodeBuilder {
    #[default]
    Empty,
    UnarySource(SpanOf<Load>),
    BinarySource(SpanOf<Load>, SpanOf<Load>),
}
impl BytecodeBuilder {
    fn push_source(&mut self, method: SpanOf<Load>) {
        *self = match self {
            Self::Empty => Self::UnarySource(method),
            Self::UnarySource(src0) => Self::BinarySource(src0.clone(), method),
            Self::BinarySource(..) => panic!("Builder already has 2 source!"),
        };
    }
}
