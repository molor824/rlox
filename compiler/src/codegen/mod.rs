use std::{cell::RefCell, rc::Rc};

use crate::{
    interpreter::{
        bytecode::{Bytecode, Load, Store},
        string::ValueStr,
        FnBody, FnSignature, LocalId, UpvalueLoc,
    },
    span::SpanOf,
};

mod binary;
mod decl;
mod expression;
mod statement;
mod unary;

#[derive(PartialEq, Eq)]
enum ScopeKind {
    Block, // if
    Loop,  // while, for (allows break, continue statements to be used)
}
struct Scope {
    kind: ScopeKind,
    base_loc: usize,
    base_local_size: LocalId, // Evaluation size recorded before the scope creation
    break_locs: Vec<usize>,   // bytecode locations at which break statements occurred
    continue_locs: Vec<usize>, // bytecode locations at which continue statements occurred
}
#[derive(Default)]
struct FnFrame {
    locals: Vec<ValueStr>,
    scopes: Vec<Scope>,
    eval_size: LocalId,
    upvalues: Vec<(ValueStr, UpvalueLoc)>,
    bytecodes: Vec<SpanOf<Bytecode>>,
}
impl FnFrame {
    fn get_upvalue(&self, name: ValueStr) -> Option<LocalId> {
        self.upvalues
            .iter()
            .rposition(|n| n.0 == name)
            .map(|i| i as LocalId)
    }
    fn get_local_var(&self, name: ValueStr) -> Option<LocalId> {
        self.locals
            .iter()
            .rposition(|n| *n == name)
            .map(|i| i as LocalId)
    }
    fn decl_local(&mut self, name: ValueStr) -> LocalId {
        assert_eq!(self.eval_size, 0);
        let id = self.locals.len();
        self.locals.push(name);
        id as LocalId
    }
    fn push_scope(&mut self, kind: ScopeKind, base_loc: usize) {
        assert_eq!(self.eval_size, 0);
        self.scopes.push(Scope {
            kind,
            base_loc,
            base_local_size: self.locals.len() as LocalId,
            break_locs: vec![],
            continue_locs: vec![],
        });
    }
    fn pop_scope(&mut self) -> Option<Scope> {
        assert_eq!(self.eval_size, 0);
        self.scopes.pop().inspect(|s| {
            self.locals.truncate(s.base_local_size as usize);
            self.eval_size = s.base_local_size - self.locals.len() as LocalId;
        })
    }
    fn total_size(&self) -> LocalId {
        self.locals.len() as LocalId + self.eval_size
    }
}

pub struct Codegen {
    frames: Vec<FnFrame>,
    global_frame: FnFrame,
    source: Rc<RefCell<String>>,
}
impl Codegen {
    pub fn with_source(source: Rc<RefCell<String>>) -> Self {
        Self {
            frames: vec![],
            global_frame: FnFrame::default(),
            source,
        }
    }
    fn last_frame(&self) -> &FnFrame {
        self.frames.last().unwrap_or(&self.global_frame)
    }
    fn last_frame_mut(&mut self) -> &mut FnFrame {
        self.frames.last_mut().unwrap_or(&mut self.global_frame)
    }
    fn push_frame(&mut self) {
        self.frames.push(FnFrame {
            locals: vec![],
            scopes: vec![],
            eval_size: 0,
            upvalues: vec![],
            bytecodes: vec![],
        });
    }
    fn pop_frame(&mut self) -> Option<FnFrame> {
        self.frames.pop()
    }
    fn push_bytecode(&mut self, bytecode: SpanOf<Bytecode>) {
        self.last_frame_mut().bytecodes.push(bytecode);
    }
    pub fn bytecodes(&self) -> &[SpanOf<Bytecode>] {
        &self.last_frame().bytecodes
    }
    fn bytecodes_mut(&mut self) -> &mut [SpanOf<Bytecode>] {
        &mut self.last_frame_mut().bytecodes
    }
    fn decl_local(&mut self, name: ValueStr) -> Option<LocalId> {
        match self.frames.last_mut() {
            Some(f) => Some(f.decl_local(name)),
            None if !self.global_frame.scopes.is_empty() => {
                Some(self.global_frame.decl_local(name))
            }
            _ => None,
        }
    }
    fn store_ident(&mut self, name: ValueStr) -> Store {
        if let Some(id) = self.get_local_var(name.clone()) {
            Store::Local(id)
        } else if let Some(id) = self.get_upvalue(name.clone()) {
            Store::Upvalue(id)
        } else {
            Store::Global(name)
        }
    }
    fn load_ident(&mut self, name: ValueStr) -> Load {
        if let Some(id) = self.get_local_var(name.clone()) {
            Load::Local(id)
        } else if let Some(id) = self.get_upvalue(name.clone()) {
            Load::Upvalue(id)
        } else {
            Load::Global(name)
        }
    }
    fn total_size(&self) -> LocalId {
        self.last_frame().total_size()
    }
    fn push_eval_id(&mut self) -> LocalId {
        let f = self.last_frame_mut();
        let id = f.eval_size + f.locals.len() as LocalId;
        f.eval_size += 1;
        id
    }
    fn get_local_var(&self, name: ValueStr) -> Option<LocalId> {
        self.last_frame().get_local_var(name)
    }
    fn get_upvalue(&mut self, name: ValueStr) -> Option<LocalId> {
        let f = self.frames.last_mut()?;
        if let Some(idx) = f.get_upvalue(name.clone()) {
            Some(idx as LocalId)
        } else {
            for idx in (0..(self.frames.len() - 1)).rev() {
                if let Some(mut id) = self.frames[idx].get_upvalue(name.clone()) {
                    // found id in parent frame's upvalue, propagate
                    for i in (idx + 1)..self.frames.len() {
                        let f = &mut self.frames[i];
                        f.upvalues.push((name.clone(), UpvalueLoc::Shared(id)));
                        id = f.upvalues.len() as LocalId - 1;
                    }
                    return Some(id);
                }
                if let Some(mut id) = self.frames[idx].get_local_var(name.clone()) {
                    // found id, add upvalue to the inner frame
                    let f = &mut self.frames[idx + 1];
                    f.upvalues.push((name.clone(), UpvalueLoc::Local(id)));
                    // now propagate inner by each parent frame's indices
                    id = f.upvalues.len() as LocalId - 1;
                    for i in (idx + 2)..self.frames.len() {
                        let f = &mut self.frames[i];
                        f.upvalues.push((name.clone(), UpvalueLoc::Shared(id)));
                        id = f.upvalues.len() as LocalId - 1;
                    }
                    return Some(id);
                }
            }
            None
        }
    }
    pub fn pop_init_sig(self) -> FnSignature {
        assert!(self.frames.is_empty(), "Incomplete function frames exist!");
        let frame = self.global_frame;
        FnSignature {
            arity: 0,
            variadic: false,
            upvalues: vec![],
            body: FnBody::Bytecode(frame.bytecodes),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{ast::Parser, codegen::Codegen};

    #[test]
    fn test_codegen() {
        let mut parser = Parser::new(
            r#"
            fn fib(n) do
                let a = 0
                let b = 1
                let i = 0
                while i < n do
                    let c = a + b
                    a = b
                    b = c
                    i = i + 1
                end
                return i
            end
            "#
            .as_bytes(),
        );
        let mut codegen = Codegen::with_source(parser.source());
        codegen
            .gen_statement(&parser.next_statement().unwrap().unwrap())
            .unwrap();

        for bc in codegen.bytecodes() {
            println!("{:?}", bc.1);
        }
    }
}
