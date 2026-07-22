use crate::{
    ast::{
        declaration::Declaration,
        expression::{Element, Expression, Pair},
    },
    error::Result,
    interpreter::{
        bytecode::{Bytecode, Load, Store},
        string::InternedStr,
        LocalId, UpvalueLoc,
    },
    span::{GetSpan, SpanOf},
};

mod binary;
mod decl;
mod unary;

struct FnFrame {
    locals: Vec<InternedStr>,
    eval_size: LocalId,
    upvalues: Vec<(InternedStr, UpvalueLoc)>,
}
impl FnFrame {
    fn get_local(&self, name: InternedStr) -> Option<LocalId> {
        self.locals
            .iter()
            .rposition(|n| *n == name)
            .map(|id| id as LocalId)
    }
    fn get_upvalue(&self, name: InternedStr) -> Option<LocalId> {
        self.upvalues
            .iter()
            .rposition(|n| n.0 == name)
            .map(|id| id as LocalId)
    }
}

#[derive(Default)]
pub struct Codegen {
    bytecodes: Vec<SpanOf<Bytecode>>,
    frames: Vec<FnFrame>,
    global_eval_size: LocalId, // This one is used for global scope
}
impl Codegen {
    fn push_bytecode(&mut self, bytecode: SpanOf<Bytecode>) {
        self.bytecodes.push(bytecode);
    }
    fn push_local(&mut self, name: InternedStr) -> Option<LocalId> {
        let f = self.frames.last_mut()?;
        f.eval_size = 0;
        f.locals.push(name);
        Some(f.locals.len() as LocalId - 1)
    }
    pub(crate) fn store_ident(&mut self, name: InternedStr) -> Store {
        if let Some(id) = self.get_local(name) {
            Store::Local(id)
        } else if let Some(id) = self.get_upvalue(name) {
            Store::Upvalue(id)
        } else {
            Store::Global(name)
        }
    }
    pub(crate) fn load_ident(&mut self, name: InternedStr) -> Load {
        if let Some(id) = self.get_local(name) {
            Load::Local(id)
        } else if let Some(id) = self.get_upvalue(name) {
            Load::Upvalue(id)
        } else {
            Load::Global(name)
        }
    }
    pub(crate) fn eval_size(&self) -> LocalId {
        match self.frames.last() {
            Some(f) => f.eval_size + f.locals.len() as LocalId,
            None => self.global_eval_size,
        }
    }
    pub(crate) fn gen_eval_id(&mut self) -> LocalId {
        if let Some(f) = self.frames.last_mut() {
            let id = f.eval_size + f.locals.len() as LocalId;
            f.eval_size += 1;
            id
        } else {
            let id = self.global_eval_size;
            self.global_eval_size += 1;
            id
        }
    }
    pub(crate) fn get_local(&self, name: InternedStr) -> Option<LocalId> {
        self.frames.last()?.get_local(name)
    }
    pub(crate) fn get_upvalue(&mut self, name: InternedStr) -> Option<LocalId> {
        // NOTE: Refer to interpreter::FnSignature for the recursive and super upvalue capture ordering!!!
        let f = self.frames.last_mut()?;
        if let Some(idx) = f.get_upvalue(name) {
            Some(idx as LocalId)
        } else {
            for idx in (0..(self.frames.len() - 1)).rev() {
                if let Some(mut id) = self.frames[idx].get_upvalue(name) {
                    // found id in parent frame's upvalue, propagate
                    for i in (idx + 1)..self.frames.len() {
                        let f = &mut self.frames[i];
                        f.upvalues.push((name, UpvalueLoc::Shared(id)));
                        id = f.upvalues.len() as LocalId - 1;
                    }
                    return Some(id);
                }
                if let Some(mut id) = self.frames[idx].get_local(name) {
                    // found id, add upvalue to the inner frame
                    let f = &mut self.frames[idx + 1];
                    f.upvalues.push((name, UpvalueLoc::Local(id)));
                    // now propagate inner by each parent frame's indices
                    id = f.upvalues.len() as LocalId - 1;
                    for i in (idx + 2)..self.frames.len() {
                        let f = &mut self.frames[i];
                        f.upvalues.push((name, UpvalueLoc::Shared(id)));
                        id = f.upvalues.len() as LocalId - 1;
                    }
                    return Some(id);
                }
            }
            None
        }
    }
    fn gen_array(
        &mut self,
        arr: &SpanOf<Vec<Element>>,
        store_method: Option<Store>,
    ) -> Result<Load> {
        let store_method = match store_method {
            Some(s) => s,
            None if arr.1.is_empty() => return Ok(Load::Array(0)),
            None => Store::Local(self.gen_eval_id()),
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
            None => Store::Local(self.gen_eval_id()),
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
            _ => todo!(),
        }
    }
    pub fn gen_decl(&mut self, declaration: &Declaration) -> Result<()> {
        match declaration {
            Declaration::VarDecl(decl) => self.gen_var_decl(&decl),
            Declaration::Expression(expr) => self.gen_expr(expr, Some(Store::Nil)).map(|_| ()),
            Declaration::FuncDecl(_) => todo!(),
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
