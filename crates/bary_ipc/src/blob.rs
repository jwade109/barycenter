use bary_core::prelude::{Components, TableIdent};
use bary_sim::World;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct Blob {
    data: Vec<u8>,
    ident: TableIdent,
}

impl Blob {
    pub fn new(data: Vec<u8>, ident: TableIdent) -> Self {
        Self { data, ident }
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn consume(self) -> Vec<u8> {
        self.data
    }

    pub fn table(&self) -> TableIdent {
        self.ident
    }
}

impl std::fmt::Debug for Blob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Blob({:?}, {} bytes)", self.ident, self.data.len())
    }
}

impl std::fmt::Display for Blob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Blob({:?}, {} bytes)", self.ident, self.data.len())
    }
}

fn unpack_blob<'a, T: Deserialize<'a>>(entities: &mut Components<T>, bytes: &'a [u8]) -> bool {
    if let Ok(e) = bincode::deserialize(bytes) {
        *entities = e;
        true
    } else {
        false
    }
}

pub fn apply_blob(world: &mut World, blob: Blob) -> bool {
    match blob.table() {
        TableIdent::Blueprints => unpack_blob(&mut world.blueprints, blob.data()),
        TableIdent::Grids => unpack_blob(&mut world.grids, blob.data()),
        TableIdent::Protos => unpack_blob(&mut world.prototypes, blob.data()),
        TableIdent::Parts => unpack_blob(&mut world.parts, blob.data()),
        TableIdent::Thrusters => unpack_blob(&mut world.thrusters, blob.data()),
        TableIdent::Computers => unpack_blob(&mut world.computers, blob.data()),
        TableIdent::Chunks => unpack_blob(&mut world.terrain_chunks, blob.data()),
        TableIdent::Tiles => unpack_blob(&mut world.terrain_tiles, blob.data()),
        TableIdent::Inventories => unpack_blob(&mut world.inventories, blob.data()),
        TableIdent::Machines => unpack_blob(&mut world.machines, blob.data()),
        TableIdent::Asteroids => unpack_blob(&mut world.asteroids, blob.data()),
        TableIdent::Pipes => unpack_blob(&mut world.pipes, blob.data()),
        TableIdent::Lights => unpack_blob(&mut world.lights, blob.data()),
        TableIdent::Excavators => unpack_blob(&mut world.excavators, blob.data()),
        TableIdent::Players => unpack_blob(&mut world.players, blob.data()),
        TableIdent::Planets => unpack_blob(&mut world.planets, blob.data()),
    }
}
