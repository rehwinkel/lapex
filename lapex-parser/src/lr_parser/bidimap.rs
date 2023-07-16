use std::{collections::HashMap, fmt::Debug, hash::Hash, rc::Rc};

pub struct BidiMap<A, B> {
    a_b_map: HashMap<Rc<A>, Rc<B>>,
    b_a_map: HashMap<Rc<B>, Rc<A>>,
}

impl<A: Debug, B: Debug> Debug for BidiMap<A, B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.a_b_map)
    }
}

impl<A: Eq + PartialEq + Hash, B: Eq + PartialEq + Hash> BidiMap<A, B> {
    pub fn new() -> Self {
        BidiMap {
            a_b_map: HashMap::new(),
            b_a_map: HashMap::new(),
        }
    }

    pub fn insert(&mut self, a: A, b: B) {
        let a_rc = Rc::new(a);
        let b_rc = Rc::new(b);
        self.a_b_map.insert(a_rc.clone(), b_rc.clone());
        self.b_a_map.insert(b_rc, a_rc);
    }

    pub fn get_a_to_b(&self, a: &A) -> Option<&B> {
        self.a_b_map.get(a).map(|rc| rc.as_ref())
    }

    pub fn get_b_to_a(&self, b: &B) -> Option<&A> {
        self.b_a_map.get(b).map(|rc| rc.as_ref())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&A, &B)> {
        self.a_b_map.iter().map(|(a, b)| (a.as_ref(), b.as_ref()))
    }
}
