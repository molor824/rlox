use std::cell::RefCell;
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
    upvalues: Vec<LocalId>,
    body: Vec<Bytecode>,
}

pub struct Interpreter {
    memory: Vec<Value>,
    instruction_pointer: usize,
    next_ip: usize,
    frame_stack: Vec<FunctionFrame>,
    globals: FxHashMap<ValueStr, Value>,
    readonly_globals: FxHashSet<ValueStr>,
}
impl Default for Interpreter {
    fn default() -> Self {
        Self {
            memory: Vec::with_capacity(0x100000),
            instruction_pointer: 0,
            next_ip: 0,
            frame_stack: vec![],
            readonly_globals: FxHashSet::default(),
            globals: FxHashMap::default(),
        }
    }
}
impl Interpreter {
    fn current_frame(&self) -> &FunctionFrame {
        self.frame_stack.last().unwrap()
    }
    fn current_frame_mut(&mut self) -> &mut FunctionFrame {
        self.frame_stack.last_mut().unwrap()
    }
    fn get_local(&self, id: LocalId) -> Value {
        let absolute_id = self.current_frame().base_pointer + id as usize;
        self.memory.get(absolute_id).cloned().unwrap_or_default()
    }
    fn set_local(&mut self, id: LocalId, new_value: Value) -> Result<(), ErrorKind> {
        let index = self.current_frame().base_pointer + id as usize;
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
    fn truncate(&mut self, amount: usize) -> Result<(), ErrorKind> {
        let new_len = self.memory.len() - amount;
        if new_len < self.current_frame().base_pointer {
            return Err(ErrorKind::StackUnderflow);
        }
        self.memory.truncate(new_len);
        Ok(())
    }
    fn create_function(&mut self, signature: Rc<FnSignature>) -> Result<Function, ErrorKind> {
        let mut upvalues = Vec::with_capacity(signature.upvalues.len());
        for offset in signature.upvalues.iter().copied() {
            let value = self.get_local(offset);
            self.set_local(offset, Value::Nil)?;
            upvalues.push(Rc::new(RefCell::new(value)));
        }
        Ok(Function {
            signature,
            upvalues,
        })
    }
}
