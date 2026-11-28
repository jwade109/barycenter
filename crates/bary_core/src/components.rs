use crate::{
    entity::Ent,
    result::{BaryError, BaryResult},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EntitySpawner {
    next_id: Ent,
}

impl Default for EntitySpawner {
    fn default() -> Self {
        Self { next_id: Ent(0) }
    }
}

impl EntitySpawner {
    pub fn spawn(&mut self) -> Ent {
        let ret = self.next_id;
        self.next_id.0 += 1;
        ret
    }

    pub fn next(&self) -> Ent {
        self.next_id
    }

    pub fn set_next(&mut self, e: Ent) {
        self.next_id = e;
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Components<E> {
    values: BTreeMap<Ent, E>,
}

impl<E> Default for Components<E> {
    fn default() -> Self {
        Self {
            values: BTreeMap::default(),
        }
    }
}

impl<E> Components<E> {
    pub fn spawn(&mut self, id: Ent, e: E) {
        self.values.insert(id, e);
    }

    pub fn update(&mut self, id: Ent, e: E) {
        assert!(self.values.contains_key(&id));
        self.values.insert(id, e);
    }

    pub fn despawn(&mut self, id: Ent) -> BaryResult<E> {
        let e = self.values.remove(&id);
        if let Some(e) = e {
            Ok(e)
        } else {
            Err(BaryError::EntityNotFound(id))
        }
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn has_entity(&self, id: Ent) -> bool {
        self.values.contains_key(&id)
    }

    pub fn clear(&mut self) {
        self.values.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Ent, &E)> {
        self.values.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&Ent, &mut E)> {
        self.values.iter_mut()
    }

    pub fn values(&self) -> impl Iterator<Item = &E> {
        self.values.values()
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut E> {
        self.values.values_mut()
    }

    pub fn get(&self, id: Ent) -> Option<&E> {
        self.values.get(&id)
    }

    pub fn require(&self, id: Ent) -> &E {
        self.values.get(&id).unwrap()
    }

    pub fn require_mut(&mut self, id: Ent) -> &E {
        self.values.get_mut(&id).unwrap()
    }

    pub fn try_get(&self, id: Ent) -> BaryResult<&E> {
        self.values.get(&id).ok_or(BaryError::EntityNotFound(id))
    }

    pub fn try_get_mut(&mut self, id: Ent) -> BaryResult<&mut E> {
        self.values
            .get_mut(&id)
            .ok_or(BaryError::EntityNotFound(id))
    }

    pub fn retain<F>(&mut self, func: F)
    where
        F: FnMut(&Ent, &mut E) -> bool,
    {
        self.values.retain(func);
    }
}

impl<E> std::fmt::Debug for Components<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} components", self.values.len(),)
    }
}
