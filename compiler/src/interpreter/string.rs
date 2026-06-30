use std::{
    cell::RefCell,
    collections::HashMap,
    fmt,
    hash::{Hash, Hasher},
    ops::Add,
    rc::Rc,
};

use rustc_hash::FxHasher;

#[derive(Clone, PartialEq, Eq)]
pub struct IndexableStr {
    kind: StringKind,
    hash: u64,
}
impl IndexableStr {
    pub fn len(&self) -> usize {
        match &self.kind {
            StringKind::Utf8(utf8) => utf8.len(),
            StringKind::Utf16(utf16) => utf16.len(),
            StringKind::Utf32(utf32) => utf32.len(),
        }
    }
}
impl<S: AsRef<str>> From<S> for IndexableStr {
    fn from(value: S) -> Self {
        StringKind::from(value).into()
    }
}
impl From<StringKind> for IndexableStr {
    fn from(value: StringKind) -> Self {
        let mut hasher = FxHasher::default();
        match &value {
            StringKind::Utf8(utf) => <[u8]>::hash(utf, &mut hasher),
            StringKind::Utf16(utf) => <[u16]>::hash(utf, &mut hasher),
            StringKind::Utf32(utf) => <[u32]>::hash(utf, &mut hasher),
        }
        Self {
            kind: value,
            hash: hasher.finish(),
        }
    }
}
impl From<ValueStr> for IndexableStr {
    fn from(value: ValueStr) -> Self {
        match value {
            ValueStr::Interned(interned) => interned.0.clone(),
            ValueStr::Owned(owned) => owned,
        }
    }
}
impl Hash for IndexableStr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash);
    }
}
impl fmt::Display for IndexableStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}
impl fmt::Debug for IndexableStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.kind)
    }
}
impl Add for &IndexableStr {
    type Output = IndexableStr;
    fn add(self, rhs: &IndexableStr) -> Self::Output {
        (&self.kind + &rhs.kind).into()
    }
}

#[derive(Clone, PartialEq, Eq)]
enum StringKind {
    Utf8(Rc<Vec<u8>>),
    Utf16(Rc<Vec<u16>>),
    Utf32(Rc<Vec<u32>>),
}
impl StringKind {
    fn from_str_as_utf8(str: &str) -> Self {
        Self::Utf8(Rc::new(str.as_bytes().to_vec()))
    }
    fn from_str_as_utf16(str: &str) -> Self {
        Self::Utf16(Rc::new(str.encode_utf16().collect()))
    }
    fn from_str_as_utf32(str: &str) -> Self {
        Self::Utf32(Rc::new(str.chars().map(|ch| ch as u32).collect()))
    }
    fn concat_utf8(&self, utf8: &[u8]) -> Self {
        match self {
            Self::Utf8(data) => Self::Utf8(Rc::new(
                data.iter().copied().chain(utf8.iter().copied()).collect(),
            )),
            Self::Utf16(data) => Self::Utf16(Rc::new(
                data.iter()
                    .copied()
                    .chain(utf8.iter().map(|i| *i as u16))
                    .collect(),
            )),
            Self::Utf32(data) => Self::Utf32(Rc::new(
                data.iter()
                    .copied()
                    .chain(utf8.iter().map(|i| *i as u32))
                    .collect(),
            )),
        }
    }
    fn concat_utf16(&self, utf16: &[u16]) -> Self {
        match self {
            Self::Utf8(data) => Self::Utf16(Rc::new(
                data.iter()
                    .map(|d| *d as u16)
                    .chain(utf16.iter().copied())
                    .collect(),
            )),
            Self::Utf16(data) => Self::Utf16(Rc::new(
                data.iter().copied().chain(utf16.iter().copied()).collect(),
            )),
            Self::Utf32(data) => Self::Utf32(Rc::new(
                data.iter()
                    .copied()
                    .chain(utf16.iter().map(|d| *d as u32))
                    .collect(),
            )),
        }
    }
    fn concat_utf32(&self, utf32: &[u32]) -> Self {
        match self {
            Self::Utf8(data) => Self::Utf32(Rc::new(
                data.iter()
                    .map(|d| *d as u32)
                    .chain(utf32.iter().copied())
                    .collect(),
            )),
            Self::Utf16(data) => Self::Utf32(Rc::new(
                data.iter()
                    .map(|d| *d as u32)
                    .chain(utf32.iter().copied())
                    .collect(),
            )),
            Self::Utf32(data) => Self::Utf32(Rc::new(
                data.iter().copied().chain(utf32.iter().copied()).collect(),
            )),
        }
    }
}
impl Add for &StringKind {
    type Output = StringKind;
    fn add(self, rhs: &StringKind) -> Self::Output {
        match rhs {
            StringKind::Utf8(utf8) => self.concat_utf8(utf8),
            StringKind::Utf16(utf16) => self.concat_utf16(utf16),
            StringKind::Utf32(utf32) => self.concat_utf32(utf32),
        }
    }
}
impl<S: AsRef<str>> From<S> for StringKind {
    fn from(value: S) -> Self {
        let str = value.as_ref();
        let max_char = str.chars().max().unwrap_or('\0');
        if max_char.len_utf8() == 1 {
            Self::from_str_as_utf8(str)
        } else if max_char.len_utf16() == 1 {
            Self::from_str_as_utf16(str)
        } else {
            Self::from_str_as_utf32(str)
        }
    }
}
impl fmt::Display for StringKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Utf8(str) => write!(f, "{}", unsafe { str::from_utf8_unchecked(str) }),
            Self::Utf16(str) => write!(f, "{}", String::from_utf16_lossy(str)),
            Self::Utf32(str) => write!(
                f,
                "{}",
                str.iter()
                    .map(|ch| unsafe { char::from_u32_unchecked(*ch) })
                    .collect::<String>()
            ),
        }
    }
}
impl fmt::Debug for StringKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut tuple = match self {
            Self::Utf8(_) => f.debug_tuple("Utf8"),
            Self::Utf16(_) => f.debug_tuple("Utf16"),
            Self::Utf32(_) => f.debug_tuple("Utf32"),
        };
        tuple.field(&self.to_string()).finish()
    }
}

#[derive(Debug, Clone)]
pub enum ValueStr {
    Interned(InternedStr),
    Owned(IndexableStr),
}
impl ValueStr {
    pub fn indexable_str(&self) -> &IndexableStr {
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
                Self::Owned(owned2) => interned1.0 == owned2,
            },
            Self::Owned(owned1) => match other {
                Self::Interned(interned2) => owned1 == interned2.0,
                Self::Owned(owned2) => owned1 == owned2,
            },
        }
    }
}
impl Eq for ValueStr {}
impl Hash for ValueStr {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.indexable_str().hash(state);
    }
}
impl Add for &ValueStr {
    type Output = ValueStr;
    fn add(self, rhs: Self) -> Self::Output {
        ValueStr::Owned(&IndexableStr::from(self.clone()) + &IndexableStr::from(rhs.clone()))
    }
}
impl fmt::Display for ValueStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.indexable_str())
    }
}

#[derive(Default)]
pub struct StrInterner {
    strings: HashMap<&'static IndexableStr, InternedStr>,
}
impl StrInterner {
    fn add_str(&mut self, str: &IndexableStr) -> InternedStr {
        match self.strings.get(str) {
            Some(interned) => *interned,
            None => {
                let str = Box::leak(Box::new(str.clone()));
                self.strings.insert(str, InternedStr(str));
                InternedStr(str)
            }
        }
    }
    // fn get_str(&self, str: &IndexableStr) -> Option<InternedStr> {
    //     self.strings.get(str).copied()
    // }
}

thread_local! {
    static THREAD_INTERNER: RefCell<StrInterner> = RefCell::new(StrInterner::default());
}

#[derive(Debug, Clone, Copy)]
pub struct InternedStr(&'static IndexableStr);
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
impl From<IndexableStr> for InternedStr {
    fn from(value: IndexableStr) -> Self {
        THREAD_INTERNER.with(|interner| interner.borrow_mut().add_str(&value))
    }
}
