use bary_core::prelude::{Ent, Isometry2d};
use bary_orbital::Body;
use serde::{Deserialize, Serialize};

use crate::World;

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub struct Planet {
    pub iso: Isometry2d,
    pub body: Body,
    pub parent: Option<Ent>,
}

pub fn spawn_planet(world: &mut World, iso: Isometry2d, radius: f64) -> Ent {
    let planet = Planet {
        iso,
        body: Body::with_mass(radius, 1E7, 1E6),
        parent: None,
    };

    let id = world.spawner.spawn();
    world.planets.spawn(id, planet);

    id
}
