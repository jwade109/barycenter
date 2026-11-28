use std::collections::BTreeMap;

use bary_core::prelude::{Ent, TableIdent};

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
