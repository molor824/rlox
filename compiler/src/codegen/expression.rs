use std::rc::Rc;

use crate::{
    ast::expression::{Element, Expression, Pair},
    codegen::Codegen,
    error::Result,
    interpreter::bytecode::{Bytecode, Load, Store},
    span::{GetSpan, SpanOf},
};

impl Codegen {
    pub(crate) fn gen_array(
        &mut self,
        arr: &SpanOf<Vec<Element>>,
        store_method: Option<Store>,
    ) -> Result<Load> {
        let store_method = match store_method {
            Some(s) => s,
            None if arr.1.is_empty() => return Ok(Load::Array(0)),
            None => Store::Local(self.push_eval_id()),
        };
        self.push_bytecode(SpanOf(
            arr.0,
            Bytecode::Move {
                dst: store_method.clone(),
                src: Load::Array(arr.1.len()),
            },
        ));
        let load_method = store_method.to_load();
        for elem in arr.1.iter() {
            match elem {
                Element::Regular(expr) => {
                    let load = self.gen_expr(expr, None)?;
                    self.push_bytecode(SpanOf(
                        expr.span(),
                        Bytecode::AppendElement {
                            dst: load_method.clone(),
                            src: load,
                        },
                    ));
                }
                Element::Unpack(unpack) => {
                    let load = self.gen_expr(&unpack.1, None)?;
                    self.push_bytecode(SpanOf(
                        unpack.0,
                        Bytecode::AppendElements {
                            dst: load_method.clone(),
                            src: load,
                        },
                    ));
                }
            }
        }
        Ok(load_method)
    }
    fn gen_object(&mut self, obj: &SpanOf<Vec<Pair>>, store_method: Option<Store>) -> Result<Load> {
        let store_method = match store_method {
            Some(s) => s,
            None if obj.1.is_empty() => return Ok(Load::Object(0)),
            None => Store::Local(self.push_eval_id()),
        };
        self.push_bytecode(SpanOf(
            obj.0,
            Bytecode::Move {
                dst: store_method.clone(),
                src: Load::Object(obj.1.len()),
            },
        ));
        let load_method = store_method.to_load();
        for pair in obj.1.iter() {
            match pair {
                Pair::Ident(key, value) => {
                    let load = self.gen_expr(value, None)?;
                    let key_str = key.get_str();
                    self.push_bytecode(SpanOf(
                        key.0.concat(value.span()),
                        Bytecode::StoreProperty {
                            dst: load_method.clone(),
                            src: load,
                            prop: (&key_str as &str).into(),
                        },
                    ));
                }
                Pair::Index(key, value) => {
                    let load_key = self.gen_expr(&key.1, None)?;
                    let load_value = self.gen_expr(value, None)?;
                    self.push_bytecode(SpanOf(
                        key.0.concat(value.span()),
                        Bytecode::StorePropertyIndirect {
                            dst: load_method.clone(),
                            src: load_value,
                            prop: load_key,
                        },
                    ));
                }
                Pair::Unpack(unpack) => {
                    let load = self.gen_expr(&unpack.1, None)?;
                    self.push_bytecode(SpanOf(
                        unpack.0,
                        Bytecode::StoreProperties {
                            dst: load_method.clone(),
                            src: load,
                        },
                    ));
                }
            }
        }
        Ok(load_method)
    }
    pub fn gen_expr(&mut self, expr: &Expression, store_method: Option<Store>) -> Result<Load> {
        match expr {
            Expression::Nil(span) => match store_method {
                Some(store_method) => {
                    self.push_bytecode(SpanOf(
                        *span,
                        Bytecode::Move {
                            dst: store_method.clone(),
                            src: Load::Nil,
                        },
                    ));
                    Ok(store_method.to_load())
                }
                None => Ok(Load::Nil),
            },
            Expression::Boolean(bool) => match store_method {
                Some(store_method) => {
                    self.push_bytecode(SpanOf(
                        bool.0,
                        Bytecode::Move {
                            dst: store_method.clone(),
                            src: Load::Bool(bool.1),
                        },
                    ));
                    Ok(store_method.to_load())
                }
                None => Ok(Load::Bool(bool.1)),
            },
            Expression::Number(n) => match store_method {
                Some(store_method) => {
                    self.push_bytecode(SpanOf(
                        n.0,
                        Bytecode::Move {
                            dst: store_method.clone(),
                            src: Load::Number(n.1.to_f64()),
                        },
                    ));
                    Ok(store_method.to_load())
                }
                None => Ok(Load::Number(n.1.to_f64())),
            },
            Expression::String(str) => match store_method {
                Some(store_method) => {
                    self.push_bytecode(SpanOf(
                        str.0,
                        Bytecode::Move {
                            dst: store_method.clone(),
                            src: Load::String(str.1.as_str().into()),
                        },
                    ));
                    Ok(store_method.to_load())
                }
                None => Ok(Load::String(str.1.as_str().into())),
            },
            Expression::Closure(closure) => {
                let sig = self.create_func_sig(closure)?;
                match store_method {
                    Some(store) => {
                        self.push_bytecode(SpanOf(
                            closure.span(),
                            Bytecode::Move {
                                dst: store.clone(),
                                src: Load::Function(Rc::new(sig)),
                            },
                        ));
                        Ok(store.to_load())
                    }
                    None => Ok(Load::Function(Rc::new(sig))),
                }
            }
            Expression::Array(arr) => self.gen_array(arr, store_method),
            Expression::Object(obj) => self.gen_object(obj, store_method),
            Expression::Ident(ident) => {
                let load_method = self.load_ident((ident.get_str().as_ref() as &str).into());
                match store_method {
                    Some(s) => {
                        self.push_bytecode(SpanOf(
                            ident.0,
                            Bytecode::Move {
                                dst: s.clone(),
                                src: load_method,
                            },
                        ));
                        Ok(s.to_load())
                    }
                    None => Ok(load_method),
                }
            }
            Expression::Postfix { operator, operand } => {
                self.gen_postfix(operand, operator, store_method)
            }
            Expression::Prefix { operator, operand } => {
                self.gen_prefix(operand, operator, store_method)
            }
            Expression::Binary {
                left_operand,
                operator,
                right_operand,
            } => self.gen_binary(left_operand, right_operand, operator, store_method),
            Expression::Assign { assignee, assigner } => {
                self.gen_assign(assignee, assigner, store_method)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        ast::Parser,
        codegen::Codegen,
        interpreter::{
            bytecode::{Bytecode, Load, Store},
            string::ValueStr,
        },
    };

    #[test]
    fn expr_codegen_test() {
        let mut parser = Parser::new("[1, 2, *[nil, true], false]".as_bytes());
        let result = parser.next_expression(false).unwrap().unwrap();
        let test_ident = ValueStr::interned("test");
        #[rustfmt::skip]
        let expected = [
            Bytecode::Move { dst: Store::Global(test_ident.clone()), src: Load::Array(4) },
            Bytecode::AppendElement { dst: Load::Global(test_ident.clone()), src: Load::Number(1.0) },
            Bytecode::AppendElement { dst: Load::Global(test_ident.clone()), src: Load::Number(2.0) },
            Bytecode::Move { dst: Store::Local(0), src: Load::Array(2) },
            Bytecode::AppendElement { dst: Load::Local(0), src: Load::Nil },
            Bytecode::AppendElement { dst: Load::Local(0), src: Load::Bool(true) },
            Bytecode::AppendElements { dst: Load::Global(test_ident.clone()), src: Load::Local(0) },
            Bytecode::AppendElement { dst: Load::Global(test_ident.clone()), src: Load::Bool(false) },
        ];
        let mut codegen = Codegen::default();
        codegen
            .gen_expr(&result, Some(Store::Global(test_ident)))
            .unwrap();
        for (bc, expected) in codegen.bytecodes().into_iter().zip(expected) {
            println!("{:?}", bc.1);
            assert_eq!(bc.1, expected);
        }
    }
}
