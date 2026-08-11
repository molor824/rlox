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
                let base = self.stack_size() - 1;

                let regular_args = args
                    .1
                    .iter()
                    .map(|arg| match arg {
                        Element::Regular(reg) => Some(reg),
                        _ => None,
                    })
                    .collect::<Option<Vec<_>>>();

                if let Some(reg_args) = regular_args {
                    for arg in reg_args {
                        self.gen_expr(arg)?;
                    }
                    self.push_bytecode(SpanOf(args.0, Bytecode::Call(base)));
                } else {
                    self.gen_array(args)?;
                    self.push_bytecode(SpanOf(args.0, Bytecode::CallVariadic));
                }
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
    use crate::{
        ast::Parser,
        codegen::Codegen,
        interpreter::{
            bytecode::{Bytecode, UnaryOp},
            string::ValueStr,
        },
    };

    #[test]
    fn unary_gen_test() {
        let mut parser = Parser::new("-~!foo[0].test:method(1, 2, 3)(4, 5, *rest)".as_bytes());
        let result = parser.next_expression(false).unwrap().unwrap();
        let expected = [
            Bytecode::LoadGlobal(ValueStr::interned("foo")),
            Bytecode::LoadNum(0.0),
            Bytecode::LoadPropertyIndirect,
            Bytecode::LoadProperty(ValueStr::interned("test")),
            Bytecode::LoadMethod(ValueStr::interned("method")),
            Bytecode::LoadNum(1.0),
            Bytecode::LoadNum(2.0),
            Bytecode::LoadNum(3.0),
            Bytecode::Call(0),
            Bytecode::LoadNum(4.0),
            Bytecode::LoadNum(5.0),
            Bytecode::LoadGlobal(ValueStr::interned("rest")),
            Bytecode::Call(0),
            Bytecode::Unary(UnaryOp::SetFalse),
            Bytecode::Unary(UnaryOp::Swap),
            Bytecode::Unary(UnaryOp::Negate),
        ];
        let mut codegen = Codegen::with_source(parser.source());
        codegen.gen_expr(&result).unwrap();
        for (bc, expected) in codegen.bytecodes().into_iter().zip(expected) {
            println!("{:?}", bc.1);
            assert_eq!(format!("{:?}", expected), format!("{:?}", bc.1));
        }
    }
}
