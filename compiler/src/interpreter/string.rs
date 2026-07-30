use std::{
    fmt,
    hash::{Hash, Hasher},
    ops::Add,
    rc::Rc,
    sync::{LazyLock, Mutex},
};

use rustc_hash::{FxHashMap, FxHasher};

#[derive(Debug, Clone)]
pub enum ValueStr {
    Interned(InternedStr),
    Owned(CachedHash<Rc<str>>),
}
impl ValueStr {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Interned(interned) => &interned.0 .1,
            Self::Owned(owned) => &owned.1,
        }
    }
}
impl PartialEq for ValueStr {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Self::Interned(interned1) => match other {
                Self::Interned(interned2) => interned1 == interned2,
                Self::Owned(owned2) => interned1.0 .1 == &*owned2.1,
            },
            Self::Owned(owned1) => match other {
                Self::Interned(interned2) => &*owned1.1 == interned2.0 .1,
                Self::Owned(owned2) => owned1 == owned2,
            },
        }
    }
}
impl Eq for ValueStr {}
impl Hash for ValueStr {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            ValueStr::Interned(i) => i.0.hash(state),
            ValueStr::Owned(o) => o.hash(state),
        }
    }
}
impl Add for &ValueStr {
    type Output = ValueStr;
    fn add(self, rhs: Self) -> Self::Output {
        ValueStr::Owned(CachedHash::from(Rc::from(
            self.as_str().to_string() + rhs.as_str(),
        )))
    }
}
impl fmt::Display for ValueStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Default)]
pub struct StrInterner {
    strings: FxHashMap<&'static str, InternedStr>,
}
impl StrInterner {
    fn add_str(&mut self, str: &str) -> InternedStr {
        match self.strings.get(str) {
            Some(interned) => *interned,
            None => {
                let str = str.to_string().leak() as &str;
                let interned = InternedStr(str.into());
                self.strings.insert(str, interned);
                interned
            }
        }
    }
}

static INTERNER: LazyLock<Mutex<StrInterner>> =
    LazyLock::new(|| Mutex::new(StrInterner::default()));

#[derive(Clone, Copy)]
pub struct CachedHash<T>(u64, T);
impl<T: Hash> From<T> for CachedHash<T> {
    fn from(value: T) -> Self {
        let mut hasher = FxHasher::default();
        value.hash(&mut hasher);
        Self(hasher.finish(), value)
    }
}
impl<T: fmt::Debug> fmt::Debug for CachedHash<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.1)
    }
}
impl<T: fmt::Display> fmt::Display for CachedHash<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.1)
    }
}
impl<T: Eq> Eq for CachedHash<T> {}
impl<T: PartialEq> PartialEq for CachedHash<T> {
    fn eq(&self, other: &Self) -> bool {
        &self.1 == &other.1
    }
}
impl<T> Hash for CachedHash<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct InternedStr(CachedHash<&'static str>);
impl PartialEq for InternedStr {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0 .1, other.0 .1)
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
        let mut interner = INTERNER.lock().unwrap();
        interner.add_str(value)
    }
}
impl fmt::Display for InternedStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
