use std::{borrow::Borrow, collections::HashMap, hash::Hash, rc::Rc};

pub struct Cache<T: ?Sized> {
    ids: HashMap<usize, Rc<T>>,
    datas: HashMap<Rc<T>, usize>,
    id_count: usize,
}
impl<T: ?Sized> Cache<T> {
    pub fn new() -> Self {
        Self {
            ids: HashMap::new(),
            datas: HashMap::new(),
            id_count: 0,
        }
    }
}
impl<T: Eq + Hash> Cache<T> {
    pub fn insert(&mut self, value: T) -> usize {
        match self.datas.get(&value) {
            Some(&id) => id,
            None => {
                self.id_count += 1;
                let data = Rc::new(value);
                self.datas.insert(data.clone(), self.id_count);
                self.ids.insert(self.id_count, data);
                self.id_count
            }
        }
    }
}
impl<T: ?Sized + Eq + Hash> Cache<T> {
    pub fn insert_rc(&mut self, value: Rc<T>) -> usize {
        match self.datas.get(&value) {
            Some(&id) => id,
            None => {
                self.id_count += 1;
                self.datas.insert(value.clone(), self.id_count);
                self.ids.insert(self.id_count, value);
                self.id_count
            }
        }
    }
    pub fn get_data(&self, id: usize) -> Option<&T> {
        self.ids.get(&id).map(|rc| rc.as_ref())
    }
    pub fn get_id<Q>(&self, data: &Q) -> Option<usize>
    where
        Rc<T>: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        self.datas.get(data).copied()
    }
}
