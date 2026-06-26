use crate::interpreter::error::ErrorKind;
use crate::interpreter::string::ValueStr;
use crate::interpreter::FnSignature;
use rustc_hash::FxHashMap;
use std::cmp::Ordering;
use std::hash::Hash;
use std::{cell::RefCell, rc::Rc};

#[derive(Default, Debug, Clone)]
pub enum Value {
    #[default]
    Nil,
    Bool(bool),
    Number(f64),
    String(ValueStr),
    Array(Rc<RefCell<Vec<Value>>>),
    Object(Rc<RefCell<Object>>),
    Function(Rc<Function>),
}

#[derive(Debug, Clone)]
pub struct Function {
    pub signature: Rc<FnSignature>,
    pub upvalues: Vec<Rc<RefCell<Value>>>,
}

#[derive(Default, Debug)]
pub struct Object {
    map: FxHashMap<ValueStr, Value>,
    super_obj: Option<Rc<RefCell<Object>>>,
}
impl Object {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            map: FxHashMap::with_capacity_and_hasher(capacity, Default::default()),
            ..Default::default()
        }
    }
    pub fn get_property(&self, key: &ValueStr) -> Result<Value, ErrorKind> {
        if let Some(value) = self.map.get(key) {
            Ok(value.clone())
        } else if let Some(super_obj) = &self.super_obj {
            super_obj.borrow().get_property(key)
        } else {
            Ok(Value::Nil)
        }
    }
    pub fn set_property(&mut self, key: ValueStr, new_value: Value) -> Result<(), ErrorKind> {
        self.map.insert(key, new_value);
        Ok(())
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Self::Nil => matches!(other, Self::Nil),
            Self::Bool(b1) => matches!(other, Self::Bool(b2) if b1 == b2),
            Self::Number(n1) => matches!(other, Self::Number(n2) if n1 == n2),
            Self::String(str1) => matches!(other, Self::String(str2) if str1 == str2),
            Self::Array(arr1) => matches!(
                other, Self::Array(arr2) if *arr1.borrow() == *arr2.borrow()
            ),
            Self::Object(obj1) => matches!(
                other, Self::Object(obj2) if Rc::ptr_eq(obj1, obj2)
            ),
            Self::Function(fn1) => matches!(
                other, Self::Function(fn2) if Rc::ptr_eq(fn1, fn2)
            ),
        }
    }
}
impl Eq for Value {}
impl Hash for Value {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u8(match self {
            Self::Nil => 0,
            Self::Bool(_) => 1,
            Self::Number(_) => 2,
            Self::String(_) => 3,
            Self::Array(_) => 4,
            Self::Object(_) => 5,
            Self::Function(_) => 6,
        });
        match self {
            Self::Nil => (),
            Self::Bool(b) => b.hash(state),
            Self::Number(num) => num.to_bits().hash(state),
            Self::String(str) => str.hash(state),
            Self::Array(arr) => arr.borrow().hash(state),
            Self::Object(obj) => Rc::as_ptr(obj).hash(state),
            Self::Function(function) => Rc::as_ptr(function).hash(state),
        }
    }
}
impl Value {
    fn type_str(&self) -> &'static str {
        match self {
            Self::Nil => "nil",
            Self::Bool(_) => "boolean",
            Self::Number(_) => "number",
            Self::String(_) => "string",
            Self::Array(_) => "array",
            Self::Object(_) => "object",
            Self::Function(_) => "function",
        }
    }
    pub fn try_add(&self, other: &Self) -> Result<Self, ErrorKind> {
        let error = || {
            Err(ErrorKind::InvalidBinary(
                "+",
                self.type_str(),
                other.type_str(),
            ))
        };
        match self {
            Self::Number(lh) => match other {
                Self::Number(rh) => Ok(Self::Number(*lh + *rh)),
                _ => error(),
            },
            Self::Array(lh) => match other {
                Self::Array(rh) => {
                    let mut array = lh.borrow().clone();
                    array.extend_from_slice(&rh.borrow());
                    Ok(Self::Array(Rc::new(RefCell::new(array))))
                }
                _ => error(),
            },
            Self::String(lh) => match other {
                Self::String(rh) => Ok(Value::String(lh + rh)),
                _ => error(),
            },
            _ => error(),
        }
    }
    pub fn try_sub(&self, other: &Self) -> Result<Self, ErrorKind> {
        let error = || {
            Err(ErrorKind::InvalidBinary(
                "-",
                self.type_str(),
                other.type_str(),
            ))
        };
        match self {
            Self::Number(lh) => match other {
                Self::Number(rh) => Ok(Self::Number(*lh - *rh)),
                _ => error(),
            },
            _ => error(),
        }
    }
    pub fn try_mul(&self, other: &Self) -> Result<Self, ErrorKind> {
        let error = || {
            Err(ErrorKind::InvalidBinary(
                "*",
                self.type_str(),
                other.type_str(),
            ))
        };
        match self {
            Self::Number(lh) => match other {
                Self::Number(rh) => Ok(Self::Number(*lh * *rh)),
                _ => error(),
            },
            _ => error(),
        }
    }
    pub fn try_div(&self, other: &Self) -> Result<Self, ErrorKind> {
        let error = || {
            Err(ErrorKind::InvalidBinary(
                "/",
                self.type_str(),
                other.type_str(),
            ))
        };
        match self {
            Self::Number(lh) => match other {
                Self::Number(rh) => Ok(Self::Number(*lh / *rh)),
                _ => error(),
            },
            _ => error(),
        }
    }
    pub fn try_rem(&self, other: &Self) -> Result<Self, ErrorKind> {
        let error = || {
            Err(ErrorKind::InvalidBinary(
                "%",
                self.type_str(),
                other.type_str(),
            ))
        };
        match self {
            Self::Number(lh) => match other {
                Self::Number(rh) => Ok(Self::Number(*lh % *rh)),
                _ => error(),
            },
            _ => error(),
        }
    }
    pub fn try_neg(&self) -> Result<Value, ErrorKind> {
        let error = || Err(ErrorKind::InvalidUnary("-", self.type_str()));
        match self {
            Self::Number(n) => Ok(Self::Number(-*n)),
            _ => error(),
        }
    }
    pub fn try_inv(&self) -> Result<Value, ErrorKind> {
        let error = || Err(ErrorKind::InvalidUnary("~", self.type_str()));
        match self {
            Self::Number(n) => Ok(Self::Number(!(*n as i64) as f64)),
            _ => error(),
        }
    }
    pub fn try_cmp(&self, other: &Self) -> Result<Option<Ordering>, ErrorKind> {
        let error = || {
            Err(ErrorKind::InvalidBinary(
                "< > <= >=",
                self.type_str(),
                other.type_str(),
            ))
        };
        match self {
            Self::Number(n1) => match other {
                Self::Number(n2) => Ok(n1.partial_cmp(n2)),
                _ => error(),
            },
            _ => error(),
        }
    }
    pub fn try_str(&self) -> Result<ValueStr, ErrorKind> {
        match self {
            Self::String(str) => Ok(str.clone()),
            _ => Err(ErrorKind::InvalidType(self.type_str())),
        }
    }
    pub fn try_obj(&self) -> Result<Rc<RefCell<Object>>, ErrorKind> {
        match self {
            Self::Object(obj) => Ok(obj.clone()),
            _ => Err(ErrorKind::InvalidType(self.type_str())),
        }
    }
    pub fn to_bool(&self) -> bool {
        match self {
            Self::Nil => false,
            Self::Number(num) => *num != 0.0,
            Self::Array(array) => !array.borrow().is_empty(),
            Self::String(str) => str.indexable_str().len() != 0,
            Self::Bool(bool) => *bool,
            Self::Object(_) | Self::Function(_) => true,
        }
    }
}
