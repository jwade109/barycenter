use crate::World;
use bary_core::prelude::{DVec2, Ent, Isometry2d};
use bary_orbital::Body;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Planet {
    pub name: String,
    pub pos: Isometry2d,
    pub vel: Isometry2d,
    pub body: Body,
    pub parent: Option<Ent>,
}

impl Planet {
    pub fn gravity(&self, loc: DVec2) -> DVec2 {
        self.body.gravity(loc)
    }
}

pub fn spawn_planet(
    world: &mut World,
    name: String,
    pos: Isometry2d,
    vel: Isometry2d,
    mass: f64,
    radius: f64,
    parent: Option<Ent>,
) -> Ent {
    let planet = Planet {
        name,
        pos,
        vel,
        body: Body::with_mass(radius, mass, 1E6),
        parent,
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
        const BODY_MASS: f64 = 3E15; // kg
        const SEMI_MAJOR_AXIS: f64 = 1200.0; // meters

        const MU: f64 = BODY_MASS * GRAVITATIONAL_CONSTANT;

        assert_eq!(MU, 200228.99999999997);

        let orbital_period = 2.0 * PI_64 * (SEMI_MAJOR_AXIS.powi(3) / MU).sqrt(); // seconds

        assert_eq!(orbital_period, 583.6980578027278);

        let orbital_velocity = 2.0 * PI_64 * SEMI_MAJOR_AXIS / orbital_period;

        let accel = MU / SEMI_MAJOR_AXIS.powi(2);

        let accel_two = orbital_velocity.powi(2) / SEMI_MAJOR_AXIS;

        assert_eq!(
            (orbital_velocity, accel, accel_two),
            (12.91733331612992, 0.13904791666666666, 0.13904791666666666)
        );
    }
}
