use crate::{
    ast::expression::{Assignee, Expression},
    codegen::Codegen,
    error::Result,
    interpreter::bytecode::{Bytecode, Load, Store},
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
        let store_method = store_method.unwrap_or_else(|| Store::Local(self.gen_eval_id()));
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

        Ok(store_method.to_load())
    }
    pub(crate) fn gen_assign(
        &mut self,
        assignee: &Assignee,
        assigner: &Expression,
        store_method: Option<Store>,
    ) -> Result<Load> {
        // Special optimization:
        // If assignee is ident and store method is nil, its a single assignment whose store is only singly determined ident
        if let Assignee::Ident(ident) = assignee {
            if matches!(store_method, Some(Store::Nil)) {
                let ident_store = self.store_ident((&ident.get_str() as &str).into());
                self.gen_expr(assigner, Some(ident_store))?;
                return Ok(Load::Nil);
            }
        }

        let span = assignee.span().concat(assigner.span());
        let load_assigner = self.gen_expr(assigner, None)?; // Load temporarily for storing onto both assignee store and requested store
        match assignee {
            Assignee::Ident(ident) => {
                let ident_store = self.store_ident((&ident.get_str() as &str).into());
                self.push_bytecode(SpanOf(
                    span,
                    Bytecode::Move {
                        dst: ident_store,
                        src: load_assigner.clone(),
                    },
                ));
            }
            Assignee::Index { arg, operand } => {
                let load_operand = self.gen_expr(operand, None)?;
                let load_arg = self.gen_expr(&arg.1, None)?;
                self.push_bytecode(SpanOf(
                    span,
                    Bytecode::StorePropertyIndirect {
                        dst: load_operand,
                        src: load_assigner.clone(),
                        prop: load_arg,
                    },
                ));
            }
            Assignee::Property { ident, operand } => {
                let load_operand = self.gen_expr(operand, None)?;
                self.push_bytecode(SpanOf(
                    span,
                    Bytecode::StoreProperty {
                        dst: load_operand,
                        src: load_assigner.clone(),
                        prop: (&ident.get_str() as &str).into(),
                    },
                ));
            }
        }
        if let Some(store) = store_method {
            self.push_bytecode(SpanOf(
                span,
                Bytecode::Move {
                    dst: store.clone(),
                    src: load_assigner,
                },
            ));
            Ok(store.to_load())
        } else {
            Ok(load_assigner)
        }
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
    fn assign_gen_test() {
        let mut parser = Parser::new("a=b=c.d=e.f[0]=1+2".as_bytes());
        let mut codegen = Codegen::default();
        codegen
            .gen_expr(
                &parser.next_expression(false).unwrap().unwrap(),
                Some(Store::Nil),
            )
            .unwrap();

        #[rustfmt::skip]
        let expected = [
            Bytecode::Add { dst: Store::Local(0), src0: Load::Number(1.0), src1: Load::Number(2.0) },
            Bytecode::LoadProperty { dst: Store::Local(1), src: Load::Global("e".into()), prop: "f".into() },
            Bytecode::StorePropertyIndirect { dst: Load::Local(1), src: Load::Local(0), prop: Load::Number(0.0) },
            Bytecode::StoreProperty { dst: Load::Global("c".into()), src: Load::Local(0), prop: "d".into() },
            Bytecode::Move { dst: Store::Global("b".into()), src: Load::Local(0) },
            Bytecode::Move { dst: Store::Global("a".into()), src: Load::Local(0) },
        ];
        for (bc, expected) in codegen.bytecodes.into_iter().zip(expected) {
            println!("{:?}", bc.1);
            assert_eq!(bc.1, expected);
        }
    }

    #[test]
    fn binary_gen_test() {
        let mut parser = Parser::new("1!=0 + 2 * 0.2 or 3 <= 3 and 3>2".as_bytes());
        let mut codegen = Codegen::default();

        #[rustfmt::skip]
        let expected = [
            Bytecode::Mul { dst: Store::Local(3), src0: Load::Number(2.0), src1: Load::Number(0.2) },
            Bytecode::Add { dst: Store::Local(2), src0: Load::Number(0.0), src1: Load::Local(3) },
            Bytecode::SetNe { dst: Store::Local(1), src0: Load::Number(1.0), src1: Load::Local(2) },
            Bytecode::BrTrue { offset: 7, src: Load::Local(1) },
            Bytecode::SetLe { dst: Store::Local(4), src0: Load::Number(3.0), src1: Load::Number(3.0) },
            Bytecode::BrFalse { offset: 3, src: Load::Local(4) },
            Bytecode::SetGt { dst: Store::Local(0), src0: Load::Number(3.0), src1: Load::Number(2.0) },
            Bytecode::Jump(2),
            Bytecode::Move { dst: Store::Local(0), src: Load::Local(4) },
            Bytecode::Jump(2),
            Bytecode::Move { dst: Store::Local(0), src: Load::Local(1) },
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
