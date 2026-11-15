use bary_core::prelude::{Components, Ent, TableIdent};
use bary_ipc::ServerResponse;
use bary_sim::*;
use log::warn;
use serde::Deserialize;

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

pub struct AsyncWorldDownload {
    world: World,
    expected_length: usize,
    got_entities: usize,
}

impl AsyncWorldDownload {
    pub fn new(expected_length: usize) -> Self {
        Self {
            world: World::empty(),
            expected_length,
            got_entities: 0,
        }
    }

    pub fn complete(self) -> World {
        if self.got_entities != self.expected_length {
            warn!(
                "Expected {} entities, but got {}",
                self.expected_length, self.got_entities
            );
        }
        self.world
    }

    pub fn add_entity(&mut self, ident: TableIdent, id: Ent, data: Vec<u8>) -> bool {
        let success = match ident {
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
        };

        if success {
            self.got_entities += 1;
        }

        success
    }
}
