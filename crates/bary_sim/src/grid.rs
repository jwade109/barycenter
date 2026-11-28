use bary_core::prelude::*;
use bary_parts::*;
use early_returns::some_or_continue;
use log::warn;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::{NOMINAL_DT, RadioTransmission, World};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VehicleGrid {
    pub name: String,
    pub blueprint: Option<BlueprintId>,
    pub parts_mass: Mass,
    pub center_of_mass: DVec2,
    pub particle_location: Isometry2d,
    pub velocity: Isometry2d,
    pub body_frame_forces: Isometry2d,
    pub parts: BTreeSet<Ent>,
    pub thrusters: BTreeSet<Ent>,
    pub computers: BTreeSet<Ent>,
    pub lights: BTreeSet<Ent>,
    pub pipes: BTreeSet<Ent>,
    /// Lower bound is inclusive; upper is exclusive.
    /// Extent is upper minus lower. An empty grid
    /// will have zero extent.
    pub vehicle_bounds: (IVec2, IVec2),

    // TODO this is not tested in any way.
    pub occupancy: BTreeMap<(i32, i32), PartOccupancy>,
    pub orbital_parent: Option<Ent>,

    pub transmission_inbox: VecDeque<RadioTransmission>,
    pub controlled_by: Option<String>,
}

impl VehicleGrid {
    pub fn empty() -> Self {
        Self::with_name("", None)
    }

    pub fn can_insert_part(&self, pl: GridRegion, layer: PartLayer) -> bool {
        for cell in pl.cells() {
            if let Some(occ) = self.get_parts_at(cell)
                && occ.at_layer(layer).is_some() {
                    return false;
                }
        }
        true
    }

    pub fn origin(&self) -> Isometry2d {
        self.particle_location.offset(-self.center_of_mass)
    }

    pub fn receive_transmission(&mut self, rt: RadioTransmission) {
        self.transmission_inbox.push_back(rt);
        if self.transmission_inbox.len() > 20 {
            self.transmission_inbox.pop_front();
        }
    }

    pub fn with_name(name: impl Into<String>, bp_id: Option<BlueprintId>) -> Self {
        VehicleGrid {
            name: name.into(),
            blueprint: bp_id,
            parts_mass: Mass::ZERO,
            center_of_mass: DVec2::ZERO,
            particle_location: Isometry2d::ZERO,
            velocity: Isometry2d::ZERO,
            body_frame_forces: Isometry2d::ZERO,
            parts: BTreeSet::new(),
            thrusters: BTreeSet::new(),
            computers: BTreeSet::new(),
            lights: BTreeSet::new(),
            pipes: BTreeSet::new(),
            vehicle_bounds: (IVec2::ZERO, IVec2::ZERO),
            occupancy: BTreeMap::new(),
            orbital_parent: None,
            transmission_inbox: VecDeque::new(),
            controlled_by: None,
        }
    }

    pub fn parts_mass(&self) -> Mass {
        self.parts_mass
    }

    pub fn centroid(&self) -> DVec2 {
        let lower = PartCoord::new(self.vehicle_bounds.0).to_meters().as_dvec2();
        let upper = PartCoord::new(self.vehicle_bounds.1).to_meters().as_dvec2();
        (upper + lower) / 2.0
    }

    pub fn centroid_isometry(&self) -> Isometry2d {
        let c = self.centroid();
        let origin = self.origin();
        origin.offset(c)
    }

    pub fn linear_acceleration(&self) -> DVec2 {
        self.body_frame_forces.translation / self.parts_mass.to_kg_f64()
    }

    pub fn angular_acceleration(&self) -> f64 {
        let moment_of_inertia = self.parts_mass.to_kg_f64() * 100.0;
        self.body_frame_forces.rotation / moment_of_inertia
    }

    pub fn bounding_radius(&self) -> f32 {
        let half_dims = self.dims().to_meters() / 2.0;
        half_dims.length() * 1.03
    }

    pub fn dims(&self) -> PartCoord {
        PartCoord::new(self.vehicle_bounds.1 - self.vehicle_bounds.0)
    }

    /// TODO test this.
    pub fn get_parts_at(&self, p: PartCoord) -> Option<&PartOccupancy> {
        let p = (p.0.x, p.0.y);
        self.occupancy.get(&p)
    }

    pub fn update_bounds(&mut self) {
        let mut bounds: Option<(IVec2, IVec2)> = None;
        for cell in self.occupancy.keys() {
            let lower = IVec2::new(cell.0, cell.1);
            let upper = lower + IVec2::ONE;
            if let Some(c) = &mut bounds {
                c.0.x = c.0.x.min(lower.x);
                c.0.y = c.0.y.min(lower.y);
                c.1.x = c.1.x.max(upper.x);
                c.1.y = c.1.y.max(upper.y);
            } else {
                bounds = Some((lower, upper));
            }
        }
        self.vehicle_bounds = bounds.unwrap_or((IVec2::ZERO, IVec2::ZERO));
    }

    pub fn mark_occupied(&mut self, region: GridRegion, layer: PartLayer, id: Ent) {
        for cell in region.cells() {
            let key = (cell.0.x, cell.0.y);
            self.occupancy
                .entry(key)
                .and_modify(|e| {
                    e.mark(layer, id);
                })
                .or_insert({
                    let mut occ = PartOccupancy::default();
                    occ.mark(layer, id);
                    occ
                });
        }
        self.update_bounds();
    }

    pub fn remove_from_index(&mut self, id: Ent) {
        for occ in self.occupancy.values_mut() {
            occ.remove(id);
        }

        self.occupancy.retain(|_, e| e.has_any());

        self.parts.retain(|e| *e != id);
        self.thrusters.retain(|e| *e != id);
        self.computers.retain(|e| *e != id);
        self.lights.retain(|e| *e != id);
        self.update_bounds();
    }

    pub fn calculate_islands(&self) -> Vec<BTreeSet<Ent>> {
        let mut unvisited = BTreeSet::new();

        for key in self.occupancy.keys() {
            unvisited.insert(*key);
        }

        let get_neighbors = |(x, y): (i32, i32)| [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)];

        let mut ret = Vec::new();

        while let Some(seed) = unvisited.first().cloned() {
            let mut parts = BTreeSet::new();

            let mut open_set: BTreeSet<(i32, i32)> = [seed].into();
            let mut closed_set = BTreeSet::new();
            while let Some(current) = open_set.pop_first() {
                if closed_set.contains(&current) {
                    continue;
                }
                closed_set.insert(current);
                if !unvisited.contains(&current) {
                    continue;
                };
                unvisited.remove(&current);

                if let Some(occ) = self.occupancy.get(&current) {
                    for (_, id) in occ.iter() {
                        parts.insert(id);
                    }
                }

                for n in get_neighbors(current) {
                    open_set.insert(n);
                }
            }

            ret.push(parts);
        }

        ret.sort_by_key(|e| -(e.len() as i32));

        ret
    }

    pub(crate) fn propagate_gravity(world: &mut World) {
        for grid in world.grids.values_mut() {
            let planet_id = some_or_continue!(grid.orbital_parent);
            let planet = some_or_continue!(world.planets.get(planet_id));
            let v = grid.particle_location.translation - planet.pos.translation;
            let accel = planet.gravity(v);
            grid.velocity.translation += accel * NOMINAL_DT;
        }
    }

    pub(crate) fn handle_collisions(world: &mut World) {
        let mut collisions = Vec::new();
        // TODO(optimization) need a spatial data structure
        // for grids!
        for (a_id, first) in world.grids.iter() {
            for (b_id, second) in world.grids.iter() {
                if b_id <= a_id {
                    continue;
                }

                let collision_distance =
                    first.bounding_radius() as f64 + second.bounding_radius() as f64;

                let p = first.centroid_isometry().tr();
                let q = second.centroid_isometry().tr();
                let r = q - p;

                let u = first.velocity.tr();
                let v = second.velocity.tr();
                let w = v - u;

                let moving_towards = w.dot(r) < 0.0;

                let close_enough = r.length_squared() < collision_distance.powi(2);

                if close_enough && moving_towards {
                    let info = CollisionInfo {
                        primary: *a_id,
                        secondary: *b_id,
                        primary_mass: first.parts_mass(),
                        secondary_mass: second.parts_mass(),
                        primary_pos: p,
                        secondary_pos: q,
                        delta_pos: r,
                        primary_vel: u,
                        secondary_vel: v,
                        delta_vel: w,
                    };

                    collisions.push(info);
                }
            }
        }

        for col in collisions {
            let grid = world.grids.try_get_mut(col.primary).unwrap();
            grid.velocity.translation += col.delta_vel;
            let grid = world.grids.try_get_mut(col.secondary).unwrap();
            grid.velocity.translation -= col.delta_vel;
            warn!("Collision: {col:?}");
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CollisionInfo {
    pub primary: Ent,
    pub secondary: Ent,

    pub primary_mass: Mass,
    pub secondary_mass: Mass,

    pub primary_pos: DVec2,
    pub secondary_pos: DVec2,
    pub delta_pos: DVec2,

    pub primary_vel: DVec2,
    pub secondary_vel: DVec2,
    pub delta_vel: DVec2,
}
