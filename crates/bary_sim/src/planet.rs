use crate::World;
use bary_core::prelude::{DVec2, Ent, Isometry2d};
use bary_orbital::Body;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub struct Planet {
    pub iso: Isometry2d,
    pub body: Body,
    pub parent: Option<Ent>,
}

impl Planet {
    pub fn gravity(&self, loc: DVec2) -> DVec2 {
        self.body.gravity(loc)
    }
}

pub fn spawn_planet(world: &mut World, iso: Isometry2d, mass: f64, radius: f64) -> Ent {
    let planet = Planet {
        iso,
        body: Body::with_mass(radius, mass, 1E6),
        parent: None,
    };

    let id = world.spawner.spawn();
    world.planets.spawn(id, planet);

    id
}

#[cfg(test)]
mod tests {
    use bary_core::prelude::*;
    use bary_orbital::GRAVITATIONAL_CONSTANT;

    #[test]
    fn orbital_math() {
        const BODY_MASS: f64 = 3E16; // kg
        const SEMI_MAJOR_AXIS: f64 = 1200.0; // meters

        const MU: f64 = BODY_MASS * GRAVITATIONAL_CONSTANT;

        assert_eq!(MU, 2002289.9999999998);

        let orbital_period = 2.0 * PI_64 * (SEMI_MAJOR_AXIS.powi(3) / MU).sqrt(); // seconds

        assert_eq!(orbital_period, 184.58153284732376);

        let orbital_velocity = 2.0 * PI_64 * SEMI_MAJOR_AXIS / orbital_period;

        assert_eq!(orbital_velocity, 40.848194574546376);

        let accel = MU / SEMI_MAJOR_AXIS.powi(2);

        assert_eq!(accel, 1.3904791666666665);

        let accel_two = orbital_velocity.powi(2) / SEMI_MAJOR_AXIS;

        assert_eq!(accel_two, 1.3904791666666667);
    }
}
