use std::{cell::RefCell, collections::HashMap, mem::swap, rc::Rc};

use crate::{
    ast::expression::Pair::Ident,
    interpreter::{bytecode::Bytecode, value::Value},
};

pub mod bytecode;
pub mod error;
pub mod value;

pub type GlobalId = u32;
pub type LocalId = i32;
pub type ClosureId = u32;

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
    globals: Globals,
    closures: Vec<Closure>,
}
impl Default for Interpreter {
    fn default() -> Self {
        Self {
            memory: Vec::with_capacity(0x100000),
            instruction_pointer: 0,
            current_frame: FunctionFrame::default(),
            frame_stack: vec![],
            globals: Globals::default(),
            closures: vec![],
        }
    }
}
impl Interpreter {
    pub fn add_closure(&mut self, closure: Closure) -> ClosureId {
        let id = self.closures.len();
        self.closures.push(closure);
        id as ClosureId
    }
    pub fn declare_global(&mut self, name: &str, value: Value) -> Option<GlobalId> {
        self.globals.declare_value(name, value)
    }
    pub fn run_closure(&mut self, closure: ClosureId) {}
    fn get_local(&self, id: LocalId) -> Option<&Value> {
        if id < -(self.current_frame.arity as LocalId) {
            return None;
        }
        let absolute_id = self.current_frame.base_pointer as LocalId + id;
        self.memory.get(absolute_id as usize)
    }
    fn set_local(&mut self, id: LocalId, new_value: Value) {
        if id < -(self.current_frame.arity as LocalId) {
            panic!("id({}) exceeds arity({})!", id, self.current_frame.arity);
        }
        let index = (self.current_frame.base_pointer as LocalId + id) as usize;
        if index >= self.memory.capacity() {
            panic!(
                "index({}) exceeds stack capacity({})!",
                index,
                self.memory.capacity()
            );
        }
        if index <= self.memory.len() {
            self.memory.resize(index + 1, Value::Nil);
        }
        self.memory[index] = new_value;
    }
    fn interpret(&mut self, bytecode: Bytecode) {
        match bytecode {
            Bytecode::LoadArity(id) => {
                self.set_local(id, Value::Integer(Rc::new(self.current_frame.arity.into())));
            }
            Bytecode::LoadNil(id) => self.set_local(id, Value::Nil),
            Bytecode::LoadInt(id, int) => self.set_local(id, Value::Integer(Rc::new(int.into()))),
            Bytecode::LoadFloat(id, float) => self.set_local(id, Value::Float(float)),
            Bytecode::LoadStr(id, str) => self.set_local(id, Value::String(str)),
            Bytecode::LoadObject(id, capacity) => self.set_local(
                id,
                Value::Object(Rc::new(RefCell::new(HashMap::with_capacity(capacity)))),
            ),
            Bytecode::LoadArray(id, capacity) => self.set_local(
                id,
                Value::Array(Rc::new(RefCell::new(Vec::with_capacity(capacity)))),
            ),
            _ => todo!(),
        }
    }
}

#[derive(Debug)]
pub struct Closure {
    min_arity: usize,
    variadic: bool,
    upvalues: Vec<LocalId>,
    body: Vec<Bytecode>,
}

#[derive(Default)]
pub struct Globals {
    names: Vec<Rc<str>>,
    name_to_id: HashMap<Rc<str>, GlobalId>,
    values: HashMap<GlobalId, Value>,
}
impl Globals {
    pub fn declare_value(&mut self, name: &str, value: Value) -> Option<GlobalId> {
        if self.name_to_id.get(name).is_some() {
            None
        } else {
            let name = Rc::<str>::from(name);
            let id = self.names.len() as GlobalId;
            self.names.push(name.clone());
            self.name_to_id.insert(name, id);
            self.values.insert(id, value);
            Some(id)
        }
    }
    pub fn set_value(&mut self, id: GlobalId, mut new_value: Value) -> Option<Value> {
        let value = self.values.get_mut(&id)?;
        swap(value, &mut new_value);
        Some(new_value)
    }
}
