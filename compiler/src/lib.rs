use std::sync::atomic::AtomicBool;

pub mod ast;
pub mod codegen;
pub mod error;
pub mod interpreter;
pub mod span;

pub static DEBUG_MODE: AtomicBool = AtomicBool::new(false);
