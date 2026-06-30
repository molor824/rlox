use crate::interpreter::error::ErrorKind;
use crate::interpreter::string::ValueStr;
use crate::interpreter::FnSignature;
use rustc_hash::FxHashMap;
use std::cmp::Ordering;
use std::fmt;
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
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nil => write!(f, "nil"),
            Self::Bool(bool) => write!(f, "{}", bool),
            Self::Number(num) => write!(f, "{}", num),
            Self::String(str) => write!(f, "{}", str),
            Self::Array(arr) => {
                write!(f, "[")?;
                for (i, elem) in arr.borrow().iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", elem)?;
                }
                write!(f, "]")
            }
            Self::Object(obj) => {
                write!(f, "{{")?;
                for (i, (key, value)) in obj.borrow().map.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", key, value)?;
                }
                write!(f, "}}")
            }
            Self::Function(fun) => {
                write!(
                    f,
                    "fn({}{})[",
                    fun.signature.arity,
                    if fun.signature.variadic { "..." } else { "" }
                )?;
                for (i, upvalue) in fun.upvalues.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{:?}:{}", upvalue.as_ptr(), upvalue.borrow())?;
                }
                write!(f, "]")
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Function {
    pub signature: Rc<FnSignature>,
    pub upvalues: Vec<Rc<RefCell<Value>>>,
}

#[derive(Debug)]
pub struct Object {
    map: FxHashMap<Value, Value>,
    super_obj: Option<Rc<RefCell<Object>>>,
}
impl Object {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            map: FxHashMap::with_capacity_and_hasher(capacity, Default::default()),
            super_obj: None,
        }
    }
    pub fn get_property(&self, key: &Value) -> Result<Value, ErrorKind> {
        if let Some(value) = self.map.get(key) {
            Ok(value.clone())
        } else if let Some(super_obj) = &self.super_obj {
            super_obj.borrow().get_property(key)
        } else {
            Ok(Value::Nil)
        }
    }
    pub fn set_property(&mut self, key: Value, new_value: Value) -> Result<(), ErrorKind> {
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
            _ => Err(ErrorKind::InvalidType(self.clone(), "string")),
        }
    }
    pub fn get_property(&self, key: &Value) -> Result<Value, ErrorKind> {
        match self {
            Self::Array(array) => match key {
                Self::Number(index) => Ok(array
                    .borrow()
                    .get(*index as usize)
                    .cloned()
                    .unwrap_or_default()),
                _ => Err(ErrorKind::InvalidArrayIndex),
            },
            Self::Object(obj) => Ok(obj.borrow().get_property(key)?),
            _ => Err(ErrorKind::InvalidPropertyAccess),
        }
    }
    pub fn set_property(&self, key: Value, new_value: Value) -> Result<(), ErrorKind> {
        match self {
            Self::Array(array) => match key {
                Self::Number(index) => {
                    let index = index as usize;
                    let mut array = array.borrow_mut();
                    if array.len() <= index {
                        array.resize_with(index + 1, Default::default);
                    }
                    array[index] = new_value;
                    Ok(())
                }
                _ => Err(ErrorKind::InvalidArrayIndex),
            },
            Self::Object(obj) => Ok(obj.borrow_mut().set_property(key, new_value)?),
            _ => Err(ErrorKind::InvalidPropertyAccess),
        }
    }
    pub fn as_callable(&self) -> Result<Rc<Function>, ErrorKind> {
        match self {
            Self::Function(fun) => Ok(fun.clone()),
            _ => Err(ErrorKind::InvalidType(self.clone(), "function")),
        }
    }
    pub fn as_bool(&self) -> bool {
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
