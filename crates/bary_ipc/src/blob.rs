use std::collections::BTreeMap;

use bary_core::prelude::{Components, Ent, TableIdent};
use bary_sim::World;
use serde::{Deserialize};

#[derive(Clone)]
pub struct Blob {
    payload: BTreeMap<Ent, Vec<u8>>,
    ident: TableIdent,
}

impl Blob {
    pub fn new(ident: TableIdent, payload: BTreeMap<Ent, Vec<u8>>) -> Self {
        Self { payload, ident }
    }

    pub fn add_entity(&mut self, id: Ent, data: Vec<u8>) {
        self.payload.insert(id, data);
    }

    pub fn payload(&self) -> &BTreeMap<Ent, Vec<u8>> {
        &self.payload
    }

    pub fn consume(self) -> BTreeMap<Ent, Vec<u8>> {
        self.payload
    }

    pub fn table(&self) -> TableIdent {
        self.ident
    }
}

impl std::fmt::Debug for Blob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Blob({:?}, {} entities)", self.ident, self.payload.len())
    }
}

impl std::fmt::Display for Blob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Blob({:?}, {} entities)", self.ident, self.payload.len())
    }
}

fn unpack_blob<'a, T: Deserialize<'a>>(
    entities: &mut Components<T>,
    data: &'a BTreeMap<Ent, Vec<u8>>,
) -> bool {
    entities.clear();

    for (id, data) in data {
        if let Ok(e) = bincode::deserialize(data) {
            entities.spawn(*id, e);
        } else {
            return false;
        }
    }
    true
}
