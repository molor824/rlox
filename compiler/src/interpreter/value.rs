use std::{cell::RefCell, collections::HashMap, rc::Rc};

use num_bigint::BigInt;

use crate::interpreter::{Closure, GlobalId};

#[derive(Debug, Clone)]
pub enum Value {
    Nil,
    Integer(Rc<BigInt>),
    Float(f64),
    String(GlobalId),
    Array(Rc<RefCell<Vec<Value>>>),
    Object(Rc<RefCell<HashMap<Value, Value>>>),
    Closure(Rc<RefCell<Closure>>),
}
