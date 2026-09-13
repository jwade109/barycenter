use bary_core::prelude::randvec;
use glam::DVec2;

const WIND: DVec2 = DVec2::new(-0.3, 0.2);

pub struct SmokeParticle {
    pub pos: DVec2,
    pub vel: DVec2,
    pub age: f64,
}

impl SmokeParticle {
    pub fn new(pos: DVec2) -> Self {
        Self {
            pos,
            vel: randvec(0.1, 1.0).as_dvec2() * 0.05,
            age: 0.0,
        }
    }

    pub fn step(&mut self, dt: f64) {
        self.age += dt;
        self.pos += self.vel;
        let dv = (WIND - self.vel) * 0.5;
        self.vel += dv * dt;
    }
}
