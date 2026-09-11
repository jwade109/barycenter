use crate::{node::*, track::*, world::World};
use bary_core::prelude::{Ent, Isometry2d, chance, rand, randint};
use std::collections::BTreeMap;

pub struct RailCar {
    pub segment: Ent,
    pub pos: f64,
    pub origin: Terminus,
    pub vel: f64,
    pub forward_connection: Option<Ent>,
    pub backward_connection: Option<Ent>,
    pub consist: Ent,
}

impl RailCar {
    pub const WIDTH_METERS: f64 = 4.0;
    pub const LENGTH_METERS: f64 = 14.0;

    pub fn new(segment: Ent, pos: f64, vel: f64, origin: Terminus, consist: Ent) -> Self {
        Self {
            segment,
            pos,
            vel,
            origin,
            forward_connection: None,
            backward_connection: None,
            consist,
        }
    }

    pub fn step(&mut self, dt: f64) {
        self.pos += self.vel * dt;
    }

    pub fn is_front(&self) -> bool {
        self.forward_connection.is_none()
    }
}

pub struct RailConsist {
    pub cars: Vec<Ent>,
}

impl RailConsist {
    pub fn new(cars: impl Iterator<Item = Ent>) -> Self {
        Self {
            cars: cars.collect(),
        }
    }
}

pub fn get_car_isometry(world: &World, car_id: Ent) -> Option<Isometry2d> {
    let car = world.cars.get(car_id)?;
    let track = world.segments.get(car.segment)?;
    let iso = track.eval_at(car.origin, car.pos);
    Some(iso)
}

pub fn spawn_new_consist(world: &mut World, loc: TrackLocation, n_cars: usize) -> Option<Ent> {
    let track = world.segments.get(loc.track_id)?;

    let vel = 800.0;
    let d = RailCar::LENGTH_METERS + 3.0;

    let consist_id = world.spawner.spawn();

    let mut cars = Vec::new();

    let ids: Vec<Ent> = (0..n_cars).map(|_| world.spawner.spawn()).collect();

    for i in 0..n_cars {
        let in_front = (i > 0).then_some(ids.get(i - 1)).flatten().cloned();
        let this_id = ids[i];
        let behind = ids.get(i + 1).cloned();

        let pos = loc.pos - d * i as f64;
        if pos > track.length || pos < 0.0 {
            break;
        }

        let mut car = RailCar::new(loc.track_id, pos, vel, loc.origin, consist_id);
        car.forward_connection = in_front;
        car.backward_connection = behind;
        cars.push(this_id);
        world.cars.spawn(this_id, car);
    }

    let consist = RailConsist::new(cars.into_iter());
    world.consists.spawn(consist_id, consist);

    Some(consist_id)
}

pub fn update_track_parentage(world: &mut World, car_id: Ent) -> Option<()> {
    let car = world.cars.get(car_id)?;
    let track = world.segments.get(car.segment)?;

    let (node_id, overshoot) = if car.pos > track.length {
        let id = track.get_node_at(car.origin.other());
        let overshoot = car.pos - track.length;
        (id, overshoot)
    } else if car.pos < 0.0 {
        let id = track.get_node_at(car.origin);
        let overshoot = -car.pos;
        (id, overshoot)
    } else {
        return Some(());
    };

    let source_track = car.segment;

    if car.is_front() {
        randomize_switch_state(world, node_id);
    }

    let next = get_next_track(world, node_id, source_track);

    let car = world.cars.try_get_mut(car_id).ok()?;

    if let Some(next) = next {
        car.segment = next.0;
        car.pos = overshoot;
        car.origin = next.1;
    } else {
        car.pos = overshoot;
        car.origin = car.origin.other();
    }

    Some(())
}
