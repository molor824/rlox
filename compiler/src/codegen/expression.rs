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
        let base = self.stack_size();
        let mut dynamic = false; // true when array initialization becomes no longer compile-time predictable
        for elem in arr.1.iter() {
            match elem {
                Element::Regular(expr) => {
                    self.gen_expr(expr)?;
                    if dynamic {
                        self.push_bytecode(SpanOf(expr.span(), Bytecode::AppendArray));
                    }
                }
                Element::Unpack(unpack) => {
                    if !dynamic {
                        dynamic = true;
                        self.push_bytecode(SpanOf(unpack.0, Bytecode::StackToArray(base)));
                    }
                    self.gen_expr(&unpack.1)?;
                    self.push_bytecode(SpanOf(unpack.0, Bytecode::ExtendArray));
                }
            }
        }
        if !dynamic {
            self.push_bytecode(SpanOf(arr.0, Bytecode::StackToArray(base)));
        }
        Ok(())
    }
    fn gen_object(&mut self, obj: &SpanOf<Vec<Pair>>) -> Result<()> {
        let base = self.stack_size();
        let mut dynamic = false;
        for pair in obj.1.iter() {
            match pair {
                Pair::Ident(key, value) => {
                    if !dynamic {
                        self.push_bytecode(SpanOf(
                            key.0,
                            Bytecode::LoadStr(ValueStr::interned(&key.get_str())),
                        ));
                    } // only beneficial on non-dynamic initialization
                    self.gen_expr(value)?;
                    if dynamic {
                        self.push_bytecode(SpanOf(
                            key.0.concat(value.span()),
                            Bytecode::AppendObj(ValueStr::interned(&key.get_str())),
                        ));
                    }
                }
                Pair::Index(key, value) => {
                    self.gen_expr(&key.1)?;
                    self.gen_expr(value)?;
                    if dynamic {
                        self.push_bytecode(SpanOf(
                            key.0.concat(value.span()),
                            Bytecode::AppendObjIndirect,
                        ));
                    }
                }
                Pair::Unpack(unpack) => {
                    if !dynamic {
                        dynamic = true;
                        self.push_bytecode(SpanOf(unpack.0, Bytecode::StackToObj(base)));
                    }
                    self.gen_expr(&unpack.1)?;
                    self.push_bytecode(SpanOf(unpack.0, Bytecode::ExtendObj));
                }
            }
        }
        if !dynamic {
            self.push_bytecode(SpanOf(obj.0, Bytecode::StackToObj(base)));
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
    use std::iter::repeat;

    use crate::{ast::Parser, codegen::Codegen};

    #[test]
    fn expr_codegen_test() {
        let mut parser =
            Parser::new("[1, 2, *[nil, true], false] {a: 0, b: 1, *c, [d]: 3}".as_bytes());
        let arr_result = parser.next_expression(false).unwrap().unwrap();
        let obj_result = parser.next_expression(false).unwrap().unwrap();
        let expected = r#"LoadNum(1.0)
        LoadNum(2.0)
        StackToArray(0)
        LoadNil
        LoadBool(true)
        StackToArray(1)
        ExtendArray
        LoadBool(false)
        AppendArray
        LoadStr("a")
        LoadNum(0.0)
        LoadStr("b")
        LoadNum(1.0)
        StackToObj(1)
        LoadGlobal("c")
        ExtendObj
        LoadGlobal("d")
        LoadNum(3.0)
        AppendObjIndirect"#
            .split('\n')
            .map(str::trim)
            .chain(repeat(""));
        let mut codegen = Codegen::with_source(parser.source());
        codegen.gen_expr(&arr_result).unwrap();
        codegen.gen_expr(&obj_result).unwrap();
        for (bc, expected) in codegen.bytecodes().into_iter().zip(expected) {
            println!("{:?}", bc.1);
            assert_eq!(expected, format!("{:?}", bc.1));
        }
    }
}
