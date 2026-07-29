use crate::{
    interpreter::{
        bytecode::{Bytecode, Load, Store},
        string::InternedStr,
        LocalId, UpvalueLoc,
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
    base_local_size: LocalId, // Evaluation size recorded before the scope creation
    break_locs: Vec<usize>,   // bytecode locations at which break statements occurred
    continue_locs: Vec<usize>, // bytecode locations at which continue statements occurred
}
#[derive(Default)]
struct FnFrame {
    locals: Vec<InternedStr>,
    scopes: Vec<Scope>,
    eval_size: LocalId,
    upvalues: Vec<(InternedStr, UpvalueLoc)>,
    bytecodes: Vec<SpanOf<Bytecode>>,
}
impl FnFrame {
    fn get_upvalue(&self, name: InternedStr) -> Option<LocalId> {
        self.upvalues
            .iter()
            .rposition(|n| n.0 == name)
            .map(|i| i as LocalId)
    }
    fn get_local_var(&self, name: InternedStr) -> Option<LocalId> {
        self.locals
            .iter()
            .rposition(|n| *n == name)
            .map(|i| i as LocalId)
    }
    fn decl_local(&mut self, name: InternedStr) -> LocalId {
        assert_eq!(self.eval_size, 0);
        let id = self.locals.len();
        self.locals.push(name);
        id as LocalId
    }
    fn push_scope(&mut self, kind: ScopeKind) {
        assert_eq!(self.eval_size, 0);
        self.scopes.push(Scope {
            kind,
            base_local_size: self.locals.len() as LocalId,
            break_locs: vec![],
            continue_locs: vec![],
        });
    }
    fn pop_scope(&mut self) -> Option<Scope> {
        assert_eq!(self.eval_size, 0);
        self.scopes.pop().map(|s| {
            self.locals.truncate(s.base_local_size as usize);
            self.eval_size = s.base_local_size - self.locals.len() as LocalId;
            s
        })
    }
    fn total_size(&self) -> LocalId {
        self.locals.len() as LocalId + self.eval_size
    }
}

#[derive(Default)]
pub struct Codegen {
    frames: Vec<FnFrame>,
    global_frame: FnFrame,
}
impl Codegen {
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
    fn decl_local(&mut self, name: InternedStr) -> Option<LocalId> {
        match self.frames.last_mut() {
            Some(f) => Some(f.decl_local(name)),
            None if !self.global_frame.scopes.is_empty() => {
                Some(self.global_frame.decl_local(name))
            }
            _ => None,
        }
    }
    fn store_ident(&mut self, name: InternedStr) -> Store {
        if let Some(id) = self.get_local_var(name) {
            Store::Local(id)
        } else if let Some(id) = self.get_upvalue(name) {
            Store::Upvalue(id)
        } else {
            Store::Global(name)
        }
    }
    fn load_ident(&mut self, name: InternedStr) -> Load {
        if let Some(id) = self.get_local_var(name) {
            Load::Local(id)
        } else if let Some(id) = self.get_upvalue(name) {
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
    fn get_local_var(&self, name: InternedStr) -> Option<LocalId> {
        self.last_frame().get_local_var(name)
    }
    fn get_upvalue(&mut self, name: InternedStr) -> Option<LocalId> {
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
                if let Some(mut id) = self.frames[idx].get_local_var(name) {
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
}
