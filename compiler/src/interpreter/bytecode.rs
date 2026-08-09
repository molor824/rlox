use crate::error::ErrorKind;
use crate::interpreter::string::ValueStr;
use crate::interpreter::value::{Object, Value};
use crate::interpreter::{FnSignature, Interpreter};
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
pub enum UnaryOps {
    Negate,
    Swap,
    SetTrue,
    SetFalse,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchCond {
    False,
    True,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone)]
#[rustfmt::skip]
/// Bytecode for the language. It assumes a linear memory made up of cell that can accept any value.
/// Constants, and globals have their own unique IDs so from the codegen perspective, global and constant identifiers needs to be interned before being used.
///
/// Operational instructions only access the local memory, where it's relative to the base function pointer.
/// The memory automatically grows if the memory index is past the stack pointer.
pub enum Bytecode {
    Nop,
    Dup, // s0 -> s0, s0
    // Binary operations
    Binary(BinaryOp), // s0, s1 -> <BINARY> s0 s1
    // Unary operations
    Unary(UnaryOps), // s0 -> <UNARY> s0
    // Branching operations
    Branch(BranchCond, isize), // if BranchCond::True || BranchCond::False then s0 -> <BR_IF> .0 s0; else s0, s1 -> <BR_IF> .0 s0 s1;
    // Global memory
    GlobalDeclare(ValueStr), // declare global
    GlobalReadOnly(ValueStr), // make global readonly
    LoadGlobal(ValueStr), // () -> <GET_GLOBAL> .0
    GlobalStore(ValueStr), // s0 -> <SET_GLOBAL> .0 s0;
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
    StoreProperty(ValueStr), // obj, val -> obj[.0] = val;
    StorePropertyIndirect, // obj, key, val -> obj[key] = val;
    LoadMethod(ValueStr), // obj -> (\... -> obj[key](obj[key], ...))
    // Array initialization
    StackToArray(usize), // starting at .0 offset: s0, s1, s2, ... -> [s0, s1, s2, ...]
    // Object initialization
    StackToObj(usize), // starting at .0 offset: k0, v0, k1, v1, ... -> {k0: v0, k1: v1, ...}
    LoadNil,
    LoadBool(bool),
    LoadNum(f64),
    LoadFn(Rc<FnSignature>),
    LoadStr(ValueStr),
    // Jumping
    Jump(isize), // pc += .0
    // Function call
    Call(usize), // starting at .0 offset: p0, p1, p2, ..., func -> func(p0, p1, p2, ...)
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
            Bytecode::GlobalReadOnly(name) => interpreter.make_global_read_only(name.clone()),
            Bytecode::GlobalDeclare(name) => interpreter.declare_global(name.clone())?,
            Bytecode::LoadProperty { dst, src, prop } => {
                let property = src
                    .load(interpreter)?
                    .get_property(&Value::String(prop.clone()))?;
                dst.store(interpreter, property)?;
            }
            Bytecode::LoadMethod { dst, src, prop } => {
                let itself = src.load(interpreter)?;
                let function = itself
                    .get_property(&Value::String(prop.clone()))?
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
                obj.set_property(Value::String(prop.clone()), value)?;
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
            Bytecode::Truncate(new_len) => interpreter.truncate(*new_len),
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
