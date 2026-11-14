use bary_core::prelude::{Components, Ent, TableIdent};
use bary_sim::*;
use serde::Deserialize;

pub struct AsyncWorldDownload {
    world: World,
}

fn unpack_data<'a, T: Deserialize<'a>>(
    entities: &'a mut Components<T>,
    id: Ent,
    data: &'a [u8],
) -> bool {
    if let Ok(e) = bincode::deserialize(data) {
        entities.spawn(id, e);
        true
    } else {
        false
    }
}

impl AsyncWorldDownload {
    pub fn new() -> Self {
        Self {
            world: World::empty(),
        }
    }

    pub fn add_entity(&mut self, ident: TableIdent, id: Ent, data: Vec<u8>) -> bool {
        match ident {
            TableIdent::Blueprints => unpack_data(&mut self.world.blueprints, id, &data),
            TableIdent::Grids => unpack_data(&mut self.world.grids, id, &data),
            TableIdent::Protos => unpack_data(&mut self.world.prototypes, id, &data),
            TableIdent::Parts => unpack_data(&mut self.world.parts, id, &data),
            TableIdent::Thrusters => unpack_data(&mut self.world.thrusters, id, &data),
            TableIdent::Computers => unpack_data(&mut self.world.computers, id, &data),
            TableIdent::Asteroids => unpack_data(&mut self.world.asteroids, id, &data),
            TableIdent::Chunks => unpack_data(&mut self.world.terrain_chunks, id, &data),
            TableIdent::Tiles => unpack_data(&mut self.world.terrain_tiles, id, &data),
            TableIdent::Inventories => unpack_data(&mut self.world.inventories, id, &data),
            TableIdent::Machines => unpack_data(&mut self.world.machines, id, &data),
            TableIdent::Pipes => unpack_data(&mut self.world.pipes, id, &data),
            TableIdent::Lights => unpack_data(&mut self.world.lights, id, &data),
            TableIdent::Excavators => unpack_data(&mut self.world.excavators, id, &data),
            TableIdent::Players => unpack_data(&mut self.world.players, id, &data),
            TableIdent::Planets => unpack_data(&mut self.world.planets, id, &data),
        }
    }

    pub fn world(self) -> World {
        self.world
    }
}
