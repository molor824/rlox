use crate::interpreter::error::ErrorKind;
use crate::interpreter::string::{InternedStr, ValueStr};
use crate::interpreter::value::{Object, Value};
use crate::interpreter::{FnSignature, Interpreter, LocalId};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone)]
/// Bytecode for the language. It assumes a linear memory made up of cell that can accept any value.
/// Constants, and globals have their own unique IDs so from the codegen perspective, global and constant identifiers needs to be interned before being used.
///
/// Operational instructions only access the local memory, where it's relative to the base function pointer.
/// The memory automatically grows if the memory index is past the stack pointer.
pub enum Bytecode {
    // Binary operations
    Add {
        dst: LocalId,
        src0: LocalId,
        src1: LocalId,
    },
    Sub {
        dst: LocalId,
        src0: LocalId,
        src1: LocalId,
    },
    Mul {
        dst: LocalId,
        src0: LocalId,
        src1: LocalId,
    },
    Div {
        dst: LocalId,
        src0: LocalId,
        src1: LocalId,
    },
    Rem {
        dst: LocalId,
        src0: LocalId,
        src1: LocalId,
    },
    SetEq {
        dst: LocalId,
        src0: LocalId,
        src1: LocalId,
    },
    SetNe {
        dst: LocalId,
        src0: LocalId,
        src1: LocalId,
    },
    SetLt {
        dst: LocalId,
        src0: LocalId,
        src1: LocalId,
    },
    SetGt {
        dst: LocalId,
        src0: LocalId,
        src1: LocalId,
    },
    SetLe {
        dst: LocalId,
        src0: LocalId,
        src1: LocalId,
    },
    SetGe {
        dst: LocalId,
        src0: LocalId,
        src1: LocalId,
    },

    // Unary operations
    Negate {
        dst: LocalId,
        src: LocalId,
    },
    Invert {
        dst: LocalId,
        src: LocalId,
    },
    SetTrue {
        dst: LocalId,
        src: LocalId,
    }, // sets true if value is object, array/string with content, true, non zero number
    SetFalse {
        dst: LocalId,
        src: LocalId,
    }, // sets false if value is nil, empty array/string, false, zero

    // Branching operations
    BrTrue {
        src: LocalId,
        offset: isize,
    },
    BrFalse {
        src: LocalId,
        offset: isize,
    },

    // Global memory
    LoadGlobal {
        dst: LocalId,
        src: InternedStr,
    }, // [.0] = GLOBAL[.1]
    LoadGlobalIndirect {
        dst: LocalId,
        src: LocalId,
    }, // [.0] = GLOBAL[[.1]]
    StoreGlobal {
        dst: InternedStr,
        src: LocalId,
    }, // GLOBAL[.0] = [.1]
    StoreGlobalIndirect {
        dst: LocalId,
        src: LocalId,
    }, // GLOBAL[[.0]] = [.1]
    GlobalReadOnly(InternedStr), // make GLOBAL[.0] read-only

    // Memory
    Clone {
        dst: LocalId,
        src: LocalId,
    }, // [.0] = [.1]
    Truncate(usize), // truncates till .0

    // Property
    LoadProperty {
        dst: LocalId,
        src: LocalId,
        prop: InternedStr,
    }, // [.0] = [.1].(.2) --- Equivalent to a.b
    LoadPropertyIndirect {
        dst: LocalId,
        src: LocalId,
        prop: LocalId,
    }, // [.0] = [.1][[.2]] --- Equivalent to a[b]
    LoadMethod {
        dst: LocalId,
        src: LocalId,
        prop: InternedStr,
    }, // [.0] = [.1]:(.2) --- Equivalent to a:b, returns closure that internally calls `a.b(a, ...)`
    StoreProperty {
        dst: LocalId,
        prop: InternedStr,
        src: LocalId,
    }, // [.0].1 = [.2] --- Equivalent to a.b = c
    StorePropertyIndirect {
        dst: LocalId,
        prop: LocalId,
        src: LocalId,
    }, // [.0][[.1]] = [.2] --- Equivalent to a[b] = c

    // Creating custom types
    LoadNil(LocalId),                       // [.0] = nil
    LoadFloat(LocalId, f64),                // [.0] = float(.1)
    LoadStr(LocalId, InternedStr),          // [.0] = str(.1)
    LoadArray(LocalId, usize),              // [.0] = array.with_capacity(.1)
    LoadObject(LocalId, usize),             // [.0] = object.with_capacity(.1)
    LoadFunction(LocalId, Rc<FnSignature>), // [.0] = closure.from_function_address(.1)

    // Jumping
    Jump(isize), // IP += .0

    // Function call
    // CallSignature { // Implement later when signature based optimization is implemented
    //     signature: Rc<FnSignature>,
    //     arity: u32,
    // },
    Call {
        src: LocalId,
        arity: u32,
    },
    // CallIntrinsic { // Implement later when built-in method based optimization is implemented
    //     ident: InternedStr,
    //     arity: u32,
    // },
    // TailCall { // Implement later when tail-call optimization is implemented
    //     src: LocalId,
    //     arity: u32,
    // },

    // Return
    Return, // IP = POP()
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
                let v0 = interpreter.get_local(*src0);
                let v1 = interpreter.get_local(*src1);
                let result = v0.try_add(&v1)?;
                interpreter.set_local(*dst, result)?;
            }
            Bytecode::Sub { src0, src1, dst } => {
                let v0 = interpreter.get_local(*src0);
                let v1 = interpreter.get_local(*src1);
                interpreter.set_local(*dst, v0.try_sub(&v1)?)?;
            }
            Bytecode::Mul { src0, src1, dst } => {
                let v0 = interpreter.get_local(*src0);
                let v1 = interpreter.get_local(*src1);
                interpreter.set_local(*dst, v0.try_mul(&v1)?)?;
            }
            Bytecode::Div { src0, src1, dst } => {
                let v0 = interpreter.get_local(*src0);
                let v1 = interpreter.get_local(*src1);
                interpreter.set_local(*dst, v0.try_div(&v1)?)?;
            }
            Bytecode::Rem { src0, src1, dst } => {
                let v0 = interpreter.get_local(*src0);
                let v1 = interpreter.get_local(*src1);
                interpreter.set_local(*dst, v0.try_rem(&v1)?)?;
            }
            Bytecode::Negate { dst, src } => {
                let value = interpreter.get_local(*src);
                interpreter.set_local(*dst, value.try_neg()?)?;
            }
            Bytecode::Invert { dst, src } => {
                let value = interpreter.get_local(*src);
                interpreter.set_local(*dst, value.try_inv()?)?;
            }
            Bytecode::SetTrue { dst, src } => {
                let value = interpreter.get_local(*src);
                interpreter.set_local(*dst, Value::Bool(value.to_bool()))?;
            }
            Bytecode::SetFalse { dst, src } => {
                let value = interpreter.get_local(*src);
                interpreter.set_local(*dst, Value::Bool(!value.to_bool()))?;
            }
            Bytecode::SetEq { dst, src0, src1 } => {
                let v0 = interpreter.get_local(*src0);
                let v1 = interpreter.get_local(*src1);
                interpreter.set_local(*dst, Value::Bool(v0 == v1))?;
            }
            Bytecode::SetNe { dst, src0, src1 } => {
                let v0 = interpreter.get_local(*src0);
                let v1 = interpreter.get_local(*src1);
                interpreter.set_local(*dst, Value::Bool(v0 != v1))?;
            }
            Bytecode::SetLt { dst, src0, src1 } => {
                let v0 = interpreter.get_local(*src0);
                let v1 = interpreter.get_local(*src1);
                interpreter.set_local(
                    *dst,
                    Value::Bool(v0.try_cmp(&v1)?.is_some_and(|ord| ord.is_lt())),
                )?;
            }
            Bytecode::SetGt { dst, src0, src1 } => {
                let v0 = interpreter.get_local(*src0);
                let v1 = interpreter.get_local(*src1);
                interpreter.set_local(
                    *dst,
                    Value::Bool(v0.try_cmp(&v1)?.is_some_and(|ord| ord.is_gt())),
                )?;
            }
            Bytecode::SetLe { dst, src0, src1 } => {
                let v0 = interpreter.get_local(*src0);
                let v1 = interpreter.get_local(*src1);
                interpreter.set_local(
                    *dst,
                    Value::Bool(v0.try_cmp(&v1)?.is_some_and(|ord| ord.is_le())),
                )?;
            }
            Bytecode::SetGe { dst, src0, src1 } => {
                let v0 = interpreter.get_local(*src0);
                let v1 = interpreter.get_local(*src1);
                interpreter.set_local(
                    *dst,
                    Value::Bool(v0.try_cmp(&v1)?.is_some_and(|ord| ord.is_ge())),
                )?;
            }
            Bytecode::LoadNil(id) => interpreter.set_local(*id, Value::Nil)?,
            Bytecode::LoadFloat(id, float) => interpreter.set_local(*id, Value::Number(*float))?,
            Bytecode::LoadStr(id, str) => {
                interpreter.set_local(*id, Value::String(ValueStr::Interned(*str)))?
            }
            Bytecode::LoadObject(id, capacity) => interpreter.set_local(
                *id,
                Value::Object(Rc::new(RefCell::new(Object::with_capacity(*capacity)))),
            )?,
            Bytecode::LoadArray(id, capacity) => interpreter.set_local(
                *id,
                Value::Array(Rc::new(RefCell::new(Vec::with_capacity(*capacity)))),
            )?,
            Bytecode::LoadFunction(dst, signature) => {
                let function = interpreter.create_function(signature.clone())?;
                interpreter.set_local(*dst, Value::Function(Rc::new(function)))?;
            }
            Bytecode::LoadGlobal { src, dst } => {
                let value = interpreter.get_global(ValueStr::Interned(*src));
                interpreter.set_local(*dst, value)?;
            }
            Bytecode::LoadGlobalIndirect { src, dst } => {
                let str_id = interpreter.get_local(*src).try_str()?;
                let value = interpreter.get_global(str_id);
                interpreter.set_local(*dst, value)?;
            }
            Bytecode::StoreGlobal { src, dst } => {
                let value = interpreter.get_local(*src);
                interpreter.set_global(ValueStr::Interned(*dst), value)?;
            }
            Bytecode::StoreGlobalIndirect { src, dst } => {
                let value = interpreter.get_local(*src);
                let id = interpreter.get_local(*dst).try_str()?;
                interpreter.set_global(id, value)?;
            }
            Bytecode::GlobalReadOnly(id) => {
                interpreter.make_global_read_only(ValueStr::Interned(*id))
            }
            Bytecode::LoadProperty { dst, src, prop } => {
                let property = interpreter
                    .get_local(*src)
                    .get_property(&Value::String(ValueStr::Interned(*prop)))?;
                interpreter.set_local(*dst, property)?;
            }
            Bytecode::LoadMethod { dst, src, prop } => {
                let itself = interpreter.get_local(*src);
                let function = itself
                    .get_property(&Value::String(ValueStr::Interned(*prop)))?
                    .try_callable()?;
                let method = Rc::new(interpreter.method_currying(itself, function)?);
                interpreter.set_local(*dst, Value::Function(method))?;
            }
            Bytecode::LoadPropertyIndirect { dst, src, prop } => {
                let obj = interpreter.get_local(*src);
                let key = interpreter.get_local(*prop);
                let property = obj.get_property(&key)?;
                interpreter.set_local(*dst, property)?;
            }
            Bytecode::StoreProperty { dst, src, prop } => {
                let value = interpreter.get_local(*src);
                let obj = interpreter.get_local(*dst);
                obj.set_property(Value::String(ValueStr::Interned(*prop)), value)?;
            }
            Bytecode::StorePropertyIndirect { dst, src, prop } => {
                let value = interpreter.get_local(*src);
                let obj = interpreter.get_local(*dst);
                let key = interpreter.get_local(*prop);
                obj.set_property(key, value)?;
            }
            Bytecode::BrTrue { src, offset } => {
                let value = interpreter.get_local(*src);
                if value.to_bool() {
                    return Ok(Some(((index as isize) + *offset) as usize));
                }
            }
            Bytecode::BrFalse { src, offset } => {
                let value = interpreter.get_local(*src);
                if !value.to_bool() {
                    return Ok(Some(((index as isize) + *offset) as usize));
                }
            }
            Bytecode::Jump(offset) => return Ok(Some(((index as isize) + *offset) as usize)),
            Bytecode::Clone { dst, src } => {
                let value = interpreter.get_local(*src);
                interpreter.set_local(*dst, value)?;
            }
            Bytecode::Truncate(new_len) => interpreter.truncate(*new_len)?,
            Bytecode::Return => return Ok(None),
            Bytecode::Call { src, arity } => {
                let function = interpreter.get_local(*src).try_callable()?;
                interpreter.call_function(function, *arity as usize)?;
            }
        }
        Ok(Some(index + 1))
    }
}
