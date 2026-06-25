use crate::interpreter::error::ErrorKind;
use crate::interpreter::string::ValueStr;
use crate::interpreter::value::{Closure, Value};
use rustc_hash::FxHashMap;

pub mod bytecode;
pub mod error;
pub mod string;
pub mod value;

pub type LocalId = i32;
pub type ClosureId = usize;

#[derive(Default)]
struct FunctionFrame {
    base_pointer: usize,
    arity: usize,
}

pub struct Interpreter {
    memory: Vec<Value>,
    instruction_pointer: usize,
    next_ip: usize,
    current_frame: FunctionFrame,
    frame_stack: Vec<FunctionFrame>,
    globals: FxHashMap<ValueStr, Value>,
    closures: Vec<Closure>,
}
impl Default for Interpreter {
    fn default() -> Self {
        Self {
            memory: Vec::with_capacity(0x100000),
            instruction_pointer: 0,
            next_ip: 0,
            current_frame: FunctionFrame::default(),
            frame_stack: vec![],
            closures: vec![],
            globals: FxHashMap::default(),
        }
    }
}
impl Interpreter {
    fn get_local(&self, id: LocalId) -> Option<Value> {
        if id < -(self.current_frame.arity as LocalId) {
            return None;
        }
        let absolute_id = self.current_frame.base_pointer as LocalId + id;
        self.memory.get(absolute_id as usize).cloned()
    }
    fn set_local(&mut self, id: LocalId, new_value: Value) -> Result<(), ErrorKind> {
        if id < -(self.current_frame.arity as LocalId) {
            return Err(ErrorKind::ArityOverflow(id, self.current_frame.arity));
        }
        let index = (self.current_frame.base_pointer as LocalId + id) as usize;
        if index >= self.memory.capacity() {
            return Err(ErrorKind::StackOverflow(index, self.memory.capacity()));
        }
        if index <= self.memory.len() {
            self.memory.resize(index + 1, Value::Nil);
        }
        self.memory[index] = new_value;
        Ok(())
    }
    fn get_global(&self, id: ValueStr) -> Option<Value> {
        self.globals.get(&id).cloned()
    }
    fn set_global(&mut self, id: ValueStr, new_value: Value) {
        self.globals.insert(id, new_value);
    }
}
