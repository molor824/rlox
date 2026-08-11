use rustc_hash::FxHashMap;

use crate::error::ErrorKind;
use crate::interpreter::string::ValueStr;
use crate::interpreter::value::{Function, Object, Value};
use crate::interpreter::{Cell, FnSignature, Interpreter};
use std::cell::RefCell;
use std::mem::replace;
use std::rc::Rc;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Pow,
    Shl,
    Shr,
    Sha,
    BitAnd,
    BitOr,
    BitXor,
    SetEq,
    SetNe,
    SetLt,
    SetLe,
    SetGt,
    SetGe,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Negate,
    Swap,
    SetTrue,
    SetFalse,
}
// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// pub enum BranchCond {
//     False,
//     True,
//     Eq,
//     Ne,
//     Lt,
//     Le,
//     Gt,
//     Ge,
// }

#[derive(Debug, Clone)]
#[rustfmt::skip]
/// Bytecode for the language. It assumes a linear memory made up of cell that can accept any value.
/// Constants, and globals have their own unique IDs so from the codegen perspective, global and constant identifiers needs to be interned before being used.
///
/// Operational instructions only access the local memory, where it's relative to the base function pointer.
/// The memory automatically grows if the memory index is past the stack pointer.
pub enum Bytecode {
    Nop,
    Dup(usize), // s0 -> [s0; .0]
    // Binary operations
    Binary(BinaryOp), // s0, s1 -> <BINARY> s0 s1
    // Unary operations
    Unary(UnaryOp), // s0 -> <UNARY> s0
    // Branching operations
    BranchIf(bool, isize), // s0 -> if s0 == .0 then <JMP> .1 else <NOP>;
    // Global memory
    GlobalDeclare(ValueStr), // declare global
    GlobalReadOnly(ValueStr), // make global readonly
    LoadGlobal(ValueStr), // () -> <GET_GLOBAL> .0
    StoreGlobal(ValueStr), // s0 -> <SET_GLOBAL> .0 s0;
    // Local memory
    Truncate(usize), // truncate till current length
    LoadLocal(usize), // () -> <LOAD_LOCAL> .0
    StoreLocal(usize), // s0 -> <STORE_LOCAL> .0 s0;
    // Upvalue
    LoadUpvalue(usize), // () -> <LOAD_UPVALUE> .0
    StoreUpvalue(usize), // s0 -> <STORE_UPVALUE> .0 s0;
    // Property
    LoadProperty(ValueStr), // obj -> obj[.0]
    LoadPropertyIndirect, // obj, key -> obj[key]
    StoreProperty(ValueStr), // val, obj -> obj[.0] = val;
    StorePropertyIndirect, // val, obj, key -> obj[key] = val;
    LoadMethod(ValueStr), // obj -> (\... -> obj[key](obj[key], ...))
    // Array initialization
    StackToArray(usize), // starting at .0 offset: s0, s1, s2, ... -> [s0, s1, s2, ...]
    // Array initialization for unpacking, might introduce unpredictable stack
    AppendArray, // arr, v -> arr:append(v);
    ExtendArray, // arr, iter -> arr:extend(iter);
    // Object initialization
    StackToObj(usize), // starting at .0 offset: k0, v0, k1, v1, ... -> {k0: v0, k1: v1, ...}
    // Object initialization for unpacking, which introduces unpredictable stack
    AppendObj(ValueStr), // obj, val -> obj[.0] = val;
    AppendObjIndirect, // obj, key, val -> obj[key] = val;
    ExtendObj, // obj, iter -> obj:extend(iter);
    LoadNil,
    LoadBool(bool),
    LoadNum(f64),
    LoadFn(Rc<FnSignature>),
    LoadStr(ValueStr),
    // Jumping
    Jump(isize), // pc += .0
    // Function call
    Call(usize), // starting at .0 offset: func, p0, p1, p2, ... -> func(p0, p1, p2, ...)
    CallVariadic, // func, params -> func(*params)
    CallBuiltin(usize, Rc<Function>), // starting at .0 offset: p0, p1, p2, ... -> .1(p0, p1, p2, ...)
    // Return
    Return, // v0 -> return(v0);
}
impl Bytecode {
    // None -> return
    // Some(i) -> next instruction index
    pub fn interpret(
        &self,
        interpreter: &mut Interpreter,
        index: usize,
    ) -> Result<Result<usize, Value>, ErrorKind> {
        match self {
            Bytecode::Nop => {}
            Bytecode::Dup(n) => {
                let v = interpreter.pop_stack();
                for _ in 0..*n {
                    interpreter.push_stack(v.clone());
                }
            }
            Bytecode::ExtendArray => {
                let iter = interpreter.pop_stack();
                let array = interpreter.pop_stack().try_array()?;
                iter.try_iterate(interpreter, |_, v| Ok(array.borrow_mut().push(v)))?;
                interpreter.push_stack(Value::Array(array));
            }
            Bytecode::ExtendObj => {
                let iter = interpreter.pop_stack();
                let object = interpreter.pop_stack();
                iter.try_iterate(interpreter, |_, pair| {
                    let k = pair.get_property(&Value::Number(0.0))?;
                    let v = pair.get_property(&Value::Number(1.0))?;
                    object.set_property(k, v)?;
                    Ok(())
                })?;
                interpreter.push_stack(object);
            }
            Bytecode::AppendArray => {
                let value = interpreter.pop_stack();
                let array = interpreter.pop_stack().try_array()?;

                array.borrow_mut().push(value);
                interpreter.push_stack(Value::Array(array));
            }
            Bytecode::AppendObj(str) => {
                let value = interpreter.pop_stack();
                let obj = interpreter.pop_stack();

                obj.set_property(Value::String(str.clone()), value)?;
                interpreter.push_stack(obj);
            }
            Bytecode::AppendObjIndirect => {
                let value = interpreter.pop_stack();
                let key = interpreter.pop_stack();
                let obj = interpreter.pop_stack();

                obj.set_property(key, value)?;
                interpreter.push_stack(obj);
            }
            Bytecode::Binary(op) => {
                let b = interpreter.pop_stack();
                let a = interpreter.pop_stack();
                interpreter.push_stack(match op {
                    BinaryOp::Add => a.try_add(&b)?,
                    BinaryOp::Sub => a.try_sub(&b)?,
                    BinaryOp::Mul => a.try_mul(&b)?,
                    BinaryOp::Div => a.try_div(&b)?,
                    BinaryOp::Rem => a.try_rem(&b)?,
                    BinaryOp::SetEq => Value::Bool(a == b),
                    BinaryOp::SetNe => Value::Bool(a != b),
                    BinaryOp::SetLt => Value::Bool(a.try_cmp(&b)?.is_some_and(|c| c.is_lt())),
                    BinaryOp::SetLe => Value::Bool(a.try_cmp(&b)?.is_some_and(|c| c.is_le())),
                    BinaryOp::SetGt => Value::Bool(a.try_cmp(&b)?.is_some_and(|c| c.is_gt())),
                    BinaryOp::SetGe => Value::Bool(a.try_cmp(&b)?.is_some_and(|c| c.is_ge())),
                    op => todo!("Operator {op:?} is not yet implemented."),
                });
            }
            Bytecode::Unary(op) => {
                let a = interpreter.pop_stack();
                interpreter.push_stack(match op {
                    UnaryOp::Negate => a.try_neg()?,
                    UnaryOp::Swap => a.try_swap()?,
                    UnaryOp::SetFalse => Value::Bool(!a.as_bool()),
                    UnaryOp::SetTrue => Value::Bool(a.as_bool()),
                });
            }
            Bytecode::GlobalReadOnly(name) => interpreter.make_global_read_only(name.clone()),
            Bytecode::GlobalDeclare(name) => interpreter.declare_global(name.clone())?,
            Bytecode::LoadProperty(name) => {
                let prop = interpreter
                    .pop_stack()
                    .get_property(&Value::String(name.clone()))?;
                interpreter.push_stack(prop);
            }
            Bytecode::LoadMethod(name) => {
                let obj = interpreter.pop_stack();
                let method = interpreter.method_currying(
                    obj.clone(),
                    obj.get_property(&Value::String(name.clone()))?
                        .try_function()?,
                );
                interpreter.push_stack(Value::Function(Rc::new(method)));
            }
            Bytecode::LoadPropertyIndirect => {
                let prop = interpreter.pop_stack();
                let obj = interpreter.pop_stack();
                interpreter.push_stack(obj.get_property(&prop)?);
            }
            Bytecode::StoreProperty(prop) => {
                let obj = interpreter.pop_stack();
                let value = interpreter.pop_stack();
                obj.set_property(Value::String(prop.clone()), value)?;
            }
            Bytecode::StorePropertyIndirect => {
                let prop = interpreter.pop_stack();
                let obj = interpreter.pop_stack();
                let value = interpreter.pop_stack();
                obj.set_property(prop, value)?;
            }
            Bytecode::LoadGlobal(name) => {
                interpreter.push_stack(interpreter.get_global(name.clone()))
            }
            Bytecode::StoreGlobal(name) => {
                let v = interpreter.pop_stack();
                interpreter.set_global(name.clone(), v)?
            }
            Bytecode::LoadLocal(id) => {
                interpreter.push_stack(interpreter.get_local(*id));
            }
            Bytecode::StoreLocal(id) => {
                let v = interpreter.pop_stack();
                interpreter.set_local(*id, v);
            }
            Bytecode::LoadUpvalue(id) => {
                interpreter.push_stack(interpreter.get_upvalue(*id));
            }
            Bytecode::StoreUpvalue(id) => {
                let value = interpreter.pop_stack();
                interpreter.set_upvalue(*id, value);
            }
            Bytecode::LoadNil => interpreter.push_stack(Value::Nil),
            Bytecode::LoadBool(b) => interpreter.push_stack(Value::Bool(*b)),
            Bytecode::LoadNum(n) => interpreter.push_stack(Value::Number(*n)),
            Bytecode::LoadFn(f) => {
                let fun = interpreter.create_function(f.clone());
                interpreter.push_stack(Value::Function(Rc::new(fun)));
            }
            Bytecode::LoadStr(s) => interpreter.push_stack(Value::String(s.clone())),
            Bytecode::StackToArray(base) => {
                let vec = interpreter.stack[*base..]
                    .iter_mut()
                    .map(|v| replace(v, Value::Nil))
                    .collect::<Vec<_>>();

                interpreter.stack.truncate(*base);
                interpreter.push_stack(Value::Array(Rc::new(RefCell::new(vec))));
            }
            Bytecode::StackToObj(base) => {
                let map = interpreter.stack[*base..]
                    .chunks_exact_mut(2)
                    .map(|pair| {
                        (
                            replace(&mut pair[0], Value::Nil),
                            replace(&mut pair[1], Value::Nil),
                        )
                    })
                    .collect::<FxHashMap<_, _>>();

                interpreter.stack.truncate(*base);
                interpreter.push_stack(Value::Object(Rc::new(RefCell::new(Object::new(map)?))));
            }
            Bytecode::BranchIf(cond, offset) => {
                let a = interpreter.pop_stack();
                if a.as_bool() == *cond {
                    return Ok(Ok(index.wrapping_add_signed(*offset)));
                }
            }
            Bytecode::Jump(offset) => return Ok(Ok(index.wrapping_add_signed(*offset))),
            Bytecode::Truncate(new_len) => interpreter.truncate(*new_len),
            Bytecode::Return => return Ok(Err(interpreter.pop_stack())),
            Bytecode::Call(base) => {
                let v = interpreter.call_function(*base)?;
                interpreter.push_stack(v);
            }
            Bytecode::CallVariadic => {
                let params = interpreter.pop_stack();
                let func = interpreter.pop_stack().try_function()?;
                let abs_base_ptr = interpreter.memory.len();
                let abs_base_stack = interpreter.stack.len();

                params.try_iterate(interpreter, |int, v| Ok(int.memory.push(Cell::Value(v))))?;
                let ret =
                    interpreter.call_function_unchecked(func, abs_base_ptr, abs_base_stack)?;
                interpreter.push_stack(ret);
            }
            Bytecode::CallBuiltin(base, func) => {
                let v = interpreter.call_builtin_function(func.clone(), *base)?;
                interpreter.push_stack(v);
            }
        }
        Ok(Ok(index + 1))
    }
}
