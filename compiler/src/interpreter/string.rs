use std::{cell::RefCell, fmt, hash::Hash, ops::Add, rc::Rc};

use rustc_hash::FxHashSet;

#[derive(Debug, Clone)]
pub enum ValueStr {
    Interned(InternedStr),
    Owned(Rc<str>),
}
impl ValueStr {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Interned(interned) => interned.0,
            Self::Owned(owned) => owned,
        }
    }
}
impl PartialEq for ValueStr {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Self::Interned(interned1) => match other {
                Self::Interned(interned2) => interned1 == interned2,
                Self::Owned(owned2) => interned1.0 == &**owned2,
            },
            Self::Owned(owned1) => match other {
                Self::Interned(interned2) => &**owned1 == interned2.0,
                Self::Owned(owned2) => owned1 == owned2,
            },
        }
    }
}
impl Eq for ValueStr {}
impl Hash for ValueStr {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}
impl Add for &ValueStr {
    type Output = ValueStr;
    fn add(self, rhs: Self) -> Self::Output {
        ValueStr::Owned((self.as_str().to_string() + rhs.as_str()).into())
    }
}
impl fmt::Display for ValueStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Default)]
pub struct StrInterner {
    strings: FxHashSet<&'static str>,
}
impl StrInterner {
    fn add_str(&mut self, str: &str) -> InternedStr {
        match self.strings.get(str) {
            Some(interned) => InternedStr(*interned),
            None => {
                let str = str.to_string().leak();
                self.strings.insert(str);
                InternedStr(str)
            }
        }
    }
}

thread_local! {
    static THREAD_INTERNER: RefCell<StrInterner> = RefCell::new(StrInterner::default());
}

#[derive(Debug, Clone, Copy)]
pub struct InternedStr(&'static str);
impl PartialEq for InternedStr {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}
impl Eq for InternedStr {}
impl Hash for InternedStr {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}
impl From<&str> for InternedStr {
    fn from(value: &str) -> Self {
        THREAD_INTERNER.with(|interner| interner.borrow_mut().add_str(&value))
    }
}
