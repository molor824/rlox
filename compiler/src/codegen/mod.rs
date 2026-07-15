use crate::{
    ast::expression::{Element, Expression, Pair},
    error::Result,
    interpreter::{
        bytecode::{Bytecode, Load, Store},
        string::InternedStr,
        LocalId,
    },
    span::{GetSpan, SpanOf},
};

mod unary;

#[derive(Default)]
pub struct Codegen {
    bytecodes: Vec<SpanOf<Bytecode>>,
    locals: Vec<Option<InternedStr>>,
}
impl Codegen {
    fn push_bytecode(&mut self, bytecode: SpanOf<Bytecode>) {
        self.bytecodes.push(bytecode);
    }
    pub(crate) fn push_local(&mut self, name: Option<InternedStr>) -> LocalId {
        let temp = self.locals.len() as LocalId;
        self.locals.push(name);
        temp
    }
    fn push_temp_local(&mut self) -> LocalId {
        self.push_local(None)
    }
    fn get_local(&self, name: InternedStr) -> Option<LocalId> {
        self.locals
            .iter()
            .rposition(|l| l.is_some_and(|s| s == name))
            .map(|idx| idx as LocalId)
    }
    fn gen_array(
        &mut self,
        arr: &SpanOf<Vec<Element>>,
        store_method: Option<Store>,
    ) -> Result<Load> {
        let store_method = match store_method {
            Some(s) => s,
            None if arr.1.is_empty() => return Ok(Load::Array(0)),
            None => Store::Local(self.push_temp_local()),
        };
        let len = self.locals.len();
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
                    self.locals.truncate(len);
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
                    self.locals.truncate(len);
                }
            }
        }
        Ok(load_method)
    }
    fn gen_object(&mut self, obj: &SpanOf<Vec<Pair>>, store_method: Option<Store>) -> Result<Load> {
        let store_method = match store_method {
            Some(s) => s,
            None if obj.1.is_empty() => return Ok(Load::Object(0)),
            None => Store::Local(self.push_temp_local()),
        };
        let len = self.locals.len();
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
                    self.locals.truncate(len);
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
                    self.locals.truncate(len);
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
                    self.locals.truncate(len);
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
            Expression::Array(arr) => self.gen_array(arr, store_method),
            Expression::Object(obj) => self.gen_object(obj, store_method),
            Expression::Ident(ident) => {
                let interned = InternedStr::from(ident.get_str().as_ref() as &str);
                let load_method = match self.get_local(interned) {
                    Some(id) => Load::Local(id),
                    None => Load::Global(interned),
                };
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
            _ => todo!(),
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
            string::InternedStr,
        },
    };

    #[test]
    fn expr_codegen_test() {
        let mut parser = Parser::new("[1, 2, *[nil, true], false]".as_bytes());
        let result = parser.next_expression(false).unwrap().unwrap();
        let test_ident = InternedStr::from("test");
        #[rustfmt::skip]
        let expected = [
            Bytecode::Move { dst: Store::Global(test_ident), src: Load::Array(4) },
            Bytecode::AppendElement { dst: Load::Global(test_ident), src: Load::Number(1.0) },
            Bytecode::AppendElement { dst: Load::Global(test_ident), src: Load::Number(2.0) },
            Bytecode::Move { dst: Store::Local(0), src: Load::Array(2) },
            Bytecode::AppendElement { dst: Load::Local(0), src: Load::Nil },
            Bytecode::AppendElement { dst: Load::Local(0), src: Load::Bool(true) },
            Bytecode::AppendElements { dst: Load::Global(test_ident), src: Load::Local(0) },
            Bytecode::AppendElement { dst: Load::Global(test_ident), src: Load::Bool(false) },
        ];
        let mut codegen = Codegen::default();
        codegen
            .gen_expr(&result, Some(Store::Global(test_ident)))
            .unwrap();
        for (bc, expected) in codegen.bytecodes.into_iter().zip(expected) {
            println!("{:?}", bc.1);
            assert_eq!(bc.1, expected);
        }
    }
}
