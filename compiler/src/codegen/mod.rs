use std::{cell::RefCell, rc::Rc};

use crate::{
    interpreter::{bytecode::Bytecode, string::ValueStr, FnBody, FnSignature, UpvalueLoc},
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
    base_local_size: usize, // local memory size recorded before the scope creation
    break_locs: Vec<usize>, // bytecode locations at which break statements occurred
    continue_locs: Vec<usize>, // bytecode locations at which continue statements occurred
}
#[derive(Default)]
struct FnFrame {
    locals: Vec<ValueStr>,
    scopes: Vec<Scope>,
    stack_size: usize,
    upvalues: Vec<(ValueStr, UpvalueLoc)>,
    bytecodes: Vec<SpanOf<Bytecode>>,
}
impl FnFrame {
    fn push_stack(&mut self) {
        self.stack_size += 1;
    }
    fn pop_stack(&mut self) {
        self.stack_size -= 1;
    }
    fn stack_size(&self) -> usize {
        self.stack_size
    }
    fn get_upvalue(&self, name: ValueStr) -> Option<usize> {
        self.upvalues.iter().rposition(|n| n.0 == name)
    }
    fn get_local_var(&self, name: ValueStr) -> Option<usize> {
        self.locals.iter().rposition(|n| *n == name)
    }
    fn decl_local(&mut self, name: ValueStr) -> usize {
        assert_eq!(self.stack_size, 0);
        let id = self.locals.len();
        self.locals.push(name);
        id
    }
    fn push_scope(&mut self, kind: ScopeKind, base_loc: usize) {
        assert_eq!(self.stack_size, 0);
        self.scopes.push(Scope {
            kind,
            base_loc,
            base_local_size: self.locals.len(),
            break_locs: vec![],
            continue_locs: vec![],
        });
    }
    fn pop_scope(&mut self) -> Option<Scope> {
        assert_eq!(self.stack_size, 0);
        self.scopes.pop().inspect(|s| {
            self.locals.truncate(s.base_local_size as usize);
            self.stack_size = s.base_local_size - self.locals.len();
        })
    }
    fn total_size(&self) -> usize {
        self.locals.len() + self.stack_size
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
            stack_size: 0,
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
    fn decl_local(&mut self, name: ValueStr) -> Option<usize> {
        match self.frames.last_mut() {
            Some(f) => Some(f.decl_local(name)),
            None if !self.global_frame.scopes.is_empty() => {
                Some(self.global_frame.decl_local(name))
            }
            _ => None,
        }
    }
    fn total_size(&self) -> usize {
        self.last_frame().total_size()
    }
    fn push_stack(&mut self) {
        self.last_frame_mut().push_stack();
    }
    fn pop_stack(&mut self) {
        self.last_frame_mut().pop_stack();
    }
    fn stack_size(&self) -> usize {
        self.last_frame().stack_size()
    }
    fn get_local_var(&self, name: ValueStr) -> Option<usize> {
        self.last_frame().get_local_var(name)
    }
    fn get_upvalue(&mut self, name: ValueStr) -> Option<usize> {
        let f = self.frames.last_mut()?;
        if let Some(idx) = f.get_upvalue(name.clone()) {
            Some(idx)
        } else {
            for idx in (0..(self.frames.len() - 1)).rev() {
                if let Some(mut id) = self.frames[idx].get_upvalue(name.clone()) {
                    // found id in parent frame's upvalue, propagate
                    for i in (idx + 1)..self.frames.len() {
                        let f = &mut self.frames[i];
                        f.upvalues.push((name.clone(), UpvalueLoc::Shared(id)));
                        id = f.upvalues.len() - 1;
                    }
                    return Some(id);
                }
                if let Some(mut id) = self.frames[idx].get_local_var(name.clone()) {
                    // found id, add upvalue to the inner frame
                    let f = &mut self.frames[idx + 1];
                    f.upvalues.push((name.clone(), UpvalueLoc::Local(id)));
                    // now propagate inner by each parent frame's indices
                    id = f.upvalues.len() - 1;
                    for i in (idx + 2)..self.frames.len() {
                        let f = &mut self.frames[i];
                        f.upvalues.push((name.clone(), UpvalueLoc::Shared(id)));
                        id = f.upvalues.len() - 1;
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
