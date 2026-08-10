use crate::{
    ast::expression::{Element, Expression, PostfixOperator},
    codegen::Codegen,
    error::Result,
    interpreter::{
        bytecode::{Bytecode, UnaryOp},
        string::ValueStr,
    },
    span::SpanOf,
};

impl Codegen {
    pub(crate) fn gen_postfix(
        &mut self,
        operand: &Expression,
        operator: &PostfixOperator,
    ) -> Result<()> {
        self.gen_expr(operand)?;

        match operator {
            PostfixOperator::Call(args) => {
                let base = self.stack_size().expect("Stack unpredictable.") - 1;

                for arg in args.1.iter() {
                    match arg {
                        Element::Regular(r) => {
                            self.gen_expr(r)?;
                        }
                        Element::Unpack(u) => {
                            self.gen_expr(&u.1)?;
                            self.push_bytecode(SpanOf(u.0, Bytecode::UnpackIter));
                        }
                    }
                }

                self.push_bytecode(SpanOf(args.0, Bytecode::Call(base)));
            }
            PostfixOperator::Index(index) => {
                self.gen_expr(&index.1)?;
                self.push_bytecode(SpanOf(index.0, Bytecode::LoadPropertyIndirect));
            }
            PostfixOperator::Property(prop) => {
                self.push_bytecode(SpanOf(
                    prop.0,
                    Bytecode::LoadProperty(ValueStr::interned(&prop.get_str())),
                ));
            }
            PostfixOperator::Method(prop) => {
                self.push_bytecode(SpanOf(
                    prop.0,
                    Bytecode::LoadMethod(ValueStr::interned(&prop.get_str())),
                ));
            }
        }

        Ok(())
    }
    pub(crate) fn gen_prefix(
        &mut self,
        operand: &Expression,
        operator: &SpanOf<&'static str>,
    ) -> Result<()> {
        self.gen_expr(operand)?;

        let op = match operator.1 {
            "-" => UnaryOp::Negate,
            "~" => UnaryOp::Swap,
            "!" => UnaryOp::SetFalse,
            op => unreachable!("Must not reach {op}"),
        };
        self.push_bytecode(SpanOf(operator.0, Bytecode::Unary(op)));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{ast::Parser, codegen::Codegen};

    #[test]
    fn unary_gen_test() {
        let mut parser = Parser::new("-~!foo[0].test:method(1, 2, 3)(4, 5, *rest)".as_bytes());
        let result = parser.next_expression(false).unwrap().unwrap();
        // #[rustfmt::skip]
        // let expected = [
        //     Bytecode::LoadPropertyIndirect { dst: Store::Local(7), src: Load::Global("foo".into()), prop: Load::Number(0.0) },
        //     Bytecode::LoadProperty { dst: Store::Local(6), src: Load::Local(7), prop: "test".into() },
        //     Bytecode::LoadMethod { dst: Store::Local(5), src: Load::Local(6), prop: "method".into() },
        //     Bytecode::Move { dst: Store::Local(8), src: Load::Number(1.0) },
        //     Bytecode::Move { dst: Store::Local(9), src: Load::Number(2.0) },
        //     Bytecode::Move { dst: Store::Local(10), src: Load::Number(3.0) },
        //     Bytecode::Call { src: Load::Local(5), base: 8, dst: Store::Local(4) },
        //     Bytecode::Move { dst: Store::Local(11), src: Load::Array(3) },
        //     Bytecode::AppendElement { dst: Load::Local(11), src: Load::Number(4.0) },
        //     Bytecode::AppendElement { dst: Load::Local(11), src: Load::Number(5.0) },
        //     Bytecode::AppendElements { dst: Load::Local(11), src: Load::Global("rest".into()) },
        //     Bytecode::CallArray { src: Load::Local(4), args: Load::Local(11), dst: Store::Local(3) },
        //     Bytecode::SetFalse { dst: Store::Local(2), src: Load::Local(3) },
        //     Bytecode::Invert { dst: Store::Local(1), src: Load::Local(2) },
        //     Bytecode::Negate { dst: Store::Local(0), src: Load::Local(1) },
        // ];
        let mut codegen = Codegen::with_source(parser.source());
        codegen.gen_expr(&result).unwrap();
        for bc in codegen.bytecodes().into_iter() {
            println!("{:?}", bc.1);
        }
    }
}
