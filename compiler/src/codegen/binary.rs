use crate::{
    ast::expression::{Assignee, Expression},
    codegen::Codegen,
    error::Result,
    interpreter::{
        bytecode::{BinaryOp, Bytecode},
        string::ValueStr,
    },
    span::{GetSpan, SpanOf},
};

impl Codegen {
    pub(crate) fn gen_binary(
        &mut self,
        left_operand: &Expression,
        right_operand: &Expression,
        operator: &SpanOf<&'static str>,
    ) -> Result<()> {
        match operator.1 {
            "||" | "&&" => {
                self.gen_expr(left_operand)?;
                self.push_bytecode(SpanOf(left_operand.span(), Bytecode::Dup(2)));

                let condition_bytecode = self.bytecodes().len();
                self.push_bytecode(SpanOf(operator.0, Bytecode::BranchIf(false, 0)));

                self.push_bytecode(SpanOf(operator.0, Bytecode::Dup(0)));
                self.gen_expr(right_operand)?;

                let offset = (self.bytecodes().len() - condition_bytecode) as isize;

                self.bytecodes_mut()[condition_bytecode].1 =
                    Bytecode::BranchIf(operator.1 == "||", offset);
            }
            op_str => {
                self.gen_expr(left_operand)?;
                self.gen_expr(right_operand)?;

                #[rustfmt::skip]
                let op = match op_str {
                    "+" => BinaryOp::Add,
                    "-" => BinaryOp::Sub,
                    "*" => BinaryOp::Mul,
                    "/" => BinaryOp::Div,
                    "%" => BinaryOp::Rem,
                    "**" => BinaryOp::Pow,
                    ">>" => BinaryOp::Shr,
                    "<<" => BinaryOp::Shl,
                    ">>>" => BinaryOp::Sha,
                    ">=" => BinaryOp::SetGe,
                    "<=" => BinaryOp::SetLe,
                    ">" => BinaryOp::SetGt,
                    "<" => BinaryOp::SetLt,
                    "==" => BinaryOp::SetEq,
                    "!=" => BinaryOp::SetNe,
                    "|" => BinaryOp::BitOr,
                    "&" => BinaryOp::BitAnd,
                    "^" => BinaryOp::BitXor,
                    op => todo!("{op} is not implemented!"),
                };
                self.push_bytecode(SpanOf(operator.0, Bytecode::Binary(op)));
            }
        }

        Ok(())
    }
    pub(crate) fn gen_assign(&mut self, assignee: &Assignee, assigner: &Expression) -> Result<()> {
        self.gen_expr(assigner)?;
        self.push_bytecode(SpanOf(assigner.span(), Bytecode::Dup(2)));

        match assignee {
            Assignee::Ident(ident) => {
                let name = ValueStr::interned(&ident.get_str());
                let bytecode = if let Some(id) = self.get_local_var(name.clone()) {
                    Bytecode::StoreLocal(id)
                } else if let Some(id) = self.get_upvalue(name.clone()) {
                    Bytecode::StoreUpvalue(id)
                } else {
                    Bytecode::StoreGlobal(name)
                };
                self.push_bytecode(SpanOf(ident.0, bytecode));
            }
            Assignee::Index { arg, operand } => {
                self.gen_expr(operand)?;
                self.gen_expr(&arg.1)?;
                self.push_bytecode(SpanOf(assignee.span(), Bytecode::StorePropertyIndirect));
            }
            Assignee::Property { ident, operand } => {
                self.gen_expr(operand)?;
                self.push_bytecode(SpanOf(
                    assignee.span(),
                    Bytecode::StoreProperty(ValueStr::interned(&ident.get_str())),
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        ast::Parser,
        codegen::Codegen,
        interpreter::bytecode::{BinaryOp, Bytecode},
    };

    #[test]
    fn assign_gen_test() {
        let mut parser = Parser::new("a=b=c.d=e.f[0]=1+2".as_bytes());
        let mut codegen = Codegen::with_source(parser.source());

        codegen
            .gen_expr(&parser.next_expression(false).unwrap().unwrap())
            .unwrap();

        let expected = [
            Bytecode::LoadNum(1.0),
            Bytecode::LoadNum(2.0),
            Bytecode::Binary(BinaryOp::Add),
            Bytecode::Dup(2),
            Bytecode::LoadGlobal("e".into()),
            Bytecode::LoadProperty("f".into()),
            Bytecode::LoadNum(0.0),
            Bytecode::StorePropertyIndirect,
            Bytecode::Dup(2),
            Bytecode::LoadGlobal("c".into()),
            Bytecode::StoreProperty("d".into()),
            Bytecode::Dup(2),
            Bytecode::StoreGlobal("b".into()),
            Bytecode::Dup(2),
            Bytecode::StoreGlobal("a".into()),
        ];
        for (bc, expected) in codegen.bytecodes().into_iter().zip(expected) {
            println!("{:?}", bc.1);
            assert_eq!(format!("{:?}", expected), format!("{:?}", bc.1));
        }
    }

    #[test]
    fn binary_gen_test() {
        let mut parser = Parser::new("1!=0 + 2 * 0.2 or 3 <= 3 and 3>2".as_bytes());
        let mut codegen = Codegen::with_source(parser.source());

        let expected = [
            Bytecode::LoadNum(1.0),
            Bytecode::LoadNum(0.0),
            Bytecode::LoadNum(2.0),
            Bytecode::LoadNum(0.2),
            Bytecode::Binary(BinaryOp::Mul),
            Bytecode::Binary(BinaryOp::Add),
            Bytecode::Binary(BinaryOp::SetNe),
            Bytecode::Dup(2),
            Bytecode::BranchIf(true, 11),
            Bytecode::Dup(0),
            Bytecode::LoadNum(3.0),
            Bytecode::LoadNum(3.0),
            Bytecode::Binary(BinaryOp::SetLe),
            Bytecode::Dup(2),
            Bytecode::BranchIf(false, 5),
            Bytecode::Dup(0),
            Bytecode::LoadNum(3.0),
            Bytecode::LoadNum(2.0),
            Bytecode::Binary(BinaryOp::SetGt),
        ];

        codegen
            .gen_expr(&parser.next_expression(false).unwrap().unwrap())
            .unwrap();

        for (bc, expected) in codegen.bytecodes().iter().zip(expected) {
            println!("{:?}", bc.1);
            assert_eq!(format!("{:?}", expected), format!("{:?}", bc.1));
        }
    }
}
