use std::{
    cell::RefCell,
    fmt,
    hash::{Hash, Hasher},
    ops::Add,
    rc::Rc,
};

use rustc_hash::{FxHashSet, FxHasher};

#[derive(Clone)]
pub struct ValueStr {
    hash: u64,
    str: Rc<str>,
    interned: bool,
}
impl ValueStr {
    fn new(str: Rc<str>, interned: bool) -> Self {
        let mut hasher = FxHasher::default();
        str.hash(&mut hasher);
        Self {
            hash: hasher.finish(),
            str,
            interned,
        }
    }
    pub fn interned(string: &str) -> Self {
        Self::new(INTERNER.with(|i| i.borrow_mut().add_str(string)), true)
    }
    pub fn as_str(&self) -> &str {
        &self.str
    }
}
impl fmt::Debug for ValueStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.str)
    }
}
impl From<Rc<str>> for ValueStr {
    fn from(value: Rc<str>) -> Self {
        Self::new(value, false)
    }
}
impl From<&str> for ValueStr {
    fn from(value: &str) -> Self {
        Rc::<str>::from(value).into()
    }
}
impl PartialEq for ValueStr {
    fn eq(&self, other: &Self) -> bool {
        if self.interned && other.interned {
            return Rc::ptr_eq(&self.str, &other.str);
        }
        self.str == other.str
    }
}
impl Eq for ValueStr {}
impl Hash for ValueStr {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
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
        write!(f, "{}", self.str)
    }
}

#[derive(Default)]
pub struct StrInterner {
    strings: FxHashSet<Rc<str>>,
}
impl StrInterner {
    pub fn add_str(&mut self, str: &str) -> Rc<str> {
        match self.strings.get(str) {
            Some(str) => str.clone(),
            None => {
                let rc_str = Rc::<str>::from(str);
                self.strings.insert(rc_str.clone());
                rc_str
            }
        }
    }
}

thread_local! {
    static INTERNER: RefCell<StrInterner> = RefCell::new(StrInterner::default());
}
