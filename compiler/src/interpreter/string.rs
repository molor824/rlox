use std::{
    cell::RefCell,
    fmt,
    hash::{Hash, Hasher},
    ops::Add,
    rc::Rc,
};

use rustc_hash::{FxHashMap, FxHasher};

#[derive(Clone)]
pub struct ValueStr(u64, Rc<str>);
impl ValueStr {
    pub fn interned(string: &str) -> ValueStr {
        INTERNER.with(|interner| interner.borrow_mut().add_str(string))
    }
    pub fn as_str(&self) -> &str {
        &self.1
    }
}
impl fmt::Debug for ValueStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.1)
    }
}
impl From<Rc<str>> for ValueStr {
    fn from(value: Rc<str>) -> Self {
        let mut hasher = FxHasher::default();
        value.hash(&mut hasher);
        Self(hasher.finish(), value)
    }
}
impl From<&str> for ValueStr {
    fn from(value: &str) -> Self {
        Rc::<str>::from(value).into()
    }
}
impl PartialEq for ValueStr {
    fn eq(&self, other: &Self) -> bool {
        self.1 == other.1
    }
}
impl Eq for ValueStr {}
impl Hash for ValueStr {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}
impl Add for &ValueStr {
    type Output = ValueStr;
    fn add(self, rhs: Self) -> Self::Output {
        let mut str = String::with_capacity(self.as_str().len() + rhs.as_str().len());
        str.push_str(self.as_str());
        str.push_str(rhs.as_str());
        ValueStr::from(str.as_str())
    }
}
impl fmt::Display for ValueStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.1)
    }
}

#[derive(Default)]
pub struct StrInterner {
    strings: FxHashMap<Rc<str>, ValueStr>,
}
impl StrInterner {
    pub fn add_str(&mut self, str: &str) -> ValueStr {
        match self.strings.get(str) {
            Some(str) => str.clone(),
            None => {
                let val_str = ValueStr::from(str);
                self.strings.insert(val_str.1.clone(), val_str.clone());
                val_str
            }
        }
    }
}

thread_local! {
    static INTERNER: RefCell<StrInterner> = RefCell::new(StrInterner::default());
}
