use std::{collections::BTreeMap, fmt::Debug, rc::Rc};

pub struct BidiMap<A, B> {
    a_b_map: BTreeMap<Rc<A>, Rc<B>>,
    b_a_map: BTreeMap<Rc<B>, Rc<A>>,
}

impl<A: Debug, B: Debug> Debug for BidiMap<A, B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.a_b_map)
    }
}

impl<A: Eq + PartialEq + Ord, B: Eq + PartialEq + Ord> BidiMap<A, B> {
    pub fn new() -> Self {
        BidiMap {
            a_b_map: BTreeMap::new(),
            b_a_map: BTreeMap::new(),
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

    pub fn remove_by_b(&mut self, b: &B) -> Option<(A, B)> {
        let a_rc = self.b_a_map.remove(b)?;
        let b_rc = self.a_b_map.remove(a_rc.as_ref())?;
        match (Rc::try_unwrap(a_rc), Rc::try_unwrap(b_rc)) {
            (Ok(a), Ok(b)) => Some((a, b)),
            (Err(a_rc), Err(b_rc)) => {
                self.a_b_map.insert(a_rc.clone(), b_rc.clone());
                self.b_a_map.insert(b_rc, a_rc);
                None
            }
            _ => unreachable!(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&A, &B)> {
        self.a_b_map.iter().map(|(a, b)| (a.as_ref(), b.as_ref()))
    }
}
