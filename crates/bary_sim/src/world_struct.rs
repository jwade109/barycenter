use crate::*;
use bary_core::prelude::*;
use bary_factory::*;
use bary_parts::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct World {
    pub ticks: u64,

    // debug info
    pub grid_acceleration_updates: u64,

    // components - to be synchronized between clients
    pub spawner: EntitySpawner,
    pub particles: Vec<Particle>,
    pub players: Components<Player>,
    pub blueprints: Components<NamedBlueprint>,
    pub prototypes: Components<PartPrototype>,
    pub parts: Components<Part>,
    pub thrusters: Components<Thruster>,
    pub computers: Components<Computer>,
    pub lights: Components<Light>,
    pub grids: Components<VehicleGrid>,
    pub tracking: Components<Tracker>,
    pub inventories: Components<Inventory>,
    pub machines: Components<Machine>,
    pub excavators: Components<Excavator>,
    pub stars: Components<Star>,
    pub planets: Components<Planet>,
    pub pipes: Components<Pipe>,
    pub debug_portals: Components<DebugPortal>,
    pub asteroids: Components<BigRock>,
    pub construction: Components<Construction>,

    pub terrain_chunks: Components<TerrainChunk>,
    pub terrain_tiles: Components<TerrainTile>,

    // TODO might move this to assets.
    pub ship_names: Vec<String>,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "World({}, {} grids, {})",
            self.ticks,
            self.grids.len(),
            self.spawner.next()
        )
    }
}

impl World {
    pub fn empty() -> Self {
        Self {
            ship_names: vec![
                "Gary".to_string(),
                "Sally".to_string(),
                "Juliet".to_string(),
                "Violet".to_string(),
                "Charlie".to_string(),
                "Orville".to_string(),
            ],
            ..Default::default()
        }
    }
}

pub fn size_in_bytes(world: &World) -> usize {
    let bytes = bincode::serialize(world).unwrap();
    bytes.len()
}
