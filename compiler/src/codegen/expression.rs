use std::rc::Rc;

use crate::{
    ast::expression::{Element, Expression, Pair},
    codegen::Codegen,
    error::Result,
    interpreter::{bytecode::Bytecode, string::ValueStr},
    span::{GetSpan, SpanOf},
};

impl Codegen {
    pub(crate) fn gen_array(&mut self, arr: &SpanOf<Vec<Element>>) -> Result<()> {
        let base = self.stack_size().expect("Stack unpredictable.");
        for elem in arr.1.iter() {
            match elem {
                Element::Regular(expr) => {
                    self.gen_expr(expr)?;
                }
                Element::Unpack(unpack) => {
                    self.gen_expr(&unpack.1)?;
                    self.push_bytecode(SpanOf(unpack.0, Bytecode::UnpackIter));
                }
            }
        }
        self.push_bytecode(SpanOf(arr.0, Bytecode::StackToArray(base)));
        Ok(())
    }
    fn gen_object(&mut self, obj: &SpanOf<Vec<Pair>>) -> Result<()> {
        self.push_bytecode(SpanOf(obj.0, Bytecode::LoadObj(obj.1.len())));
        self.push_bytecode(SpanOf(obj.0, Bytecode::Dup(obj.1.len() + 1)));
        for pair in obj.1.iter() {
            match pair {
                Pair::Ident(key, value) => {
                    self.gen_expr(value)?;
                    let prop = ValueStr::interned(&key.get_str());
                    self.push_bytecode(SpanOf(
                        key.0.concat(value.span()),
                        Bytecode::StoreProperty(prop.clone()),
                    ));
                }
                Pair::Index(key, value) => {
                    self.gen_expr(&key.1)?;
                    self.gen_expr(value)?;
                    self.push_bytecode(SpanOf(
                        key.0.concat(value.span()),
                        Bytecode::StorePropertyIndirect,
                    ));
                }
                Pair::Unpack(unpack) => {
                    self.gen_expr(&unpack.1)?;
                    self.push_bytecode(SpanOf(unpack.0, Bytecode::MergeObj));
                }
            }
        }
        Ok(())
    }
    pub fn gen_expr(&mut self, expr: &Expression) -> Result<()> {
        match expr {
            Expression::Nil(span) => self.push_bytecode(SpanOf(*span, Bytecode::LoadNil)),
            Expression::Boolean(bool) => {
                self.push_bytecode(SpanOf(bool.0, Bytecode::LoadBool(bool.1)))
            }
            Expression::Number(n) => {
                self.push_bytecode(SpanOf(n.0, Bytecode::LoadNum(n.1.to_f64())))
            }
            Expression::String(str) => {
                self.push_bytecode(SpanOf(str.0, Bytecode::LoadStr(ValueStr::interned(&str.1))))
            }
            Expression::Closure(closure) => {
                let sig = self.create_func_sig(closure)?;
                self.push_bytecode(SpanOf(closure.span(), Bytecode::LoadFn(Rc::new(sig))));
            }
            Expression::Array(arr) => self.gen_array(arr)?,
            Expression::Object(obj) => self.gen_object(obj)?,
            Expression::Ident(ident) => {
                let name = ValueStr::interned(&ident.get_str());
                let bytecode = if let Some(id) = self.get_local_var(name.clone()) {
                    Bytecode::LoadLocal(id)
                } else if let Some(id) = self.get_upvalue(name.clone()) {
                    Bytecode::LoadUpvalue(id)
                } else {
                    Bytecode::LoadGlobal(name)
                };
                self.push_bytecode(SpanOf(ident.0, bytecode));
            }
            Expression::Postfix { operator, operand } => self.gen_postfix(operand, operator)?,
            Expression::Prefix { operator, operand } => self.gen_prefix(operand, operator)?,
            Expression::Binary {
                left_operand,
                operator,
                right_operand,
            } => self.gen_binary(left_operand, right_operand, operator)?,
            Expression::Assign { assignee, assigner } => self.gen_assign(assignee, assigner)?,
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{ast::Parser, codegen::Codegen, interpreter::bytecode::Bytecode};

    #[test]
    fn expr_codegen_test() {
        let mut parser = Parser::new("[1, 2, *[nil, true], false]".as_bytes());
        let result = parser.next_expression(false).unwrap().unwrap();
        let expected = [
            Bytecode::LoadNum(1.0),
            Bytecode::LoadNum(2.0),
            Bytecode::LoadNil,
            Bytecode::LoadBool(true),
            Bytecode::StackToArray(2),
            Bytecode::UnpackIter,
            Bytecode::LoadBool(false),
            Bytecode::StackToArray(0),
        ];
        let mut codegen = Codegen::with_source(parser.source());
        codegen.gen_expr(&result).unwrap();
        for (bc, expected) in codegen.bytecodes().into_iter().zip(expected) {
            println!("{:?}", bc.1);
            assert_eq!(format!("{:?}", expected), format!("{:?}", bc.1));
        }
    }
}
