use core::fmt;
use std::cell::RefCell;
use std::mem;
use std::rc::Rc;

use crate::interpreter::error::ErrorKind;
use crate::interpreter::string::ValueStr;
use crate::interpreter::value::Value;
use crate::interpreter::{bytecode::Bytecode, value::Function};
use rustc_hash::{FxHashMap, FxHashSet};

pub mod bytecode;
pub mod error;
pub mod string;
pub mod value;

pub type LocalId = u32;

struct FunctionFrame {
    base_pointer: usize,
    arity: usize,
    function: Rc<Function>,
}

#[derive(Debug)]
pub struct FnSignature {
    min_arity: usize,
    variadic: bool,
    capture_locations: Vec<LocalId>, // location relative to parent's local scope
    parent_capture_indices: Vec<usize>, // indices of parent's captured upvalues, to be recursively captured
    body: FnBody,
}

pub enum FnBody {
    Bytecode(Vec<Bytecode>),
    Builtin(Box<dyn Fn(&mut Interpreter) -> Result<(), ErrorKind>>),
}
impl fmt::Debug for FnBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

pub struct Interpreter {
    memory: Vec<Value>,
    current_frame: Option<FunctionFrame>,
    globals: FxHashMap<ValueStr, Value>,
    readonly_globals: FxHashSet<ValueStr>,
}
impl Default for Interpreter {
    fn default() -> Self {
        Self {
            memory: Vec::with_capacity(0x100000),
            current_frame: None,
            readonly_globals: FxHashSet::default(),
            globals: FxHashMap::default(),
        }
    }
}
impl Interpreter {
    fn get_local(&self, id: LocalId) -> Value {
        let absolute_id = self.current_frame.as_ref().unwrap().base_pointer + id as usize;
        self.memory.get(absolute_id).cloned().unwrap_or_default()
    }
    fn set_local(&mut self, id: LocalId, new_value: Value) -> Result<(), ErrorKind> {
        let index = self.current_frame.as_ref().unwrap().base_pointer + id as usize;
        if index >= self.memory.capacity() {
            return Err(ErrorKind::StackOverflow);
        }
        if index >= self.memory.len() {
            self.memory.resize(index + 1, Value::Nil);
        }
        self.memory[index] = new_value;
        Ok(())
    }
    fn make_global_read_only(&mut self, id: ValueStr) {
        self.readonly_globals.insert(id);
    }
    fn get_global(&self, id: ValueStr) -> Value {
        self.globals.get(&id).cloned().unwrap_or_default()
    }
    fn set_global(&mut self, id: ValueStr, new_value: Value) -> Result<(), ErrorKind> {
        if self.readonly_globals.contains(&id) {
            return Err(ErrorKind::ReadonlyGlobalWrite(id));
        }
        self.globals.insert(id, new_value);
        Ok(())
    }
    fn truncate(&mut self, new_len: usize) -> Result<(), ErrorKind> {
        if new_len < self.current_frame.as_ref().unwrap().base_pointer {
            return Err(ErrorKind::StackUnderflow);
        }
        self.memory.truncate(new_len);
        Ok(())
    }
    fn method_currying(
        &mut self,
        itself: Value,
        function: Rc<Function>,
    ) -> Result<Function, ErrorKind> {
        let function1 = function.clone();
        let curried_method = move |interpreter: &mut Self| -> Result<(), ErrorKind> {
            // shift all arguments forward, while inserting itself
            let current_frame = interpreter.current_frame.as_ref().unwrap();
            let arity = current_frame.arity;
            let start = current_frame.base_pointer + 1;
            let end = start + arity;
            interpreter.set_local(end as LocalId, itself.clone())?;
            let slice = &mut interpreter.memory[start..=end];
            slice.rotate_right(1);
            interpreter.call_function(function.clone(), arity + 1)
        };
        self.create_function(Rc::new(FnSignature {
            body: FnBody::Builtin(Box::new(curried_method)),
            min_arity: function1.signature.min_arity + 1,
            variadic: function1.signature.variadic,
            capture_locations: vec![],
            parent_capture_indices: vec![],
        }))
    }
    fn create_function(&mut self, signature: Rc<FnSignature>) -> Result<Function, ErrorKind> {
        let mut upvalues = Vec::with_capacity(
            signature.capture_locations.len() + signature.parent_capture_indices.len(),
        );
        for index in signature.capture_locations.iter().copied() {
            let value = Rc::new(RefCell::new(self.get_local(index)));
            self.set_local(index, Value::Upvalue(value.clone()))?;
            upvalues.push(value);
        }
        for index in signature.parent_capture_indices.iter().copied() {
            let value = self.current_frame.as_ref().unwrap().function.upvalues[index].clone();
            upvalues.push(value);
        }
        Ok(Function {
            signature,
            upvalues,
        })
    }
    fn call_function(&mut self, function: Rc<Function>, arity: usize) -> Result<(), ErrorKind> {
        let mut old_frame = Some(FunctionFrame {
            base_pointer: self
                .memory
                .len()
                .checked_sub(arity + 1)
                .ok_or(ErrorKind::StackUnderflow)?,
            arity,
            function: function.clone(),
        });
        let old_len = self.memory.len();
        mem::swap(&mut old_frame, &mut self.current_frame);

        match &function.signature.body {
            FnBody::Builtin(builtin) => builtin(self)?,
            FnBody::Bytecode(bytecodes) => {
                let mut index = 0;
                loop {
                    match bytecodes[index].interpret(self, index)? {
                        Some(next) => index = next,
                        None => break,
                    }
                }
            }
        }

        self.truncate(old_len)?;
        self.current_frame = old_frame;
        Ok(())
    }
}
