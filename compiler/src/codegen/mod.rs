use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    ast::expression::FunctionDecl,
    interpreter::{string::InternedStr, LocalId},
};

#[derive(Default)]
pub struct FunctionState {
    local_ids: FxHashMap<InternedStr, LocalId>,
    upvalue_ids: FxHashMap<LocalId, LocalId>, // Maps parent function's local id to upvalue id
    parent_upvalues: FxHashSet<LocalId>,      // Set of parent function's upvalues
    children: Vec<FunctionState>,             // Inner functions
}
impl FunctionState {
    pub fn from_decl(decl: &FunctionDecl, parents: &[&FunctionState]) -> FunctionState {
        todo!()
    }
}
