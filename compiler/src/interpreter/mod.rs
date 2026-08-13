use core::fmt;
use std::cell::RefCell;
use std::mem::{self, replace};
use std::rc::Rc;
use std::sync::atomic::Ordering;

use crate::error::ErrorKind;
use crate::interpreter::string::ValueStr;
use crate::interpreter::{bytecode::Bytecode, value::Function, value::Value};
use crate::span::SpanOf;
use crate::DEBUG_MODE;
use rustc_hash::FxHashMap;

pub mod builtin;
pub mod bytecode;
pub mod string;
pub mod value;

struct FunctionFrame {
    base_pointer: usize,
    base_stack: usize,
    function: Rc<Function>,
}

#[derive(Debug)]
pub enum UpvalueLoc {
    Local(usize),  // Get upvalue from parent frame's local memory
    Shared(usize), // Get upvalue from parent frame's upvalue storage
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
    Builtin(Box<RefCell<dyn FnMut(&mut Interpreter) -> Result<Value, ErrorKind>>>),
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
            Self::Builtin(..) => f.debug_tuple("Builtin").field(&"..").finish(),
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

const INIT_MEM_SIZE: usize = 0x10000;

pub struct Interpreter {
    memory: Vec<Cell>,
    stack: Vec<Value>,
    current_frame: Option<FunctionFrame>,
    globals: FxHashMap<ValueStr, (Value, bool)>, // true - read-only
}
impl Default for Interpreter {
    fn default() -> Self {
        let globals = builtin::GLOBALS.with(|globals| {
            globals
                .iter()
                .map(|(name, function)| (name.clone(), (Value::Function(function.clone()), true)))
                .collect()
        });
        Self {
            memory: Vec::with_capacity(INIT_MEM_SIZE),
            stack: Vec::new(),
            current_frame: None,
            globals,
        }
    }
}
impl Interpreter {
    fn base_stack(&self) -> usize {
        self.current_frame
            .as_ref()
            .map(|f| f.base_stack)
            .unwrap_or(0)
    }
    fn push_stack(&mut self, value: Value) {
        self.stack.push(value);
    }
    fn pop_stack(&mut self) -> Value {
        if self.stack.len() <= self.base_stack() {
            return Value::Nil;
        }
        self.stack.pop().unwrap_or_default()
    }
    fn get_local(&self, id: usize) -> Value {
        let absolute_id = self.base_pointer() + id as usize;
        match self.memory.get(absolute_id) {
            Some(Cell::Value(v)) => v.clone(),
            Some(Cell::Upvalue(up)) => up.borrow().clone(),
            None => Value::Nil,
        }
    }
    fn set_local(&mut self, id: usize, new_value: Value) {
        let index = self.base_pointer() + id as usize;
        if index >= self.memory.len() {
            self.memory.resize_with(index + 1, Cell::default);
        }
        match &mut self.memory[index] {
            Cell::Upvalue(up) => *up.borrow_mut() = new_value,
            Cell::Value(value) => *value = new_value,
        }
    }
    fn make_local_upvalue(&mut self, id: usize) -> Rc<RefCell<Value>> {
        let index = self.base_pointer() + id as usize;
        if self.memory.len() <= index {
            self.memory.resize_with(index + 1, Cell::default);
        }
        let cell = &mut self.memory[index];
        match cell {
            Cell::Upvalue(val) => val.clone(),
            Cell::Value(val) => {
                let shared = Rc::new(RefCell::new(val.clone()));
                *cell = Cell::Upvalue(shared.clone());
                shared
            }
        }
    }
    fn get_upvalue(&self, id: usize) -> Value {
        let fun = self.current_frame.as_ref().unwrap().function.as_ref();
        fun.upvalues[id].borrow().clone()
    }
    fn set_upvalue(&self, id: usize, new_value: Value) {
        let fun = self.current_frame.as_ref().unwrap().function.as_ref();
        *fun.upvalues[id].borrow_mut() = new_value;
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
    fn truncate(&mut self, new_len: usize) {
        self.memory.truncate(new_len + self.base_pointer());
    }
    fn base_pointer(&self) -> usize {
        self.current_frame
            .as_ref()
            .map(|frame| frame.base_pointer)
            .unwrap_or(0)
    }
    fn method_currying(&mut self, itself: Value, function: Rc<Function>) -> Function {
        let function1 = function.clone();
        let curried_method = move |interpreter: &mut Self| -> Result<Value, ErrorKind> {
            // shift all arguments forward, while inserting itself
            let current_frame = interpreter.current_frame.as_ref().unwrap();
            let arity = current_frame.function.signature.required_arity();
            let base_pointer = current_frame.base_pointer;

            interpreter.set_local(arity, itself.clone());
            interpreter.memory[base_pointer..].rotate_right(1);
            interpreter.call_with_frame(FunctionFrame {
                base_pointer,
                base_stack: interpreter.stack.len(),
                function: function1.clone(),
            })
        };
        Self::create_builtin_function(
            function.signature.arity + 1,
            function.signature.variadic,
            curried_method,
        )
    }
    pub fn create_function(&mut self, signature: Rc<FnSignature>) -> Function {
        let upvalues = signature
            .upvalues
            .iter()
            .map(|loc| match loc {
                UpvalueLoc::Shared(id) => {
                    self.current_frame.as_ref().unwrap().function.upvalues[*id].clone()
                }
                UpvalueLoc::Local(id) => self.make_local_upvalue(*id),
            })
            .collect::<Vec<_>>();
        Function {
            signature,
            upvalues,
        }
    }
    pub fn create_builtin_function(
        arity: usize,
        variadic: bool,
        builtin: impl FnMut(&mut Interpreter) -> Result<Value, ErrorKind> + 'static,
    ) -> Function {
        Function {
            signature: Rc::new(FnSignature {
                arity,
                variadic,
                upvalues: vec![],
                body: FnBody::Builtin(Box::new(RefCell::new(builtin))),
            }),
            upvalues: vec![],
        }
    }
    fn call_with_frame(&mut self, frame: FunctionFrame) -> Result<Value, ErrorKind> {
        let function = frame.function.clone();
        let mut old_frame = Some(frame);
        mem::swap(&mut old_frame, &mut self.current_frame);

        let return_value = match &function.signature.body {
            FnBody::Builtin(builtin) => builtin.borrow_mut()(self)?,
            FnBody::Bytecode(bytecodes) => {
                let mut index = 0;
                loop {
                    let Some(bc) = bytecodes.get(index) else {
                        break Value::Nil;
                    };
                    if DEBUG_MODE.load(Ordering::Relaxed) {
                        println!(
                            "{:?}: [{}]",
                            bc.1,
                            self.stack
                                .iter()
                                .map(|v| v.to_string())
                                .collect::<Vec<_>>()
                                .join(", ")
                        );
                    }
                    match bc.1.interpret(self, index)? {
                        Ok(next) => index = next,
                        Err(ret) => break ret,
                    }
                }
            }
        };

        self.truncate(0);
        self.current_frame = old_frame;

        Ok(return_value)
    }
    fn call_stack_args(
        &mut self,
        function: Rc<Function>,
        stack_base: usize,
    ) -> Result<Value, ErrorKind> {
        let abs_stack = stack_base + self.base_stack();
        let abs_ptr = self.memory.len();
        let stack_len = self.stack.len();
        let arity = function.signature.arity;
        let variadic = function.signature.variadic;

        let iter = self.stack[abs_stack..(abs_stack + arity).min(stack_len)]
            .iter_mut()
            .map(|elem| Cell::Value(replace(elem, Value::Nil)))
            .chain(std::iter::repeat_with(Cell::default))
            .take(arity);
        self.memory.extend(iter);

        if variadic {
            let array = Value::Array(Rc::new(RefCell::new(
                self.stack[(abs_stack + arity)..]
                    .iter_mut()
                    .map(|elem| replace(elem, Value::Nil))
                    .collect::<Vec<_>>(),
            )));
            self.memory.push(Cell::Value(array));
        }
        self.stack.truncate(abs_stack);

        self.call_with_frame(FunctionFrame {
            base_pointer: abs_ptr,
            base_stack: abs_stack,
            function,
        })
    }
    fn call_on_stack(&mut self, stack_base: usize) -> Result<Value, ErrorKind> {
        let abs_stack = stack_base + self.base_stack();
        let function = self.stack[abs_stack].try_function()?;
        self.call_stack_args(function, stack_base + 1).inspect(|_| {
            self.stack.truncate(abs_stack);
        })
    }
    pub fn call_function_args(
        &mut self,
        function: Rc<Function>,
        args: impl IntoIterator<Item = Value>,
    ) -> Result<Value, ErrorKind> {
        let next_base = self.stack.len() - self.base_stack();
        for arg in args {
            self.stack.push(arg);
        }
        self.call_stack_args(function, next_base)
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use crate::{
        interpreter::{
            bytecode::{BinaryOp, Bytecode},
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
            Bytecode::LoadLocal(0),
            Bytecode::LoadLocal(1),
            Bytecode::Binary(BinaryOp::Add),
            Bytecode::Return,
        ];
        let signature = Rc::new(FnSignature {
            arity: 2,
            variadic: false,
            body: FnBody::Bytecode(bytecode.map(|bc| SpanOf(Span::default(), bc)).to_vec()),
            upvalues: vec![],
        });
        let mut interpreter = Interpreter::default();
        let function = Rc::new(interpreter.create_function(signature));
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
            Bytecode::LoadNum(0.0),
            Bytecode::LoadNum(1.0),
            Bytecode::StoreLocal(2),
            Bytecode::StoreLocal(1),

            // while start
            Bytecode::LoadLocal(0),
            Bytecode::LoadNum(0.0),
            Bytecode::Binary(BinaryOp::SetGt),
            Bytecode::BranchIf(false, 12),

            Bytecode::LoadLocal(1),
            Bytecode::LoadLocal(2),
            Bytecode::Binary(BinaryOp::Add),

            Bytecode::LoadLocal(2),
            Bytecode::StoreLocal(1),
            Bytecode::StoreLocal(2),

            Bytecode::LoadLocal(0),
            Bytecode::LoadNum(1.0),
            Bytecode::Binary(BinaryOp::Sub),
            Bytecode::StoreLocal(0),

            Bytecode::Jump(-14),
            // While end
            Bytecode::LoadLocal(1),
            Bytecode::Return,
        ];
        let signature = Rc::new(FnSignature {
            arity: 1,
            variadic: false,
            upvalues: vec![],
            body: FnBody::Bytecode(bytecode.map(|bc| SpanOf(Span::default(), bc)).to_vec()),
        });
        let mut interpreter = Interpreter::default();
        let function = Rc::new(interpreter.create_function(signature));
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
            Bytecode::LoadLocal(0),
            Bytecode::LoadNum(1.0),
            Bytecode::Binary(BinaryOp::SetLe),
            Bytecode::BranchIf(true, 13),

            Bytecode::LoadGlobal(name.clone()),
            Bytecode::LoadLocal(0),
            Bytecode::LoadNum(1.0),
            Bytecode::Binary(BinaryOp::Sub),
            Bytecode::Call(0),

            Bytecode::LoadGlobal(name.clone()),
            Bytecode::LoadLocal(0),
            Bytecode::LoadNum(2.0),
            Bytecode::Binary(BinaryOp::Sub),
            Bytecode::Call(1),

            Bytecode::Binary(BinaryOp::Add),
            Bytecode::Return,

            Bytecode::LoadLocal(0),
            Bytecode::Return,
        ];
        let signature = Rc::new(FnSignature {
            arity: 1,
            upvalues: vec![],
            variadic: false,
            body: FnBody::Bytecode(bytecode.map(|bc| SpanOf(Span::default(), bc)).to_vec()),
        });
        let mut interpreter = Interpreter::default();
        let function = Rc::new(interpreter.create_function(signature));
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
            Bytecode::LoadUpvalue(0),
            Bytecode::LoadNum(1.0),
            Bytecode::Binary(BinaryOp::Add),
            Bytecode::Dup(2),
            Bytecode::StoreUpvalue(0),
            Bytecode::Return,
        ];
        let inc_signature = Rc::new(FnSignature {
            arity: 0,
            variadic: false,
            upvalues: vec![UpvalueLoc::Local(0)],
            body: FnBody::Bytecode(inc_bytecode.map(|bc| SpanOf(Span::default(), bc)).to_vec()),
        });
        #[rustfmt::skip]
        let dec_bytecode = [
            Bytecode::LoadUpvalue(0),
            Bytecode::LoadNum(1.0),
            Bytecode::Binary(BinaryOp::Sub),
            Bytecode::Dup(2),
            Bytecode::StoreUpvalue(0),
            Bytecode::Return,
        ];
        let dec_signature = Rc::new(FnSignature {
            arity: 0,
            variadic: false,
            upvalues: vec![UpvalueLoc::Local(0)],
            body: FnBody::Bytecode(dec_bytecode.map(|bc| SpanOf(Span::default(), bc)).to_vec()),
        });
        #[rustfmt::skip]
        let bytecode = [
            Bytecode::LoadNum(0.0),
            Bytecode::StoreLocal(0),
            Bytecode::LoadStr(inc_name.clone()),
            Bytecode::LoadFn(inc_signature.clone()),
            Bytecode::LoadStr(dec_name.clone()),
            Bytecode::LoadFn(dec_signature.clone()),
            Bytecode::StackToObj(0),
            Bytecode::Return,
        ];
        let signature = Rc::new(FnSignature {
            arity: 0,
            variadic: false,
            upvalues: vec![],
            body: FnBody::Bytecode(bytecode.map(|bc| SpanOf(Span::default(), bc)).to_vec()),
        });
        let mut interpreter = Interpreter::default();
        let function = Rc::new(interpreter.create_function(signature));
        let result = interpreter.call_function_args(function, []).unwrap();

        let inc = result
            .get_property(&Value::String(inc_name))
            .unwrap()
            .try_function()
            .unwrap();
        let dec = result
            .get_property(&Value::String(dec_name))
            .unwrap()
            .try_function()
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
