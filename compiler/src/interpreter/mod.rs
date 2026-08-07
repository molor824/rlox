use core::fmt;
use std::cell::RefCell;
use std::mem;
use std::rc::Rc;

use crate::error::ErrorKind;
use crate::interpreter::string::ValueStr;
use crate::interpreter::{bytecode::Bytecode, value::Function, value::Value};
use crate::span::SpanOf;
use rustc_hash::FxHashMap;

mod builtin;
pub mod bytecode;
pub mod string;
pub mod value;

pub type LocalId = u32;

struct FunctionFrame {
    base_pointer: usize,
    function: Rc<Function>,
}

#[derive(Debug)]
pub enum UpvalueLoc {
    Local(LocalId),  // Get upvalue from parent frame's local memory
    Shared(LocalId), // Get upvalue from parent frame's upvalue storage
}

#[derive(Debug)]
pub struct FnSignature {
    pub arity: usize,   // NOTE: arity EXCLUDES variadic parameter!
    pub variadic: bool, // if true, function has variadic parameter.
    pub upvalues: Vec<UpvalueLoc>,
    pub body: FnBody,
}
impl FnSignature {
    fn required_arity(&self) -> usize {
        self.arity + if self.variadic { 1 } else { 0 }
    }
}

pub enum FnBody {
    Bytecode(Vec<SpanOf<Bytecode>>),
    Builtin(Box<dyn Fn(&mut Interpreter) -> Result<Value, ErrorKind>>),
}
impl fmt::Debug for FnBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bytecode(code) => {
                writeln!(f, "Bytecodes[")?;
                for bc in code {
                    writeln!(f, "  {}", format!("{:?}", bc.1).replace("\n", "\n  "))?;
                }
                write!(f, "]")
            }
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
    globals: FxHashMap<ValueStr, (Value, bool)>, // true - read-only
}
impl Default for Interpreter {
    fn default() -> Self {
        const STACK_SIZE: usize = 0x10000;
        let globals = builtin::GLOBALS
            .iter()
            .cloned()
            .map(|(name, sig)| {
                (
                    name,
                    (
                        Value::Function(Rc::new(Function {
                            signature: sig,
                            upvalues: vec![],
                        })),
                        true,
                    ),
                )
            })
            .collect::<FxHashMap<_, _>>();
        Self {
            memory: Vec::with_capacity(STACK_SIZE),
            current_frame: None,
            globals,
        }
    }
}
impl Interpreter {
    fn get_local(&self, id: LocalId) -> Value {
        let absolute_id = self.base_pointer() + id as usize;
        match self.memory.get(absolute_id) {
            Some(Cell::Value(v)) => v.clone(),
            Some(Cell::Upvalue(up)) => up.borrow().clone(),
            None => Value::Nil,
        }
    }
    fn set_local(&mut self, id: LocalId, new_value: Value) -> Result<(), ErrorKind> {
        let index = self.base_pointer() + id as usize;
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
        let index = self.base_pointer() + id as usize;
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
    fn get_upvalue(&self, id: LocalId) -> Result<Value, ErrorKind> {
        let fun = self
            .current_frame
            .as_ref()
            .ok_or(ErrorKind::LocalAccessInGlobal)?
            .function
            .as_ref();
        Ok(fun
            .upvalues
            .get(id as usize)
            .map(|upvalue| upvalue.borrow().clone())
            .unwrap_or_default())
    }
    fn set_upvalue(&self, id: LocalId, new_value: Value) -> Result<(), ErrorKind> {
        let fun = self
            .current_frame
            .as_ref()
            .ok_or(ErrorKind::LocalAccessInGlobal)?
            .function
            .as_ref();
        if let Some(v) = fun.upvalues.get(id as usize) {
            *v.borrow_mut() = new_value;
            Ok(())
        } else {
            Err(ErrorKind::InvalidUpvalueAccess)
        }
    }
    fn make_global_read_only(&mut self, name: ValueStr) {
        self.globals.get_mut(&name).map(|global| global.1 = true);
    }
    fn get_global(&self, name: ValueStr) -> Value {
        self.globals.get(&name).cloned().unwrap_or_default().0
    }
    fn set_global(&mut self, name: ValueStr, new_value: Value) -> Result<(), ErrorKind> {
        match self.globals.get_mut(&name) {
            Some((_, true)) => Err(ErrorKind::ConstGlobal(name)),
            Some((value, _)) => {
                *value = new_value;
                Ok(())
            }
            None => Err(ErrorKind::UndeclaredGlobal(name)),
        }
    }
    fn declare_global(&mut self, name: ValueStr) -> Result<(), ErrorKind> {
        if self.globals.contains_key(&name) {
            return Err(ErrorKind::RedeclareGlobal(name));
        }
        self.globals.insert(name, (Value::Nil, false));
        Ok(())
    }
    fn truncate(&mut self, new_len: usize) -> Result<(), ErrorKind> {
        if new_len < self.base_pointer() {
            return Err(ErrorKind::StackUnderflow);
        }
        self.memory.truncate(new_len);
        Ok(())
    }
    fn base_pointer(&self) -> usize {
        self.current_frame
            .as_ref()
            .map(|frame| frame.base_pointer)
            .unwrap_or(0)
    }
    fn method_currying(
        &mut self,
        itself: Value,
        function: Rc<Function>,
    ) -> Result<Function, ErrorKind> {
        let function1 = function.clone();
        let curried_method = move |interpreter: &mut Self| -> Result<Value, ErrorKind> {
            // shift all arguments forward, while inserting itself
            let current_frame = interpreter
                .current_frame
                .as_ref()
                .ok_or(ErrorKind::LocalAccessInGlobal)?;
            let arity = current_frame.function.signature.required_arity();
            let base_pointer = current_frame.base_pointer;

            interpreter.set_local(arity as LocalId, itself.clone())?;
            interpreter.memory[base_pointer..].rotate_right(1);
            interpreter.call_function_unchecked(function.clone(), base_pointer)
        };
        self.create_function(Rc::new(FnSignature {
            body: FnBody::Builtin(Box::new(curried_method)),
            arity: function1.signature.arity + 1,
            variadic: function1.signature.variadic,
            upvalues: vec![],
        }))
    }
    fn create_function(&mut self, signature: Rc<FnSignature>) -> Result<Function, ErrorKind> {
        let upvalues = signature
            .upvalues
            .iter()
            .map(|loc| match loc {
                UpvalueLoc::Shared(id) => Ok(self
                    .current_frame
                    .as_ref()
                    .ok_or(ErrorKind::LocalAccessInGlobal)?
                    .function
                    .upvalues[*id as usize]
                    .clone()),
                UpvalueLoc::Local(id) => self.make_local_upvalue(*id),
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Function {
            signature,
            upvalues,
        })
    }
    fn call_function_unchecked(
        &mut self,
        function: Rc<Function>,
        base_pointer: usize,
    ) -> Result<Value, ErrorKind> {
        let mut old_frame = Some(FunctionFrame {
            base_pointer,
            function: function.clone(),
        });
        mem::swap(&mut old_frame, &mut self.current_frame);

        let return_value = match &function.signature.body {
            FnBody::Builtin(builtin) => builtin(self)?,
            FnBody::Bytecode(bytecodes) => {
                let mut index = 0;
                loop {
                    match bytecodes[index].1.interpret(self, index)? {
                        Ok(next) => index = next,
                        Err(ret) => break ret,
                    }
                }
            }
        };

        self.truncate(base_pointer)?;
        self.current_frame = old_frame;

        Ok(return_value)
    }
    fn call_function(&mut self, function: Rc<Function>, base: LocalId) -> Result<Value, ErrorKind> {
        let base_pointer = self.base_pointer() + base as usize;
        let arity = self.memory.len() - base_pointer;
        let signature = function.signature.as_ref();
        if signature.variadic {
            // additional arguments are all combined into list
            let array = (signature.arity..arity)
                .map(|i| match &self.memory[base_pointer + i] {
                    // Upvalue is generally not allowed as function argument, but if it does happen, just clone the value
                    Cell::Upvalue(shared) => shared.borrow().clone(),
                    Cell::Value(value) => value.clone(),
                })
                .collect::<Vec<_>>();
            let variadic = Value::Array(Rc::new(RefCell::new(array)));
            let new_index = base_pointer + signature.arity;
            if new_index >= self.memory.len() {
                self.memory.resize_with(new_index + 1, Cell::default);
            }
            self.memory[new_index] = Cell::Value(variadic);
        }
        // Truncate until it's no longer past the expected arity
        self.memory
            .truncate(base_pointer + signature.required_arity());

        self.call_function_unchecked(function, base_pointer)
    }
    pub fn call_function_args(
        &mut self,
        function: Rc<Function>,
        args: impl IntoIterator<Item = Value>,
    ) -> Result<Value, ErrorKind> {
        let base = self.memory.len() - self.base_pointer();
        for arg in args {
            self.memory.push(Cell::Value(arg));
        }
        self.call_function(function, base as LocalId)
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use crate::{
        interpreter::{
            bytecode::{Bytecode, Load, Store},
            string::ValueStr,
            value::Value,
            FnBody, FnSignature, Interpreter, UpvalueLoc,
        },
        span::{Span, SpanOf},
    };

    #[test]
    fn basic_function() {
        #[rustfmt::skip]
        let bytecode = [
            Bytecode::Add { dst: Store::Local(2), src0: Load::Local(0), src1: Load::Local(1) },
            Bytecode::Return(Load::Local(2)),
        ];
        let signature = Rc::new(FnSignature {
            arity: 2,
            variadic: false,
            body: FnBody::Bytecode(bytecode.map(|bc| SpanOf(Span::default(), bc)).to_vec()),
            upvalues: vec![],
        });
        let mut interpreter = Interpreter::default();
        let function = Rc::new(interpreter.create_function(signature).unwrap());
        let result = interpreter
            .call_function_args(function, [Value::Number(1.0), Value::Number(2.0)])
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
            Bytecode::Move { dst: Store::Local(1), src: Load::Number(0.0) },
            Bytecode::Move { dst: Store::Local(2), src: Load::Number(1.0) },
            Bytecode::Move { dst: Store::Local(3), src: Load::Number(0.0) },
            // While start
            Bytecode::BrGe { offset: 6, src0: Load::Local(3), src1: Load::Local(0) },
            Bytecode::Add { dst: Store::Local(4), src0: Load::Local(1), src1: Load::Local(2) },
            Bytecode::Move { dst: Store::Local(1), src: Load::Local(2) },
            Bytecode::Move { dst: Store::Local(2), src: Load::Local(4) },
            Bytecode::Add { dst: Store::Local(3), src0: Load::Local(3), src1: Load::Number(1.0) },
            Bytecode::Jump(-5),
            // While end
            Bytecode::Return(Load::Local(1)),
        ];
        let signature = Rc::new(FnSignature {
            arity: 1,
            variadic: false,
            upvalues: vec![],
            body: FnBody::Bytecode(bytecode.map(|bc| SpanOf(Span::default(), bc)).to_vec()),
        });
        let mut interpreter = Interpreter::default();
        let function = Rc::new(interpreter.create_function(signature).unwrap());
        let mut a = 0.0;
        let mut b = 1.0;
        for i in 0..=100 {
            let result = interpreter
                .call_function_args(function.clone(), [Value::Number(i as f64)])
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
        let name = ValueStr::interned("fib");

        #[rustfmt::skip]
        let bytecode = [
            Bytecode::BrLe { offset: 7, src0: Load::Local(0), src1: Load::Number(1.0) },
            Bytecode::Sub { dst: Store::Local(1), src0: Load::Local(0), src1: Load::Number(1.0) },
            Bytecode::Call { src: Load::Global(name.clone()), dst: Store::Local(1), base: 1 },
            Bytecode::Sub { dst: Store::Local(2), src0: Load::Local(0), src1: Load::Number(2.0) },
            Bytecode::Call { src: Load::Global(name.clone()), dst: Store::Local(2), base: 2 },
            Bytecode::Add { dst: Store::Local(3), src0: Load::Local(1), src1: Load::Local(2) },
            Bytecode::Return(Load::Local(3)),
            Bytecode::Return(Load::Local(0)),
        ];
        let signature = Rc::new(FnSignature {
            arity: 1,
            upvalues: vec![],
            variadic: false,
            body: FnBody::Bytecode(bytecode.map(|bc| SpanOf(Span::default(), bc)).to_vec()),
        });
        let mut interpreter = Interpreter::default();
        let function = Rc::new(interpreter.create_function(signature).unwrap());
        interpreter.declare_global(name.clone()).unwrap();
        interpreter
            .set_global(name, Value::Function(function.clone()))
            .unwrap();

        let mut a = 0.0;
        let mut b = 1.0;

        for i in 0..=20 {
            let result = interpreter
                .call_function_args(function.clone(), [Value::Number(i as f64)])
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
        let inc_name = ValueStr::interned("inc");
        let dec_name = ValueStr::interned("dec");

        #[rustfmt::skip]
        let inc_bytecode = [
            Bytecode::Add { dst: Store::Upvalue(0), src0: Load::Upvalue(0), src1: Load::Number(1.0) },
            Bytecode::Return(Load::Upvalue(0)),
        ];
        let inc_signature = Rc::new(FnSignature {
            arity: 0,
            variadic: false,
            upvalues: vec![UpvalueLoc::Local(1)],
            body: FnBody::Bytecode(inc_bytecode.map(|bc| SpanOf(Span::default(), bc)).to_vec()),
        });
        #[rustfmt::skip]
        let dec_bytecode = [
            Bytecode::Sub { dst: Store::Upvalue(0), src0: Load::Upvalue(0), src1: Load::Number(1.0) },
            Bytecode::Return(Load::Upvalue(0)),
        ];
        let dec_signature = Rc::new(FnSignature {
            arity: 0,
            variadic: false,
            upvalues: vec![UpvalueLoc::Local(1)],
            body: FnBody::Bytecode(dec_bytecode.map(|bc| SpanOf(Span::default(), bc)).to_vec()),
        });
        #[rustfmt::skip]
        let bytecode = [
            Bytecode::Move { dst: Store::Local(0), src: Load::Object(2) },
            Bytecode::Move { dst: Store::Local(1), src: Load::Number(0.0) },
            Bytecode::StoreProperty { dst: Load::Local(0), prop: inc_name.clone(), src: Load::Function(inc_signature.clone()) },
            Bytecode::StoreProperty { dst: Load::Local(0), prop: dec_name.clone(), src: Load::Function(dec_signature.clone()) },
            Bytecode::Return(Load::Local(0)),
        ];
        let signature = Rc::new(FnSignature {
            arity: 0,
            variadic: false,
            upvalues: vec![],
            body: FnBody::Bytecode(bytecode.map(|bc| SpanOf(Span::default(), bc)).to_vec()),
        });
        let mut interpreter = Interpreter::default();
        let function = Rc::new(interpreter.create_function(signature).unwrap());
        let result = interpreter.call_function_args(function, []).unwrap();

        let inc = result
            .get_property(&Value::String(inc_name))
            .unwrap()
            .as_callable()
            .unwrap();
        let dec = result
            .get_property(&Value::String(dec_name))
            .unwrap()
            .as_callable()
            .unwrap();
        assert_eq!(
            interpreter.call_function_args(inc, []).unwrap(),
            Value::Number(1.0)
        );
        println!("{}", result);
        assert_eq!(
            interpreter.call_function_args(dec, []).unwrap(),
            Value::Number(0.0)
        );
        println!("{}", result);
    }
}
