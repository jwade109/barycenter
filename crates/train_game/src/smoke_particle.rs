use bary_core::prelude::*;
use glam::DVec2;
use rend::Color;

const WIND: DVec2 = DVec2::new(-1.4, 0.7);

pub struct SmokeParticle {
    pub pos: DVec2,
    pub vel: DVec2,
    pub age: f64,
    pub radius: f64,
    pub value: f64,
}

impl SmokeParticle {
    pub fn new(pos: DVec2, vel: DVec2) -> Self {
        Self {
            pos,
            vel: vel + randvec(0.6, 1.0).as_dvec2() * 4.0,
            age: 0.0,
            radius: rand(5.0, 7.0) as f64,
            value: rand(0.2, 0.7) as f64,
        }
    }

    pub fn step(&mut self, dt: f64) {
        self.age += dt;
        self.pos += self.vel * dt;
        let u = (WIND - self.vel).normalize_or_zero();
        let mag = (WIND - self.vel).length_squared();
        let dv = 0.2 * u * mag + randvec(0.01, 0.03).as_dvec2();
        self.vel += dv * dt;
    }

    pub fn radius(&self) -> f64 {
        self.radius * self.age.sqrt() + 1.5
    }

    pub fn opacity(&self) -> f64 {
        (0.7 - self.age / 2.0)
            .max(0.1 - self.age.sqrt() / 40.0)
            .clamp(0.0, 1.0)
    }

    pub fn color(&self) -> Color {
        Color::gray(self.value, self.opacity())
    }
}
