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
    function: Rc<Function>,
}

#[derive(Debug)]
pub struct FnSignature {
    arity: usize,                       // NOTE: arity EXCLUDES variadic parameter!
    variadic: bool,                     // if true, function has variadic parameter.
    capture_locations: Vec<LocalId>,    // location relative to parent's local scope
    parent_capture_indices: Vec<usize>, // indices of parent's captured upvalues, to be recursively captured
    body: FnBody,
}
impl FnSignature {
    fn required_arity(&self) -> usize {
        self.arity + if self.variadic { 1 } else { 0 }
    }
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

#[derive(Clone)]
enum Cell {
    Value(Value),
    Upvalue(Rc<RefCell<Value>>),
}
pub struct Interpreter {
    memory: Vec<Cell>,
    current_frame: Option<FunctionFrame>,
    globals: FxHashMap<ValueStr, Value>,
    readonly_globals: FxHashSet<ValueStr>,
}
impl Default for Interpreter {
    fn default() -> Self {
        const STACK_SIZE: usize = 0x10000;
        Self {
            memory: Vec::with_capacity(STACK_SIZE),
            current_frame: None,
            readonly_globals: FxHashSet::default(),
            globals: FxHashMap::default(),
        }
    }
}
impl Interpreter {
    fn get_local(&self, id: LocalId) -> Value {
        let absolute_id = self.current_frame.as_ref().unwrap().base_pointer + id as usize;
        match self.memory.get(absolute_id) {
            Some(Cell::Value(v)) => v.clone(),
            Some(Cell::Upvalue(up)) => up.borrow().clone(),
            None => Value::Nil,
        }
    }
    fn set_local(&mut self, id: LocalId, new_value: Value) -> Result<(), ErrorKind> {
        let index = self.current_frame.as_ref().unwrap().base_pointer + id as usize;
        if index >= self.memory.capacity() {
            return Err(ErrorKind::StackOverflow);
        }
        if index >= self.memory.len() {
            self.memory.resize(index + 1, Cell::Value(Value::Nil));
        }
        match &mut self.memory[index] {
            Cell::Upvalue(up) => *up.borrow_mut() = new_value,
            Cell::Value(value) => *value = new_value,
        }
        Ok(())
    }
    fn make_local_upvalue(&mut self, id: LocalId) -> Result<Rc<RefCell<Value>>, ErrorKind> {
        let index = self.current_frame.as_ref().unwrap().base_pointer + id as usize;
        match self.memory.get_mut(index) {
            Some(cell) => match cell {
                Cell::Upvalue(val) => Ok(val.clone()),
                Cell::Value(val) => {
                    let shared = Rc::new(RefCell::new(val.clone()));
                    *cell = Cell::Upvalue(shared.clone());
                    Ok(shared)
                }
            },
            None => Err(ErrorKind::UninitCellShare),
        }
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
            let arity = current_frame.function.signature.required_arity();
            let start = current_frame.base_pointer + 1;

            interpreter.set_local(arity as LocalId + 1, itself.clone())?;
            interpreter.memory[start..].rotate_right(1);
            interpreter.call_function_exact(function.clone())
        };
        self.create_function(Rc::new(FnSignature {
            body: FnBody::Builtin(Box::new(curried_method)),
            arity: function1.signature.arity + 1,
            variadic: function1.signature.variadic,
            capture_locations: vec![],
            parent_capture_indices: vec![],
        }))
    }
    fn create_function(&mut self, signature: Rc<FnSignature>) -> Result<Function, ErrorKind> {
        let mut upvalues = Vec::with_capacity(
            signature.capture_locations.len() + signature.parent_capture_indices.len(),
        );
        for index in signature.parent_capture_indices.iter().copied() {
            let value = self.current_frame.as_ref().unwrap().function.upvalues[index].clone();
            upvalues.push(value);
        }
        for index in signature.capture_locations.iter().copied() {
            let value = self.make_local_upvalue(index)?;
            upvalues.push(value);
        }
        Ok(Function {
            signature,
            upvalues,
        })
    }
    fn call_function_exact(&mut self, function: Rc<Function>) -> Result<(), ErrorKind> {
        let base_pointer = self
            .memory
            .len()
            .checked_sub(function.signature.required_arity() + 1)
            .ok_or(ErrorKind::StackUnderflow)?;
        let mut old_frame = Some(FunctionFrame {
            base_pointer,
            function: function.clone(),
        });
        mem::swap(&mut old_frame, &mut self.current_frame);

        match &function.signature.body {
            FnBody::Builtin(builtin) => builtin(self)?,
            FnBody::Bytecode(bytecodes) => {
                let mut index = 0;
                while let Some(next) = bytecodes[index].interpret(self, index)? {
                    index = next;
                }
            }
        }

        self.truncate(base_pointer + 1)?;
        self.current_frame = old_frame;

        Ok(())
    }
    fn call_function(&mut self, function: Rc<Function>, arity: usize) -> Result<(), ErrorKind> {
        let base_pointer = self
            .memory
            .len()
            .checked_sub(arity + 1)
            .ok_or(ErrorKind::StackUnderflow)?;
        let return_len = base_pointer + 1;
        let signature = function.signature.as_ref();
        if signature.variadic {
            // additional arguments are all combined into list
            let array = (signature.arity..arity)
                .map(|i| match &self.memory[return_len + i] {
                    // Upvalue is generally not allowed as function argument, but if it does happen, just clone the value
                    Cell::Upvalue(shared) => shared.borrow().clone(),
                    Cell::Value(value) => value.clone(),
                })
                .collect::<Vec<_>>();
            let variadic = Value::Array(Rc::new(RefCell::new(array)));
            self.memory[return_len + signature.arity] = Cell::Value(variadic);
        }
        // Truncate until it's no longer past the expected arity
        self.memory
            .truncate(return_len + signature.required_arity());

        self.call_function_exact(function)
    }
}
