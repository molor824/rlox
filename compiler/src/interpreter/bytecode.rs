use crate::error::ErrorKind;
use crate::interpreter::string::{InternedStr, ValueStr};
use crate::interpreter::value::{Object, Value};
use crate::interpreter::{FnSignature, Interpreter, LocalId};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub enum Load {
    Local(LocalId),
    Upvalue(LocalId),
    Global(InternedStr),
    Nil,
    Bool(bool),
    Number(f64),
    String(InternedStr),
    Function(Rc<FnSignature>),
    Array(usize),
    Object(usize),
}
impl Load {
    fn load(&self, interpreter: &mut Interpreter) -> Result<Value, ErrorKind> {
        Ok(match self {
            Self::Local(id) => interpreter.get_local(*id),
            Self::Upvalue(id) => interpreter.get_upvalue(*id),
            Self::Global(id) => interpreter.get_global(ValueStr::Interned(*id)),
            Self::Nil => Value::Nil,
            Self::Bool(b) => Value::Bool(*b),
            Self::Number(n) => Value::Number(*n),
            Self::String(s) => Value::String(ValueStr::Interned(*s)),
            Self::Function(s) => Value::Function(Rc::new(interpreter.create_function(s.clone())?)),
            Self::Array(c) => Value::Array(Rc::new(RefCell::new(Vec::with_capacity(*c)))),
            Self::Object(c) => Value::Object(Rc::new(RefCell::new(Object::with_capacity(*c)))),
        })
    }
}
#[derive(Debug, Clone)]
pub enum Store {
    Local(LocalId),
    Global(InternedStr),
    Upvalue(LocalId),
}
impl Store {
    fn store(&self, interpreter: &mut Interpreter, new_value: Value) -> Result<(), ErrorKind> {
        match self {
            Self::Local(id) => interpreter.set_local(*id, new_value),
            Self::Global(id) => interpreter.set_global(ValueStr::Interned(*id), new_value),
            Self::Upvalue(id) => interpreter.set_upvalue(*id, new_value),
        }
    }
}

#[derive(Debug, Clone)]
#[rustfmt::skip]
/// Bytecode for the language. It assumes a linear memory made up of cell that can accept any value.
/// Constants, and globals have their own unique IDs so from the codegen perspective, global and constant identifiers needs to be interned before being used.
///
/// Operational instructions only access the local memory, where it's relative to the base function pointer.
/// The memory automatically grows if the memory index is past the stack pointer.
pub enum Bytecode {
    // Binary operations
    Add { dst: Store, src0: Load, src1: Load },
    Sub { dst: Store, src0: Load, src1: Load },
    Mul { dst: Store, src0: Load, src1: Load },
    Div { dst: Store, src0: Load, src1: Load },
    Rem { dst: Store, src0: Load, src1: Load },
    SetEq { dst: Store, src0: Load, src1: Load },
    SetNe { dst: Store, src0: Load, src1: Load },
    SetLt { dst: Store, src0: Load, src1: Load },
    SetGt { dst: Store, src0: Load, src1: Load },
    SetLe { dst: Store, src0: Load, src1: Load },
    SetGe { dst: Store, src0: Load, src1: Load },

    // Unary operations
    Negate { dst: Store, src: Load },
    Invert { dst: Store, src: Load },
    SetTrue { dst: Store, src: Load },
    SetFalse { dst: Store, src: Load },

    // Branching operations
    BrEq { offset: isize, src0: Load, src1: Load },
    BrNe { offset: isize, src0: Load, src1: Load },
    BrLt { offset: isize, src0: Load, src1: Load },
    BrGt { offset: isize, src0: Load, src1: Load },
    BrLe { offset: isize, src0: Load, src1: Load },
    BrGe { offset: isize, src0: Load, src1: Load },
    BrTrue { offset: isize, src: Load },
    BrFalse { offset: isize, src: Load },

    // Global memory
    GlobalReadOnly(InternedStr), // make GLOBAL[.0] read-only

    // Memory
    Copy { dst: Store, src: Load }, // [.0] = [.1]
    Truncate(usize), // truncates till .0

    // Property
    LoadProperty { dst: Store, src: Load, prop: InternedStr }, // [.0] = [.1].(.2) --- Equivalent to a.b
    LoadMethod { dst: Store, src: Load, prop: InternedStr }, // [.0] = [.1]:(.2) --- Equivalent to a:b, returns closure that internally calls `a.b(a, ...)`
    StoreProperty { dst: Load, src: Load, prop: InternedStr }, // [.0].1 = [.2] --- Equivalent to a.b = c
    LoadPropertyIndirect { dst: Store, src: Load, prop: Load }, // [.0] = [.1][[.2]] --- Equivalent to a[b]
    StorePropertyIndirect { dst: Load, src: Load, prop: Load }, // [.0][[.1]] = [.2] --- Equivalent to a[b] = c

    // Jumping
    Jump(isize), // IP += .0

    // Function call
    Call { src: Load, arity: u32 },

    // Return
    Return,
}
impl Bytecode {
    // None -> return
    // Some(i) -> next instruction index
    pub fn interpret(
        &self,
        interpreter: &mut Interpreter,
        index: usize,
    ) -> Result<Option<usize>, ErrorKind> {
        match self {
            Bytecode::Add { src0, src1, dst } => {
                let v0 = src0.load(interpreter)?;
                let v1 = src1.load(interpreter)?;
                dst.store(interpreter, v0.try_add(&v1)?)?;
            }
            Bytecode::Sub { src0, src1, dst } => {
                let v0 = src0.load(interpreter)?;
                let v1 = src1.load(interpreter)?;
                dst.store(interpreter, v0.try_sub(&v1)?)?;
            }
            Bytecode::Mul { src0, src1, dst } => {
                let v0 = src0.load(interpreter)?;
                let v1 = src1.load(interpreter)?;
                dst.store(interpreter, v0.try_mul(&v1)?)?;
            }
            Bytecode::Div { src0, src1, dst } => {
                let v0 = src0.load(interpreter)?;
                let v1 = src1.load(interpreter)?;
                dst.store(interpreter, v0.try_div(&v1)?)?;
            }
            Bytecode::Rem { src0, src1, dst } => {
                let v0 = src0.load(interpreter)?;
                let v1 = src1.load(interpreter)?;
                dst.store(interpreter, v0.try_rem(&v1)?)?;
            }
            Bytecode::SetEq { dst, src0, src1 } => {
                let v0 = src0.load(interpreter)?;
                let v1 = src1.load(interpreter)?;
                dst.store(interpreter, Value::Bool(v0 == v1))?;
            }
            Bytecode::SetNe { dst, src0, src1 } => {
                let v0 = src0.load(interpreter)?;
                let v1 = src1.load(interpreter)?;
                dst.store(interpreter, Value::Bool(v0 != v1))?;
            }
            Bytecode::SetLt { dst, src0, src1 } => {
                let v0 = src0.load(interpreter)?;
                let v1 = src1.load(interpreter)?;
                dst.store(
                    interpreter,
                    Value::Bool(v0.try_cmp(&v1)?.is_some_and(|ord| ord.is_lt())),
                )?;
            }
            Bytecode::SetGt { dst, src0, src1 } => {
                let v0 = src0.load(interpreter)?;
                let v1 = src1.load(interpreter)?;
                dst.store(
                    interpreter,
                    Value::Bool(v0.try_cmp(&v1)?.is_some_and(|ord| ord.is_gt())),
                )?;
            }
            Bytecode::SetLe { dst, src0, src1 } => {
                let v0 = src0.load(interpreter)?;
                let v1 = src1.load(interpreter)?;
                dst.store(
                    interpreter,
                    Value::Bool(v0.try_cmp(&v1)?.is_some_and(|ord| ord.is_le())),
                )?;
            }
            Bytecode::SetGe { dst, src0, src1 } => {
                let v0 = src0.load(interpreter)?;
                let v1 = src1.load(interpreter)?;
                dst.store(
                    interpreter,
                    Value::Bool(v0.try_cmp(&v1)?.is_some_and(|ord| ord.is_ge())),
                )?;
            }
            Bytecode::Negate { dst, src } => {
                let value = src.load(interpreter)?;
                dst.store(interpreter, value.try_neg()?)?;
            }
            Bytecode::Invert { dst, src } => {
                let value = src.load(interpreter)?;
                dst.store(interpreter, value.try_inv()?)?;
            }
            Bytecode::SetTrue { dst, src } => {
                let value = src.load(interpreter)?;
                dst.store(interpreter, Value::Bool(value.as_bool()))?;
            }
            Bytecode::SetFalse { dst, src } => {
                let value = src.load(interpreter)?;
                dst.store(interpreter, Value::Bool(!value.as_bool()))?;
            }
            Bytecode::GlobalReadOnly(id) => {
                interpreter.make_global_read_only(ValueStr::Interned(*id))
            }
            Bytecode::LoadProperty { dst, src, prop } => {
                let property = src
                    .load(interpreter)?
                    .get_property(&Value::String(ValueStr::Interned(*prop)))?;
                dst.store(interpreter, property)?;
            }
            Bytecode::LoadMethod { dst, src, prop } => {
                let itself = src.load(interpreter)?;
                let function = itself
                    .get_property(&Value::String(ValueStr::Interned(*prop)))?
                    .as_callable()?;
                let method = Rc::new(interpreter.method_currying(itself, function)?);
                dst.store(interpreter, Value::Function(method))?;
            }
            Bytecode::LoadPropertyIndirect { dst, src, prop } => {
                let obj = src.load(interpreter)?;
                let key = prop.load(interpreter)?;
                let property = obj.get_property(&key)?;
                dst.store(interpreter, property)?;
            }
            Bytecode::StoreProperty { dst, src, prop } => {
                let value = src.load(interpreter)?;
                let obj = dst.load(interpreter)?;
                obj.set_property(Value::String(ValueStr::Interned(*prop)), value)?;
            }
            Bytecode::StorePropertyIndirect { dst, src, prop } => {
                let value = src.load(interpreter)?;
                let obj = dst.load(interpreter)?;
                let key = prop.load(interpreter)?;
                obj.set_property(key, value)?;
            }
            Bytecode::BrTrue { src, offset } => {
                let value = src.load(interpreter)?;
                if value.as_bool() {
                    return Ok(Some(((index as isize) + *offset) as usize));
                }
            }
            Bytecode::BrFalse { src, offset } => {
                let value = src.load(interpreter)?;
                if !value.as_bool() {
                    return Ok(Some(((index as isize) + *offset) as usize));
                }
            }
            Bytecode::BrEq { offset, src0, src1 } => {
                let v0 = src0.load(interpreter)?;
                let v1 = src1.load(interpreter)?;
                if v0 == v1 {
                    return Ok(Some(((index as isize) + *offset) as usize));
                }
            }
            Bytecode::BrNe { offset, src0, src1 } => {
                let v0 = src0.load(interpreter)?;
                let v1 = src1.load(interpreter)?;
                if v0 != v1 {
                    return Ok(Some(((index as isize) + *offset) as usize));
                }
            }
            Bytecode::BrGe { offset, src0, src1 } => {
                let v0 = src0.load(interpreter)?;
                let v1 = src1.load(interpreter)?;
                if v0.try_cmp(&v1)?.is_some_and(|cmp| cmp.is_ge()) {
                    return Ok(Some(((index as isize) + *offset) as usize));
                }
            }
            Bytecode::BrLe { offset, src0, src1 } => {
                let v0 = src0.load(interpreter)?;
                let v1 = src1.load(interpreter)?;
                if v0.try_cmp(&v1)?.is_some_and(|cmp| cmp.is_le()) {
                    return Ok(Some(((index as isize) + *offset) as usize));
                }
            }
            Bytecode::BrGt { offset, src0, src1 } => {
                let v0 = src0.load(interpreter)?;
                let v1 = src1.load(interpreter)?;
                if v0.try_cmp(&v1)?.is_some_and(|cmp| cmp.is_gt()) {
                    return Ok(Some(((index as isize) + *offset) as usize));
                }
            }
            Bytecode::BrLt { offset, src0, src1 } => {
                let v0 = src0.load(interpreter)?;
                let v1 = src1.load(interpreter)?;
                if v0.try_cmp(&v1)?.is_some_and(|cmp| cmp.is_lt()) {
                    return Ok(Some(((index as isize) + *offset) as usize));
                }
            }
            Bytecode::Jump(offset) => return Ok(Some(((index as isize) + *offset) as usize)),
            Bytecode::Copy { dst, src } => {
                let value = src.load(interpreter)?;
                dst.store(interpreter, value)?;
            }
            Bytecode::Truncate(new_len) => interpreter.truncate(*new_len)?,
            Bytecode::Return => return Ok(None),
            Bytecode::Call { src, arity } => {
                let function = src.load(interpreter)?.as_callable()?;
                interpreter.call_function(function, *arity as usize)?;
            }
        }
        Ok(Some(index + 1))
    }
}
