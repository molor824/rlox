use crate::{
    ast::expression::{Element, Expression, Pair},
    interpreter::{
        bytecode::{Bytecode, Load, Store},
        string::InternedStr,
        LocalId,
    },
    span::{GetSpan, SpanOf},
};

#[derive(Default)]
pub struct Codegen {
    bytecodes: Vec<SpanOf<Bytecode>>,
    local_len: LocalId,
}
impl Codegen {
    fn push_bytecode(&mut self, bytecode: SpanOf<Bytecode>) {
        self.bytecodes.push(bytecode);
    }
    fn inc_local(&mut self) -> LocalId {
        let temp = self.local_len;
        self.local_len += 1;
        temp
    }
    fn gen_array(&mut self, arr: &SpanOf<Vec<Element>>, store_method: &Store) {
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
                    let next_local = self.inc_local();
                    self.gen_expr(expr, &Store::Local(next_local));
                    self.push_bytecode(SpanOf(
                        expr.span(),
                        Bytecode::AppendElement {
                            dst: load_method.clone(),
                            src: Load::Local(next_local),
                        },
                    ));
                }
                Element::Unpack(unpack) => {
                    let next_local = self.inc_local();
                    self.gen_expr(&unpack.1, &Store::Local(next_local));
                    self.push_bytecode(SpanOf(
                        unpack.0,
                        Bytecode::AppendElements {
                            dst: load_method.clone(),
                            src: Load::Local(next_local),
                        },
                    ));
                }
            }
        }
    }
    fn gen_object(&mut self, obj: &SpanOf<Vec<Pair>>, store_method: &Store) {
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
                    let next_local = self.inc_local();
                    self.gen_expr(value, &Store::Local(next_local));
                    let key_str = key.get_str();
                    self.push_bytecode(SpanOf(
                        key.0.concat(value.span()),
                        Bytecode::StoreProperty {
                            dst: load_method.clone(),
                            src: Load::Local(next_local),
                            prop: InternedStr::from(&key_str as &str),
                        },
                    ));
                }
                Pair::Index(key, value) => {
                    let next_key_local = self.inc_local();
                    self.gen_expr(&key.1, &Store::Local(next_key_local));
                    let next_value_local = self.inc_local();
                    self.gen_expr(value, &Store::Local(next_value_local));
                    self.push_bytecode(SpanOf(
                        key.0.concat(value.span()),
                        Bytecode::StorePropertyIndirect {
                            dst: load_method.clone(),
                            src: Load::Local(next_value_local),
                            prop: Load::Local(next_key_local),
                        },
                    ));
                }
                Pair::Unpack(unpack) => {
                    let next_local = self.inc_local();
                    self.gen_expr(&unpack.1, &Store::Local(next_local));
                    self.push_bytecode(SpanOf(
                        unpack.0,
                        Bytecode::StoreProperties {
                            dst: load_method.clone(),
                            src: Load::Local(next_local),
                        },
                    ));
                }
            }
        }
    }
    pub fn gen_expr(&mut self, expr: &Expression, store_method: &Store) {
        match expr {
            Expression::Nil(span) => self.push_bytecode(SpanOf(
                *span,
                Bytecode::Move {
                    dst: store_method.clone(),
                    src: Load::Nil,
                },
            )),
            Expression::Boolean(bool) => self.push_bytecode(SpanOf(
                bool.0,
                Bytecode::Move {
                    dst: store_method.clone(),
                    src: Load::Bool(bool.1),
                },
            )),
            Expression::Number(n) => self.push_bytecode(SpanOf(
                n.0,
                Bytecode::Move {
                    dst: store_method.clone(),
                    src: Load::Number(n.1.to_f64()),
                },
            )),
            Expression::String(str) => self.push_bytecode(SpanOf(
                str.0,
                Bytecode::Move {
                    dst: store_method.clone(),
                    src: Load::String(str.1.as_str().into()),
                },
            )),
            Expression::Array(arr) => self.gen_array(arr, store_method),
            Expression::Object(obj) => self.gen_object(obj, store_method),
            _ => todo!(),
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
    fn expr_codegen_test() {
        let mut parser = Parser::new("[1, 2, *[nil, true], false]".as_bytes());
        let result = parser.next_expression(false).unwrap().unwrap();
        #[rustfmt::skip]
        let expected = [
            Bytecode::Move { dst: Store::Local(0), src: Load::Array(4) },
            Bytecode::Move { dst: Store::Local(1), src: Load::Number(1.0) },
            Bytecode::AppendElement { dst: Load::Local(0), src: Load::Local(1) },
            Bytecode::Move { dst: Store::Local(2), src: Load::Number(2.0) },
            Bytecode::AppendElement { dst: Load::Local(0), src: Load::Local(2) },
            Bytecode::Move { dst: Store::Local(3), src: Load::Array(2) },
            Bytecode::Move { dst: Store::Local(4), src: Load::Nil },
            Bytecode::AppendElement { dst: Load::Local(3), src: Load::Local(4) },
            Bytecode::Move { dst: Store::Local(5), src: Load::Bool(true) },
            Bytecode::AppendElement { dst: Load::Local(3), src: Load::Local(5) },
            Bytecode::AppendElements { dst: Load::Local(0), src: Load::Local(3) },
            Bytecode::Move { dst: Store::Local(6), src: Load::Bool(false) },
            Bytecode::AppendElement { dst: Load::Local(0), src: Load::Local(6) },
        ];
        let mut codegen = Codegen::default();
        let local = codegen.inc_local();
        codegen.gen_expr(&result, &Store::Local(local));
        for (bc, expected) in codegen.bytecodes.into_iter().zip(expected) {
            println!("{:?}", bc.1);
            assert_eq!(bc.1, expected);
        }
    }
}
