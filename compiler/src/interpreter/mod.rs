use crate::interpreter::error::ErrorKind;
use crate::interpreter::value::{Closure, Value};
use rustc_hash::FxHashMap;
use std::rc::Rc;

pub mod bytecode;
pub mod error;
pub mod value;

pub type StringId = usize;
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
    current_frame: FunctionFrame,
    frame_stack: Vec<FunctionFrame>,
    globals: FxHashMap<StringId, Value>,
    str_interner: StrInterner,
    closures: Vec<Closure>,
}
impl Default for Interpreter {
    fn default() -> Self {
        Self {
            memory: Vec::with_capacity(0x100000),
            instruction_pointer: 0,
            current_frame: FunctionFrame::default(),
            frame_stack: vec![],
            str_interner: StrInterner::default(),
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
    fn get_global(&self, id: StringId) -> Option<Value> {
        self.globals.get(&id).cloned()
    }
    fn set_global(&mut self, id: StringId, new_value: Value) {
        self.globals.insert(id, new_value);
    }
}

#[derive(Default)]
pub struct StrInterner {
    strings: Vec<Rc<str>>,
    str_to_id: FxHashMap<Rc<str>, StringId>,
}
impl StrInterner {
    pub fn add_string(&mut self, str: &str) -> StringId {
        match self.str_to_id.get(str) {
            Some(id) => *id,
            None => {
                let id = self.strings.len() as StringId;
                let str = Rc::<str>::from(str);
                self.strings.push(str.clone());
                self.str_to_id.insert(str, id);
                id
            }
        }
    }
    pub fn id(&self, str: &str) -> Option<StringId> {
        self.str_to_id.get(str).copied()
    }
    pub fn str(&self, id: StringId) -> Option<&Rc<str>> {
        self.strings.get(id as usize)
    }
}
