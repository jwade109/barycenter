use bary_core::prelude::{Components, Ent, TableIdent};
use bary_ipc::DownloadFragment;
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

struct PartialDownloadState {
    name: String,
    world: World,
    n_expected_entities: usize,
    n_got_entities: usize,
}

pub struct WorldDownloader {
    state: Option<PartialDownloadState>,
}

impl WorldDownloader {
    pub fn new() -> Self {
        Self { state: None }
    }

    pub fn on_fragment(&mut self, fragment: DownloadFragment) -> Option<(World, String)> {
        match fragment {
            DownloadFragment::Start {
                name,
                n_entities,
                ticks,
            } => {
                self.state = Some(PartialDownloadState {
                    name,
                    world: World::empty_at_ticks(ticks),
                    n_expected_entities: n_entities,
                    n_got_entities: 0,
                });
                None
            }
            DownloadFragment::Entity(e) => {
                self.add_entity(e.table, e.id, e.data);
                None
            }
            DownloadFragment::End(_) => {
                let state = std::mem::take(&mut self.state)?;

                if state.n_expected_entities != state.n_got_entities {
                    warn!(
                        "Expected {} entities, got {}",
                        state.n_expected_entities, state.n_got_entities
                    );
                }

                Some((state.world, state.name))
            }
        }
    }

    fn add_entity(&mut self, table: TableIdent, id: Ent, data: Vec<u8>) -> bool {
        let Some(state) = self.state.as_mut() else {
            return false;
        };

        let success = match table {
            TableIdent::Blueprints => unpack_data(&mut state.world.blueprints, id, &data),
            TableIdent::Grids => unpack_data(&mut state.world.grids, id, &data),
            TableIdent::Protos => unpack_data(&mut state.world.prototypes, id, &data),
            TableIdent::Parts => unpack_data(&mut state.world.parts, id, &data),
            TableIdent::Thrusters => unpack_data(&mut state.world.thrusters, id, &data),
            TableIdent::Computers => unpack_data(&mut state.world.computers, id, &data),
            TableIdent::Asteroids => unpack_data(&mut state.world.asteroids, id, &data),
            TableIdent::Chunks => unpack_data(&mut state.world.terrain_chunks, id, &data),
            TableIdent::Tiles => unpack_data(&mut state.world.terrain_tiles, id, &data),
            TableIdent::Inventories => unpack_data(&mut state.world.inventories, id, &data),
            TableIdent::Machines => unpack_data(&mut state.world.machines, id, &data),
            TableIdent::Pipes => unpack_data(&mut state.world.pipes, id, &data),
            TableIdent::Lights => unpack_data(&mut state.world.lights, id, &data),
            TableIdent::Excavators => unpack_data(&mut state.world.excavators, id, &data),
            TableIdent::Players => unpack_data(&mut state.world.players, id, &data),
            TableIdent::Planets => unpack_data(&mut state.world.planets, id, &data),
        };


        if success {
            state.n_got_entities += 1;
        }

        success
    }
}
