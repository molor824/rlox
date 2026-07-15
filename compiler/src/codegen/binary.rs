use crate::{
    ast::expression::Expression,
    codegen::Codegen,
    error::Result,
    interpreter::{
        bytecode::{Bytecode, Load, Store},
        LocalId,
    },
    span::{GetSpan, SpanOf},
};

impl Codegen {
    pub(crate) fn gen_binary(
        &mut self,
        left_operand: &Expression,
        right_operand: &Expression,
        operator: &SpanOf<&'static str>,
        store_method: Option<Store>,
    ) -> Result<Load> {
        let mut max_len = self.locals.len();
        let store_method = store_method.unwrap_or_else(|| {
            let id = max_len as LocalId;
            max_len += 1;
            Store::Local(id)
        });
        let left_load = self.gen_expr(left_operand, None)?;
        let span = left_operand.span().concat(right_operand.span());

        match operator.1 {
            "||" | "&&" => {
                let condition_bytecode = self.bytecodes.len();
                self.push_bytecode(SpanOf(span, Bytecode::Nop));
                self.gen_expr(right_operand, Some(store_method.clone()))?;
                self.push_bytecode(SpanOf(span, Bytecode::Jump(2)));
                let offset = (self.bytecodes.len() - condition_bytecode) as isize;
                self.bytecodes[condition_bytecode].1 = match operator.1 {
                    "||" => Bytecode::BrTrue {
                        offset,
                        src: left_load.clone(),
                    },
                    _ => Bytecode::BrFalse {
                        offset,
                        src: left_load.clone(),
                    },
                };
                self.push_bytecode(SpanOf(
                    span,
                    Bytecode::Move {
                        dst: store_method.clone(),
                        src: left_load,
                    },
                ));
            }
            op => {
                let right_load = self.gen_expr(right_operand, None)?;

                #[rustfmt::skip]
                let bytecode = match op {
                    "+" => Bytecode::Add { dst: store_method.clone(), src0: left_load, src1: right_load },
                    "-" => Bytecode::Sub { dst: store_method.clone(), src0: left_load, src1: right_load },
                    "*" => Bytecode::Mul { dst: store_method.clone(), src0: left_load, src1: right_load },
                    "/" => Bytecode::Div { dst: store_method.clone(), src0: left_load, src1: right_load },
                    "%" => Bytecode::Rem { dst: store_method.clone(), src0: left_load, src1: right_load },
                    "**" => Bytecode::Pow { dst: store_method.clone(), src0: left_load, src1: right_load },
                    ">>" => Bytecode::Shr { dst: store_method.clone(), src0: left_load, src1: right_load },
                    "<<" => Bytecode::Shl { dst: store_method.clone(), src0: left_load, src1: right_load },
                    ">>>" => Bytecode::Sha { dst: store_method.clone(), src0: left_load, src1: right_load },
                    ">=" => Bytecode::SetGe { dst: store_method.clone(), src0: left_load, src1: right_load },
                    "<=" => Bytecode::SetLe { dst: store_method.clone(), src0: left_load, src1: right_load },
                    ">" => Bytecode::SetGt { dst: store_method.clone(), src0: left_load, src1: right_load },
                    "<" => Bytecode::SetLt { dst: store_method.clone(), src0: left_load, src1: right_load },
                    "==" => Bytecode::SetEq { dst: store_method.clone(), src0: left_load, src1: right_load },
                    "!=" => Bytecode::SetNe { dst: store_method.clone(), src0: left_load, src1: right_load },
                    "|" => Bytecode::Or { dst: store_method.clone(), src0: left_load, src1: right_load },
                    "&" => Bytecode::And { dst: store_method.clone(), src0: left_load, src1: right_load },
                    "^" => Bytecode::Xor { dst: store_method.clone(), src0: left_load, src1: right_load },
                    op => todo!("{op} is not implemented!"),
                };
                self.push_bytecode(SpanOf(span, bytecode));
            }
        }

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
    fn binary_gen_test() {
        let mut parser = Parser::new("1!=0 + 2 * 0.2 or 3 <= 3 and 3>2".as_bytes());
        let mut codegen = Codegen::default();

        #[rustfmt::skip]
        let expected = [
            Bytecode::Mul { dst: Store::Local(0), src0: Load::Number(2.0), src1: Load::Number(0.2) },
            Bytecode::Add { dst: Store::Local(0), src0: Load::Number(0.0), src1: Load::Local(0) },
            Bytecode::SetNe { dst: Store::Local(0), src0: Load::Number(1.0), src1: Load::Local(0) },
            Bytecode::BrTrue { offset: 7, src: Load::Local(0) },
            Bytecode::SetLe { dst: Store::Local(1), src0: Load::Number(3.0), src1: Load::Number(3.0) },
            Bytecode::BrFalse { offset: 3, src: Load::Local(1) },
            Bytecode::SetGt { dst: Store::Local(0), src0: Load::Number(3.0), src1: Load::Number(2.0) },
            Bytecode::Jump(2),
            Bytecode::Move { dst: Store::Local(0), src: Load::Local(1) },
            Bytecode::Jump(2),
            Bytecode::Move { dst: Store::Local(0), src: Load::Local(0) },
        ];

        codegen
            .gen_expr(&parser.next_expression(false).unwrap().unwrap(), None)
            .unwrap();
        for (bc, expected) in codegen.bytecodes.iter().zip(expected) {
            println!("{:?}", bc.1);
            assert_eq!(bc.1, expected);
        }
    }
}
