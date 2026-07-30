use crate::error::ErrorKind;
use crate::interpreter::string::{InternedStr, ValueStr};
use crate::interpreter::value::{Object, Value};
use crate::interpreter::{FnSignature, Interpreter, LocalId};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub enum Load {
    Local(LocalId),
    LocalIndirect(LocalId),
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
impl PartialEq for Load {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Self::Local(id) => matches!(other, Self::Local(id2) if id == id2),
            Self::LocalIndirect(id) => matches!(other, Self::LocalIndirect(id2) if id == id2),
            Self::Upvalue(id) => matches!(other, Self::Upvalue(id2) if id == id2),
            Self::Global(id) => matches!(other, Self::Global(id2) if id == id2),
            Self::Nil => matches!(other, Self::Nil),
            Self::Bool(b) => matches!(other, Self::Bool(b2) if b == b2),
            Self::Number(n) => matches!(other, Self::Number(n2) if n == n2),
            Self::String(s) => matches!(other, Self::String(s2) if s == s2),
            Self::Function(f) => matches!(other, Self::Function(f2) if Rc::ptr_eq(f, f2)),
            Self::Array(_) => matches!(other, Self::Array(_)),
            Self::Object(_) => matches!(other, Self::Object(_)),
        }
    }
}
impl Load {
    fn load(&self, interpreter: &mut Interpreter) -> Result<Value, ErrorKind> {
        Ok(match self {
            Self::Local(id) => interpreter.get_local(*id),
            Self::LocalIndirect(id) => {
                interpreter.get_local(interpreter.get_local(*id).as_number()? as LocalId)
            }
            Self::Upvalue(id) => interpreter.get_upvalue(*id)?,
            Self::Global(id) => interpreter.get_global(*id),
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
#[derive(Debug, Clone, PartialEq)]
pub enum Store {
    Local(LocalId),
    LocalIndirect(LocalId),
    Global(InternedStr),
    Upvalue(LocalId),
    Nil,
}
impl Store {
    pub fn to_load(&self) -> Load {
        match self {
            Self::Local(id) => Load::Local(*id),
            Self::LocalIndirect(id) => Load::LocalIndirect(*id),
            Self::Global(id) => Load::Global(*id),
            Self::Upvalue(id) => Load::Upvalue(*id),
            Self::Nil => Load::Nil,
        }
    }
    fn store(&self, interpreter: &mut Interpreter, new_value: Value) -> Result<(), ErrorKind> {
        match self {
            Self::Local(id) => interpreter.set_local(*id, new_value),
            Self::LocalIndirect(id) => interpreter.set_local(
                interpreter.get_local(*id).as_number()? as LocalId,
                new_value,
            ),
            Self::Global(id) => interpreter.set_global(*id, new_value),
            Self::Upvalue(id) => interpreter.set_upvalue(*id, new_value),
            Self::Nil => Ok(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
#[rustfmt::skip]
/// Bytecode for the language. It assumes a linear memory made up of cell that can accept any value.
/// Constants, and globals have their own unique IDs so from the codegen perspective, global and constant identifiers needs to be interned before being used.
///
/// Operational instructions only access the local memory, where it's relative to the base function pointer.
/// The memory automatically grows if the memory index is past the stack pointer.
pub enum Bytecode {
    Nop,

    // Binary operations
    Add { dst: Store, src0: Load, src1: Load },
    Sub { dst: Store, src0: Load, src1: Load },
    Mul { dst: Store, src0: Load, src1: Load },
    Div { dst: Store, src0: Load, src1: Load },
    Rem { dst: Store, src0: Load, src1: Load },
    Pow { dst: Store, src0: Load, src1: Load },
    Shl { dst: Store, src0: Load, src1: Load },
    Shr { dst: Store, src0: Load, src1: Load },
    Sha { dst: Store, src0: Load, src1: Load },
    And { dst: Store, src0: Load, src1: Load },
    Or { dst: Store, src0: Load, src1: Load },
    Xor { dst: Store, src0: Load, src1: Load },

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
    GlobalDeclare(InternedStr), // declare global variable
    GlobalReadOnly(InternedStr), // make GLOBAL[.0] read-only

    // Memory
    Move { dst: Store, src: Load }, // [.0] = [.1]
    Truncate(usize), // truncates till .0

    // Property
    LoadProperty { dst: Store, src: Load, prop: InternedStr }, // [.0] = [.1].(.2) --- Equivalent to a.b
    LoadMethod { dst: Store, src: Load, prop: InternedStr }, // [.0] = [.1]:(.2) --- Equivalent to a:b, returns closure that internally calls `a.b(a, ...)`
    StoreProperty { dst: Load, src: Load, prop: InternedStr }, // [.0].1 = [.2] --- Equivalent to a.b = c
    LoadPropertyIndirect { dst: Store, src: Load, prop: Load }, // [.0] = [.1][[.2]] --- Equivalent to a[b]
    StorePropertyIndirect { dst: Load, src: Load, prop: Load }, // [.0][[.1]] = [.2] --- Equivalent to a[b] = c

    // Array initialization
    AppendElement { dst: Load, src: Load },
    AppendElements { dst: Load, src: Load }, // elems should be array or any iterator

    // Object initialization
    StoreProperties { dst: Load, src: Load }, // src should be object or any iterator

    // Jumping
    Jump(isize), // IP += .0

    // Function call
    Call { src: Load, base: LocalId, dst: Store },
    CallArray { src: Load, args: Load, dst: Store },

    // Return
    Return(Load),
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
            Bytecode::GlobalReadOnly(name) => interpreter.make_global_read_only(*name),
            Bytecode::GlobalDeclare(name) => interpreter.declare_global(*name)?,
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
                    return Ok(Ok(((index as isize) + *offset) as usize));
                }
            }
            Bytecode::BrFalse { src, offset } => {
                let value = src.load(interpreter)?;
                if !value.as_bool() {
                    return Ok(Ok(((index as isize) + *offset) as usize));
                }
            }
            Bytecode::BrEq { offset, src0, src1 } => {
                let v0 = src0.load(interpreter)?;
                let v1 = src1.load(interpreter)?;
                if v0 == v1 {
                    return Ok(Ok(((index as isize) + *offset) as usize));
                }
            }
            Bytecode::BrNe { offset, src0, src1 } => {
                let v0 = src0.load(interpreter)?;
                let v1 = src1.load(interpreter)?;
                if v0 != v1 {
                    return Ok(Ok(((index as isize) + *offset) as usize));
                }
            }
            Bytecode::BrGe { offset, src0, src1 } => {
                let v0 = src0.load(interpreter)?;
                let v1 = src1.load(interpreter)?;
                if v0.try_cmp(&v1)?.is_some_and(|cmp| cmp.is_ge()) {
                    return Ok(Ok(((index as isize) + *offset) as usize));
                }
            }
            Bytecode::BrLe { offset, src0, src1 } => {
                let v0 = src0.load(interpreter)?;
                let v1 = src1.load(interpreter)?;
                if v0.try_cmp(&v1)?.is_some_and(|cmp| cmp.is_le()) {
                    return Ok(Ok(((index as isize) + *offset) as usize));
                }
            }
            Bytecode::BrGt { offset, src0, src1 } => {
                let v0 = src0.load(interpreter)?;
                let v1 = src1.load(interpreter)?;
                if v0.try_cmp(&v1)?.is_some_and(|cmp| cmp.is_gt()) {
                    return Ok(Ok(((index as isize) + *offset) as usize));
                }
            }
            Bytecode::BrLt { offset, src0, src1 } => {
                let v0 = src0.load(interpreter)?;
                let v1 = src1.load(interpreter)?;
                if v0.try_cmp(&v1)?.is_some_and(|cmp| cmp.is_lt()) {
                    return Ok(Ok(((index as isize) + *offset) as usize));
                }
            }
            Bytecode::Jump(offset) => return Ok(Ok(((index as isize) + *offset) as usize)),
            Bytecode::Move { dst, src } => {
                let value = src.load(interpreter)?;
                dst.store(interpreter, value)?;
            }
            Bytecode::Truncate(new_len) => interpreter.truncate(*new_len)?,
            Bytecode::Return(src) => return src.load(interpreter).map(Err),
            Bytecode::Call { src, base, dst } => {
                let function = src.load(interpreter)?.as_callable()?;
                let value = interpreter.call_function(function, *base)?;
                dst.store(interpreter, value)?;
            }
            Bytecode::AppendElement { dst, src } => {
                dst.load(interpreter)?
                    .try_array()?
                    .borrow_mut()
                    .push(src.load(interpreter)?);
            }
            Bytecode::AppendElements { dst, src } => {
                dst.load(interpreter)?
                    .try_array()?
                    .borrow_mut()
                    .extend(src.load(interpreter)?.try_iterator()?);
            }
            Bytecode::StoreProperties { dst, src } => {
                let dst_obj = dst.load(interpreter)?;
                let src_iter = src.load(interpreter)?.try_iterator()?;

                for v in src_iter {
                    let arr = v.try_array()?;
                    dst_obj.set_property(
                        arr.borrow().get(0).cloned().unwrap_or_default(),
                        arr.borrow().get(1).cloned().unwrap_or_default(),
                    )?;
                }
            }
            bc => todo!("{bc:?} is not implemented yet!"),
        }
        Ok(Ok(index + 1))
    }
}
