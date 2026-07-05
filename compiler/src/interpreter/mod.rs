use core::fmt;
use std::cell::RefCell;
use std::mem;
use std::rc::Rc;

use crate::error::ErrorKind;
use crate::interpreter::{bytecode::Bytecode, string::ValueStr, value::Function, value::Value};
use crate::span::SpanOf;
use rustc_hash::{FxHashMap, FxHashSet};

pub mod bytecode;
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
    Bytecode(Vec<SpanOf<Bytecode>>),
    Builtin(Box<dyn Fn(&mut Interpreter) -> Result<(), ErrorKind>>),
}
impl fmt::Debug for FnBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bytecode(code) => f.debug_tuple("Bytecode").field(&code.len()).finish(),
            Self::Builtin(builtin) => f
                .debug_tuple("Builtin")
                .field(&(builtin.as_ref() as *const _))
                .finish(),
        }
    }
}

#[derive(Clone)]
enum Cell {
    Value(Value),
    Upvalue(Rc<RefCell<Value>>),
}
impl Default for Cell {
    fn default() -> Self {
        Self::Value(Value::Nil)
    }
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
            self.memory.resize_with(index + 1, Cell::default);
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
    fn get_upvalue(&self, id: LocalId) -> Value {
        let fun = self.current_frame.as_ref().unwrap().function.as_ref();
        fun.upvalues
            .get(id as usize)
            .map(|upvalue| upvalue.borrow().clone())
            .unwrap_or_default()
    }
    fn set_upvalue(&self, id: LocalId, new_value: Value) -> Result<(), ErrorKind> {
        let fun = self.current_frame.as_ref().unwrap().function.as_ref();
        if let Some(v) = fun.upvalues.get(id as usize) {
            *v.borrow_mut() = new_value;
            Ok(())
        } else {
            Err(ErrorKind::InvalidUpvalueAccess)
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
                while let Some(next) = bytecodes[index].1.interpret(self, index)? {
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
            let new_index = return_len + signature.arity;
            if new_index >= self.memory.len() {
                self.memory.resize_with(new_index + 1, Cell::default);
            }
            self.memory[new_index] = Cell::Value(variadic);
        }
        // Truncate until it's no longer past the expected arity
        self.memory
            .truncate(return_len + signature.required_arity());

        self.call_function_exact(function)
    }
    pub fn call_and_return(
        &mut self,
        function: Rc<Function>,
        args: impl IntoIterator<Item = Value>,
    ) -> Result<Value, ErrorKind> {
        self.memory.push(Cell::default());
        let mut arity = 0;
        for arg in args {
            self.memory.push(Cell::Value(arg));
            arity += 1;
        }
        self.call_function(function, arity)?;
        Ok(match self.memory.pop().unwrap() {
            Cell::Value(val) => val,
            Cell::Upvalue(upval) => upval.borrow().clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use crate::{
        interpreter::{
            bytecode::{Bytecode, Load, Store},
            string::{IndexableStr, InternedStr, ValueStr},
            value::Value,
            FnBody, FnSignature, Interpreter,
        },
        span::{Span, SpanOf},
    };

    #[test]
    fn basic_function() {
        #[rustfmt::skip]
        let bytecode = [
            Bytecode::Add { dst: Store::Local(0), src0: Load::Local(1), src1: Load::Local(2) },
            Bytecode::Return,
        ];
        let signature = Rc::new(FnSignature {
            arity: 2,
            variadic: false,
            body: FnBody::Bytecode(bytecode.map(|bc| SpanOf(Span::default(), bc)).to_vec()),
            capture_locations: vec![],
            parent_capture_indices: vec![],
        });
        let mut interpreter = Interpreter::default();
        let function = Rc::new(interpreter.create_function(signature).unwrap());
        let result = interpreter
            .call_and_return(function, [Value::Number(1.0), Value::Number(2.0)])
            .unwrap();
        println!("{}", result);
        match result {
            Value::Number(n) => assert_eq!(n, 3.0),
            _ => panic!("Invalid type"),
        }
    }
    #[test]
    fn fibonacci_iterative() {
        #[rustfmt::skip]
        let bytecode = [
            Bytecode::Copy { dst: Store::Local(2), src: Load::Number(0.0) },
            Bytecode::Copy { dst: Store::Local(3), src: Load::Number(1.0) },
            Bytecode::Copy { dst: Store::Local(4), src: Load::Number(0.0) },
            // While start
            Bytecode::BrGe { offset: 6, src0: Load::Local(4), src1: Load::Local(1) },
            Bytecode::Add { dst: Store::Local(5), src0: Load::Local(2), src1: Load::Local(3) },
            Bytecode::Copy { dst: Store::Local(2), src: Load::Local(3) },
            Bytecode::Copy { dst: Store::Local(3), src: Load::Local(5) },
            Bytecode::Add { dst: Store::Local(4), src0: Load::Local(4), src1: Load::Number(1.0) },
            Bytecode::Jump(-5),
            // While end
            Bytecode::Truncate(5),
            Bytecode::Copy { dst: Store::Local(0), src: Load::Local(2) },
            Bytecode::Return,
        ];
        let signature = Rc::new(FnSignature {
            arity: 1,
            variadic: false,
            capture_locations: vec![],
            parent_capture_indices: vec![],
            body: FnBody::Bytecode(bytecode.map(|bc| SpanOf(Span::default(), bc)).to_vec()),
        });
        let mut interpreter = Interpreter::default();
        let function = Rc::new(interpreter.create_function(signature).unwrap());
        let mut a = 0.0;
        let mut b = 1.0;
        for i in 0..=100 {
            let result = interpreter
                .call_and_return(function.clone(), [Value::Number(i as f64)])
                .unwrap();
            println!("{}: {}", i, result);
            match result {
                Value::Number(num) => assert_eq!(num, a),
                _ => panic!("Invalid type"),
            }
            let c = a + b;
            a = b;
            b = c;
        }
    }
    #[test]
    fn fibonacci_recursive() {
        let name = InternedStr::from(IndexableStr::from("fib"));

        #[rustfmt::skip]
        let bytecode = [
            Bytecode::BrLe { offset: 7, src0: Load::Local(1), src1: Load::Number(1.0) },
            Bytecode::Sub { dst: Store::Local(3), src0: Load::Local(1), src1: Load::Number(1.0) },
            Bytecode::Call { src: Load::Global(name), arity: 1 },
            Bytecode::Sub { dst: Store::Local(4), src0: Load::Local(1), src1: Load::Number(2.0) },
            Bytecode::Call { src: Load::Global(name), arity: 1 },
            Bytecode::Add { dst: Store::Local(0), src0: Load::Local(2), src1: Load::Local(3) },
            Bytecode::Return,
            Bytecode::Copy { dst: Store::Local(0), src: Load::Local(1) },
            Bytecode::Return,
        ];
        let signature = Rc::new(FnSignature {
            arity: 1,
            capture_locations: vec![],
            parent_capture_indices: vec![],
            variadic: false,
            body: FnBody::Bytecode(bytecode.map(|bc| SpanOf(Span::default(), bc)).to_vec()),
        });
        let mut interpreter = Interpreter::default();
        let function = Rc::new(interpreter.create_function(signature).unwrap());
        interpreter
            .set_global(ValueStr::Interned(name), Value::Function(function.clone()))
            .unwrap();

        let mut a = 0.0;
        let mut b = 1.0;

        for i in 0..=30 {
            let result = interpreter
                .call_and_return(function.clone(), [Value::Number(i as f64)])
                .unwrap();
            println!("{}: {}", i, result);
            match result {
                Value::Number(n) => assert_eq!(n, a),
                _ => panic!("Invalid type"),
            }
            let c = a + b;
            a = b;
            b = c;
        }
    }
    #[test]
    fn upvalue_test() {
        let inc_name = InternedStr::from(IndexableStr::from("inc"));
        let dec_name = InternedStr::from(IndexableStr::from("dec"));

        #[rustfmt::skip]
        let inc_bytecode = [
            Bytecode::Add { dst: Store::Upvalue(0), src0: Load::Upvalue(0), src1: Load::Number(1.0) },
            Bytecode::Copy { dst: Store::Local(0), src: Load::Upvalue(0) },
            Bytecode::Return,
        ];
        let inc_signature = Rc::new(FnSignature {
            arity: 0,
            variadic: false,
            capture_locations: vec![1],
            parent_capture_indices: vec![],
            body: FnBody::Bytecode(inc_bytecode.map(|bc| SpanOf(Span::default(), bc)).to_vec()),
        });
        #[rustfmt::skip]
        let dec_bytecode = [
            Bytecode::Sub { dst: Store::Upvalue(0), src0: Load::Upvalue(0), src1: Load::Number(1.0) },
            Bytecode::Copy { dst: Store::Local(0), src: Load::Upvalue(0) },
            Bytecode::Return,
        ];
        let dec_signature = Rc::new(FnSignature {
            arity: 0,
            variadic: false,
            capture_locations: vec![1],
            parent_capture_indices: vec![],
            body: FnBody::Bytecode(dec_bytecode.map(|bc| SpanOf(Span::default(), bc)).to_vec()),
        });
        #[rustfmt::skip]
        let bytecode = [
            Bytecode::Copy { dst: Store::Local(1), src: Load::Number(0.0) },
            Bytecode::Copy { dst: Store::Local(0), src: Load::Object(2) },
            Bytecode::StoreProperty { dst: Load::Local(0), prop: inc_name, src: Load::Function(inc_signature.clone()) },
            Bytecode::StoreProperty { dst: Load::Local(0), prop: dec_name, src: Load::Function(dec_signature.clone()) },
            Bytecode::Return,
        ];
        let signature = Rc::new(FnSignature {
            arity: 0,
            variadic: false,
            capture_locations: vec![],
            parent_capture_indices: vec![],
            body: FnBody::Bytecode(bytecode.map(|bc| SpanOf(Span::default(), bc)).to_vec()),
        });
        let mut interpreter = Interpreter::default();
        let function = Rc::new(interpreter.create_function(signature).unwrap());
        let result = interpreter.call_and_return(function, []).unwrap();

        let inc = result
            .get_property(&Value::String(ValueStr::Interned(inc_name)))
            .unwrap()
            .as_callable()
            .unwrap();
        let dec = result
            .get_property(&Value::String(ValueStr::Interned(dec_name)))
            .unwrap()
            .as_callable()
            .unwrap();
        assert_eq!(
            interpreter.call_and_return(inc, []).unwrap(),
            Value::Number(1.0)
        );
        println!("{}", result);
        assert_eq!(
            interpreter.call_and_return(dec, []).unwrap(),
            Value::Number(0.0)
        );
        println!("{}", result);
    }
}
