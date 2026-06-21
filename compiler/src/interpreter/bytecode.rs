use crate::interpreter::{ClosureId, GlobalId, LocalId};

#[derive(Debug, Clone, Copy)]
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
    SetNotEq {
        dst: LocalId,
        src0: LocalId,
        src1: LocalId,
    },
    SetLessThan {
        dst: LocalId,
        src0: LocalId,
        src1: LocalId,
    },
    SetGreaterThan {
        dst: LocalId,
        src0: LocalId,
        src1: LocalId,
    },
    SetLessEquals {
        dst: LocalId,
        src0: LocalId,
        src1: LocalId,
    },
    SetGreaterEquals {
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
    BrNotEq {
        src0: LocalId,
        src1: LocalId,
        offset: isize,
    },
    BrLessThan {
        src0: LocalId,
        src1: LocalId,
        offset: isize,
    },
    BrGreaterThan {
        src0: LocalId,
        src1: LocalId,
        offset: isize,
    },
    BrLessEquals {
        src0: LocalId,
        src1: LocalId,
        offset: isize,
    },
    BrGreaterEquals {
        src0: LocalId,
        src1: LocalId,
        offset: isize,
    },

    // Global memory
    LoadGlobal {
        dst: LocalId,
        src: GlobalId,
    }, // [.0] = GLOBAL[.1]
    LoadGlobalIndirect {
        dst: LocalId,
        src: LocalId,
    }, // [.0] = GLOBAL[[.1]]
    StoreGlobal {
        dst: GlobalId,
        src: LocalId,
    }, // GLOBAL[.0] = [.1]
    StoreGlobalIndirect {
        dst: LocalId,
        src: LocalId,
    }, // GLOBAL[[.0]] = [.1]
    GlobalReadOnly(GlobalId), // make GLOBAL[.0] read-only

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
        prop: GlobalId,
    }, // [.0] = [.1].(.2) --- Equivalent to a.b
    LoadPropertyIndirect {
        dst: LocalId,
        src: LocalId,
        prop: LocalId,
    }, // [.0] = [.1][[.2]] --- Equivalent to a[b]
    LoadMethod {
        dst: LocalId,
        src: LocalId,
        prop: GlobalId,
    }, // [.0] = [.1]:(.2) --- Equivalent to a:b, returns closure that internally calls `a.b(a, ...)`
    StoreProperty {
        dst: LocalId,
        prop: GlobalId,
        src: LocalId,
    }, // [.0].1 = [.2] --- Equivalent to a.b = c
    StorePropertyIndirect {
        dst: LocalId,
        prop: LocalId,
        src: LocalId,
    }, // [.0][[.1]] = [.2] --- Equivalent to a[b] = c

    // Creating custom types
    LoadNil(LocalId),                // [.0] = nil
    LoadInt(LocalId, i64),           // [.0] = int(.1)
    LoadFloat(LocalId, f64),         // [.0] = float(.1)
    LoadStr(LocalId, GlobalId),      // [.0] = str(.1)
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
        ident: GlobalId,
        arity: u32,
    },

    // Return
    Return, // IP = POP()
}
