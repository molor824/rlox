use crate::interpreter::{bytecode::Bytecode, error::ErrorKind, LocalId, StrInterner, StringId};
use std::hash::Hash;
use std::{cell::RefCell, rc::Rc};
use rustc_hash::FxHashMap;

#[derive(Default, Debug, Clone)]
pub enum Value {
    #[default]
    Nil,
    Number(f64),
    String(StringId),
    Array(Rc<RefCell<Vec<Value>>>),
    Object(Rc<RefCell<Object>>),
    Closure(Rc<RefCell<Closure>>),
}

#[derive(Default, Debug)]
pub struct Object {
    map: FxHashMap<Value, Value>,
    super_obj: Option<Rc<RefCell<Object>>>,
}
impl Object {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            map: FxHashMap::with_capacity_and_hasher(capacity, Default::default()),
            ..Default::default()
        }
    }
    fn validate_key(key: &Value) -> Result<(), ErrorKind> {
        match key {
            Value::Nil => Err(ErrorKind::NilIndexing),
            Value::Number(num) if num.is_nan() => Err(ErrorKind::NanIndexing),
            _ => Ok(())
        }
    }
    pub fn get_property(&self, key: &Value) -> Result<Option<Value>, ErrorKind> {
        Self::validate_key(key)?;
        if let Some(value) = self.map.get(key) {
            Ok(Some(value.clone()))
        } else if let Some(super_obj) = &self.super_obj {
            super_obj.borrow().get_property(key)
        } else {
            Ok(None)
        }
    }
    pub fn set_property(&mut self, key: Value, new_value: Value) -> Result<(), ErrorKind> {
        Self::validate_key(&key)?;
        self.map.insert(key, new_value);
        Ok(())
    }
}

#[derive(Debug)]
pub struct Closure {
    min_arity: usize,
    variadic: bool,
    upvalues: Vec<LocalId>,
    body: Vec<Bytecode>,
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Self::Nil => matches!(other, Self::Nil),
            Self::Number(n1) => matches!(other, Self::Number(n2) if *n1 == *n2),
            Self::String(str1) => matches!(other, Self::String(str2) if *str1 == *str2),
            Self::Array(arr1) => matches!(
                other, Self::Array(arr2) if *arr1.borrow() == *arr2.borrow()
            ),
            Self::Object(obj1) => matches!(
                other, Self::Object(obj2) if Rc::ptr_eq(obj1, obj2)
            ),
            Self::Closure(fn1) => matches!(
                other, Self::Closure(fn2) if Rc::ptr_eq(fn1, fn2)
            ),
        }
    }
}
impl Eq for Value {}
impl Hash for Value {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u8(match self {
            Self::Nil => 0,
            Self::Number(_) => 1,
            Self::String(_) => 2,
            Self::Array(_) => 3,
            Self::Object(_) => 4,
            Self::Closure(_) => 5,
        });
        match self {
            Self::Nil => (),
            Self::Number(num) => num.to_bits().hash(state),
            Self::String(str) => str.hash(state),
            Self::Array(arr) => arr.borrow().hash(state),
            Self::Object(obj) => obj.as_ptr().hash(state),
            Self::Closure(closure) => closure.as_ptr().hash(state),
        }
    }
}
impl Value {
    fn type_str(&self) -> &'static str {
        match self {
            Self::Nil => "nil",
            Self::Number(_) => "number",
            Self::String(_) => "string",
            Self::Array(_) => "array",
            Self::Object(_) => "object",
            Self::Closure(_) => "function",
        }
    }
    pub fn try_add(&self, interner: &mut StrInterner, other: &Self) -> Result<Self, ErrorKind> {
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
                Self::String(rh) => {
                    let lh = interner.str(*lh).ok_or(ErrorKind::StringIdNotFound(*lh))?;
                    let rh = interner.str(*rh).ok_or(ErrorKind::StringIdNotFound(*rh))?;

                    let mut str = String::with_capacity(lh.len() + rh.len());
                    str.push_str(lh);
                    str.push_str(rh);

                    Ok(Value::String(interner.add_string(&str)))
                }
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
    pub fn try_str(&self) -> Result<StringId, ErrorKind> {
        match self {
            Self::String(str) => Ok(*str),
            _ => Err(ErrorKind::InvalidType(self.type_str()))
        }
    }
    pub fn try_obj(&self) -> Result<Rc<RefCell<Object>>, ErrorKind> {
        match self {
            Self::Object(obj) => Ok(obj.clone()),
            _ => Err(ErrorKind::InvalidType(self.type_str()))
        }
    }
}
