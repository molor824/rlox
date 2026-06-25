use crate::interpreter::error::ErrorKind;
use crate::interpreter::string::{InternedStr, ValueStr};
use crate::interpreter::value::{Object, Value};
use crate::interpreter::{ClosureId, Interpreter, LocalId};
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
    BrEq {
        src0: LocalId,
        src1: LocalId,
        offset: isize,
    },
    BrNe {
        src0: LocalId,
        src1: LocalId,
        offset: isize,
    },
    BrLt {
        src0: LocalId,
        src1: LocalId,
        offset: isize,
    },
    BrGt {
        src0: LocalId,
        src1: LocalId,
        offset: isize,
    },
    BrLe {
        src0: LocalId,
        src1: LocalId,
        offset: isize,
    },
    BrGe {
        src0: LocalId,
        src1: LocalId,
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

    // Special
    LoadArity(LocalId), // [.0] = len(ARGS)

    // Memory
    Clone {
        dst: LocalId,
        src: LocalId,
    }, // [.0] = [.1]
    Truncate(usize), // [n-.0:n] = nil; This also triggers upvalue analysis for escape

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
    LoadNil(LocalId),                // [.0] = nil
    LoadFloat(LocalId, f64),         // [.0] = float(.1)
    LoadStr(LocalId, InternedStr),   // [.0] = str(.1)
    LoadArray(LocalId, usize),       // [.0] = array.with_capacity(.1)
    LoadObject(LocalId, usize),      // [.0] = object.with_capacity(.1)
    LoadClosure(LocalId, ClosureId), // [.0] = closure.from_function_address(.1)

    // Jumping
    Jump(isize),           // IP += .0
    JumpIndirect(LocalId), // IP += [.0]

    // Function call
    Call {
        procedure: ClosureId,
        arity: u32,
    },
    CallIndirect {
        closure: LocalId,
        arity: u32,
    },
    CallIntrinsic {
        ident: InternedStr,
        arity: u32,
    },

    // Return
    Return, // IP = POP()
}
impl Bytecode {
    pub fn interpret(self, interpreter: &mut Interpreter) -> Result<(), ErrorKind> {
        match self {
            Bytecode::LoadArity(id) => {
                interpreter.set_local(id, Value::Number(interpreter.current_frame.arity as f64))
            }
            Bytecode::LoadNil(id) => interpreter.set_local(id, Value::Nil),
            Bytecode::LoadFloat(id, float) => interpreter.set_local(id, Value::Number(float)),
            Bytecode::LoadStr(id, str) => {
                interpreter.set_local(id, Value::String(ValueStr::Interned(str)))
            }
            Bytecode::LoadObject(id, capacity) => interpreter.set_local(
                id,
                Value::Object(Rc::new(RefCell::new(Object::with_capacity(capacity)))),
            ),
            Bytecode::LoadArray(id, capacity) => interpreter.set_local(
                id,
                Value::Array(Rc::new(RefCell::new(Vec::with_capacity(capacity)))),
            ),
            Bytecode::LoadGlobal { src, dst } => {
                let value = interpreter
                    .get_global(ValueStr::Interned(src))
                    .unwrap_or_default();
                interpreter.set_local(dst, value)
            }
            Bytecode::LoadGlobalIndirect { src, dst } => {
                let str_id = interpreter.get_local(src).unwrap_or_default().try_str()?;
                let value = interpreter.get_global(str_id).unwrap_or_default();
                interpreter.set_local(dst, value)
            }
            Bytecode::StoreGlobal { src, dst } => {
                let value = interpreter.get_local(src).unwrap_or_default();
                interpreter.set_global(ValueStr::Interned(dst), value);
                Ok(())
            }
            Bytecode::StoreGlobalIndirect { src, dst } => {
                let value = interpreter.get_local(src).unwrap_or_default();
                let id = interpreter.get_local(dst).unwrap_or_default().try_str()?;
                interpreter.set_global(id, value);
                Ok(())
            }
            Bytecode::Add { src0, src1, dst } => {
                let v0 = interpreter.get_local(src0).unwrap_or_default();
                let v1 = interpreter.get_local(src1).unwrap_or_default();
                let result = v0.try_add(&v1)?;
                interpreter.set_local(dst, result)
            }
            Bytecode::Sub { src0, src1, dst } => {
                let v0 = interpreter.get_local(src0).unwrap_or_default();
                let v1 = interpreter.get_local(src1).unwrap_or_default();
                interpreter.set_local(dst, v0.try_sub(&v1)?)
            }
            Bytecode::Mul { src0, src1, dst } => {
                let v0 = interpreter.get_local(src0).unwrap_or_default();
                let v1 = interpreter.get_local(src1).unwrap_or_default();
                interpreter.set_local(dst, v0.try_mul(&v1)?)
            }
            Bytecode::Div { src0, src1, dst } => {
                let v0 = interpreter.get_local(src0).unwrap_or_default();
                let v1 = interpreter.get_local(src1).unwrap_or_default();
                interpreter.set_local(dst, v0.try_div(&v1)?)
            }
            Bytecode::Rem { src0, src1, dst } => {
                let v0 = interpreter.get_local(src0).unwrap_or_default();
                let v1 = interpreter.get_local(src1).unwrap_or_default();
                interpreter.set_local(dst, v0.try_rem(&v1)?)
            }
            Bytecode::Negate { dst, src } => {
                let value = interpreter.get_local(src).unwrap_or_default();
                interpreter.set_local(dst, value.try_neg()?)
            }
            Bytecode::Invert { dst, src } => {
                let value = interpreter.get_local(src).unwrap_or_default();
                interpreter.set_local(dst, value.try_inv()?)
            }
            Bytecode::LoadProperty { dst, src, prop } => {
                let obj = interpreter.get_local(src).unwrap_or_default().try_obj()?;
                let property = obj
                    .borrow()
                    .get_property(&Value::String(ValueStr::Interned(prop)))?
                    .unwrap_or_default();
                interpreter.set_local(dst, property)
            }
            Bytecode::LoadPropertyIndirect { dst, src, prop } => {
                let obj = interpreter.get_local(src).unwrap_or_default().try_obj()?;
                let key = interpreter.get_local(prop).unwrap_or_default();
                let property = obj.borrow().get_property(&key)?.unwrap_or_default();
                interpreter.set_local(dst, property)
            }
            Bytecode::StoreProperty { dst, src, prop } => {
                let value = interpreter.get_local(src).unwrap_or_default();
                let obj = interpreter.get_local(dst).unwrap_or_default().try_obj()?;
                let mut obj = obj.borrow_mut();
                obj.set_property(Value::String(ValueStr::Interned(prop)), value)
            }
            Bytecode::StorePropertyIndirect { dst, src, prop } => {
                let value = interpreter.get_local(src).unwrap_or_default();
                let obj = interpreter.get_local(dst).unwrap_or_default().try_obj()?;
                let key = interpreter.get_local(prop).unwrap_or_default();
                let mut obj = obj.borrow_mut();
                obj.set_property(key, value)
            }
            Bytecode::SetEq { dst, src0, src1 } => {
                let v0 = interpreter.get_local(src0).unwrap_or_default();
                let v1 = interpreter.get_local(src1).unwrap_or_default();
                interpreter.set_local(dst, Value::Bool(v0 == v1))
            }
            Bytecode::SetNe { dst, src0, src1 } => {
                let v0 = interpreter.get_local(src0).unwrap_or_default();
                let v1 = interpreter.get_local(src1).unwrap_or_default();
                interpreter.set_local(dst, Value::Bool(v0 != v1))
            }
            Bytecode::SetLt { dst, src0, src1 } => {
                let v0 = interpreter.get_local(src0).unwrap_or_default();
                let v1 = interpreter.get_local(src1).unwrap_or_default();
                interpreter.set_local(
                    dst,
                    Value::Bool(v0.try_cmp(&v1)?.is_some_and(|ord| ord.is_lt())),
                )
            }
            Bytecode::SetGt { dst, src0, src1 } => {
                let v0 = interpreter.get_local(src0).unwrap_or_default();
                let v1 = interpreter.get_local(src1).unwrap_or_default();
                interpreter.set_local(
                    dst,
                    Value::Bool(v0.try_cmp(&v1)?.is_some_and(|ord| ord.is_gt())),
                )
            }
            Bytecode::SetLe { dst, src0, src1 } => {
                let v0 = interpreter.get_local(src0).unwrap_or_default();
                let v1 = interpreter.get_local(src1).unwrap_or_default();
                interpreter.set_local(
                    dst,
                    Value::Bool(v0.try_cmp(&v1)?.is_some_and(|ord| ord.is_le())),
                )
            }
            Bytecode::SetGe { dst, src0, src1 } => {
                let v0 = interpreter.get_local(src0).unwrap_or_default();
                let v1 = interpreter.get_local(src1).unwrap_or_default();
                interpreter.set_local(
                    dst,
                    Value::Bool(v0.try_cmp(&v1)?.is_some_and(|ord| ord.is_ge())),
                )
            }
            Bytecode::BrEq { src0, src1, offset } => {
                let v0 = interpreter.get_local(src0).unwrap_or_default();
                let v1 = interpreter.get_local(src1).unwrap_or_default();
                if v0 == v1 {
                    interpreter.next_ip =
                        ((interpreter.instruction_pointer as isize) + offset) as usize;
                }
                Ok(())
            }
            Bytecode::BrNe { src0, src1, offset } => {
                let v0 = interpreter.get_local(src0).unwrap_or_default();
                let v1 = interpreter.get_local(src1).unwrap_or_default();
                if v0 != v1 {
                    interpreter.next_ip =
                        ((interpreter.instruction_pointer as isize) + offset) as usize;
                }
                Ok(())
            }
            Bytecode::BrLt { src0, src1, offset } => {
                let v0 = interpreter.get_local(src0).unwrap_or_default();
                let v1 = interpreter.get_local(src1).unwrap_or_default();
                if v0.try_cmp(&v1)?.is_some_and(|cmp| cmp.is_lt()) {
                    interpreter.next_ip =
                        ((interpreter.instruction_pointer as isize) + offset) as usize;
                }
                Ok(())
            }
            Bytecode::BrGt { src0, src1, offset } => {
                let v0 = interpreter.get_local(src0).unwrap_or_default();
                let v1 = interpreter.get_local(src1).unwrap_or_default();
                if v0.try_cmp(&v1)?.is_some_and(|cmp| cmp.is_gt()) {
                    interpreter.next_ip =
                        ((interpreter.instruction_pointer as isize) + offset) as usize;
                }
                Ok(())
            }
            Bytecode::BrLe { src0, src1, offset } => {
                let v0 = interpreter.get_local(src0).unwrap_or_default();
                let v1 = interpreter.get_local(src1).unwrap_or_default();
                if v0.try_cmp(&v1)?.is_some_and(|cmp| cmp.is_le()) {
                    interpreter.next_ip =
                        ((interpreter.instruction_pointer as isize) + offset) as usize;
                }
                Ok(())
            }
            Bytecode::BrGe { src0, src1, offset } => {
                let v0 = interpreter.get_local(src0).unwrap_or_default();
                let v1 = interpreter.get_local(src1).unwrap_or_default();
                if v0.try_cmp(&v1)?.is_some_and(|cmp| cmp.is_ge()) {
                    interpreter.next_ip =
                        ((interpreter.instruction_pointer as isize) + offset) as usize;
                }
                Ok(())
            }
            _ => todo!(),
        }
    }
}
