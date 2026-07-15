use crate::{
    ast::expression::{Element, Expression, PostfixOperator},
    codegen::Codegen,
    error::Result,
    interpreter::{
        bytecode::{Bytecode, Load, Store},
        string::InternedStr,
        LocalId,
    },
    span::{GetSpan, SpanOf},
};

impl Codegen {
    pub(crate) fn gen_postfix(
        &mut self,
        operand: &Expression,
        operator: &PostfixOperator,
        store_method: Option<Store>,
    ) -> Result<Load> {
        let mut max_len = self.locals.len();
        let load_operand = self.gen_expr(operand, None)?;
        let store_method = store_method.unwrap_or_else(|| {
            let id = max_len as LocalId;
            max_len += 1;
            Store::Local(id)
        });
        let span = operand.span().concat(operator.span());

        match operator {
            PostfixOperator::Call(args) => {
                let regular_args = args
                    .1
                    .iter()
                    .map(|arg| match arg {
                        Element::Regular(arg) => Some(arg),
                        _ => None,
                    })
                    .collect::<Option<Vec<_>>>();
                let callcode = match regular_args {
                    None => {
                        let local = self.push_temp_local();
                        let load_array = self.gen_array(args, Some(Store::Local(local)))?;

                        Bytecode::CallArray {
                            dst: store_method.clone(),
                            src: load_operand,
                            args: load_array,
                        }
                    }
                    Some(args) => {
                        let base = self.locals.len();
                        for arg in args {
                            let local = self.push_temp_local();
                            self.gen_expr(arg, Some(Store::Local(local)))?;
                        }

                        Bytecode::Call {
                            src: load_operand,
                            base: base as LocalId,
                            dst: store_method.clone(),
                        }
                    }
                };
                self.push_bytecode(SpanOf(span, callcode));
            }
            PostfixOperator::Index(index) => {
                let load_index = self.gen_expr(&index.1, None)?;

                self.push_bytecode(SpanOf(
                    span,
                    Bytecode::LoadPropertyIndirect {
                        dst: store_method.clone(),
                        src: load_operand,
                        prop: load_index,
                    },
                ));
            }
            PostfixOperator::Property(prop) => {
                self.push_bytecode(SpanOf(
                    span,
                    Bytecode::LoadProperty {
                        dst: store_method.clone(),
                        src: load_operand,
                        prop: InternedStr::from(prop.get_str().as_ref() as &str),
                    },
                ));
            }
            PostfixOperator::Method(prop) => {
                self.push_bytecode(SpanOf(
                    span,
                    Bytecode::LoadMethod {
                        dst: store_method.clone(),
                        src: load_operand,
                        prop: InternedStr::from(prop.get_str().as_ref() as &str),
                    },
                ));
            }
        }

        self.locals.resize_with(max_len, || None);

        Ok(store_method.to_load())
    }
    pub(crate) fn gen_prefix(
        &mut self,
        operand: &Expression,
        operator: &SpanOf<&'static str>,
        store_method: Option<Store>,
    ) -> Result<Load> {
        let mut max_len = self.locals.len();
        let load_operand = self.gen_expr(operand, None)?;
        let store_method = store_method.unwrap_or_else(|| {
            let id = max_len as LocalId;
            max_len += 1;
            Store::Local(id)
        });
        let span = operand.span().concat(operator.0);
        let bytecode = match operator.1 {
            "-" => Bytecode::Negate {
                dst: store_method.clone(),
                src: load_operand,
            },
            "~" => Bytecode::Invert {
                dst: store_method.clone(),
                src: load_operand,
            },
            "!" => Bytecode::SetFalse {
                dst: store_method.clone(),
                src: load_operand,
            },
            op => unreachable!("Must not reach {op}"),
        };

        self.push_bytecode(SpanOf(span, bytecode));
        self.locals.resize_with(max_len, || None);

        Ok(store_method.to_load())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        ast::Parser,
        codegen::Codegen,
        interpreter::bytecode::{Bytecode, Load, Store},
    };

    #[test]
    fn unary_gen_test() {
        let mut parser = Parser::new("-~!foo[0].test:method(1, 2, 3)(4, 5, *rest)".as_bytes());
        let result = parser.next_expression(false).unwrap().unwrap();
        #[rustfmt::skip]
        let expected = [
            Bytecode::LoadPropertyIndirect { dst: Store::Local(0), src: Load::Global("foo".into()), prop: Load::Number(0.0) },
            Bytecode::LoadProperty { dst: Store::Local(0), src: Load::Local(0), prop: "test".into() },
            Bytecode::LoadMethod { dst: Store::Local(0), src: Load::Local(0), prop: "method".into() },
            Bytecode::Move { dst: Store::Local(1), src: Load::Number(1.0) },
            Bytecode::Move { dst: Store::Local(2), src: Load::Number(2.0) },
            Bytecode::Move { dst: Store::Local(3), src: Load::Number(3.0) },
            Bytecode::Call { src: Load::Local(0), base: 1, dst: Store::Local(0) },
            Bytecode::Move { dst: Store::Local(1), src: Load::Array(3) },
            Bytecode::AppendElement { dst: Load::Local(1), src: Load::Number(4.0) },
            Bytecode::AppendElement { dst: Load::Local(1), src: Load::Number(5.0) },
            Bytecode::AppendElements { dst: Load::Local(1), src: Load::Global("rest".into()) },
            Bytecode::CallArray { src: Load::Local(0), args: Load::Local(1), dst: Store::Local(0) },
            Bytecode::SetFalse { dst: Store::Local(0), src: Load::Local(0) },
            Bytecode::Invert { dst: Store::Local(0), src: Load::Local(0) },
            Bytecode::Negate { dst: Store::Local(0), src: Load::Local(0) },
        ];
        let mut codegen = Codegen::default();
        codegen.gen_expr(&result, None).unwrap();
        for (bc, expected) in codegen.bytecodes.into_iter().zip(expected) {
            println!("{:?}", bc.1);
            assert_eq!(bc.1, expected);
        }
    }
}
