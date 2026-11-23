use bary_core::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, VecDeque};

use crate::World;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Tracker {
    origin: VecDeque<DVec2>,
    center_of_mass: VecDeque<DVec2>,
    centroid: VecDeque<DVec2>,
}

impl Tracker {
    pub const MAX_LEN: usize = 500;

    pub fn series(&self) -> [&VecDeque<DVec2>; 3] {
        [&self.origin, &self.center_of_mass, &self.centroid]
    }

    pub fn center_of_mass(&self) -> &VecDeque<DVec2> {
        &self.center_of_mass
    }

    fn enqueue(hist: &mut VecDeque<DVec2>, pose: Isometry2d) {
        hist.push_back(pose.translation);
        if hist.len() > Self::MAX_LEN {
            hist.pop_front();
        }
    }

    pub fn add(&mut self, o: Isometry2d, com: Isometry2d, c: Isometry2d) {
        Self::enqueue(&mut self.origin, o);
        Self::enqueue(&mut self.center_of_mass, com);
        Self::enqueue(&mut self.centroid, c);
    }

    /// Appends grid locations to tracker entities.
    pub(crate) fn update_all(world: &mut World) {
        if world.ticks % 20 > 0 {
            return;
        }

        let mut to_despawn = BTreeSet::new();

        for (grid_id, grid) in world.grids.iter() {
            let is_controllable = !grid.computers.is_empty();
            if world.tracking.try_get(*grid_id).is_err() && is_controllable {
                let tracker = Tracker::default();
                world.tracking.spawn(*grid_id, tracker);
            }
        }

        for (player_id, _) in world.players.iter() {
            if world.tracking.try_get(*player_id).is_err() {
                let tracker = Tracker::default();
                world.tracking.spawn(*player_id, tracker);
            }
        }

        for (id, tracker) in world.tracking.iter_mut() {
            if let Ok(grid) = world.grids.try_get(*id) {
                tracker.add(
                    grid.origin(),
                    grid.particle_location,
                    grid.centroid_isometry(),
                );
            } else if let Ok(player) = world.players.try_get(*id) {
                tracker.add(player.pos, player.pos, player.pos);
            } else {
                to_despawn.insert(*id);
            }
        }

        for id in to_despawn {
            _ = world.tracking.despawn(id);
        }
    }
}
