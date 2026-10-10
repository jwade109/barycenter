use crate::TICKS_PER_SECOND;
use bary_core::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Particle {
    Ping(PingParticle),
    Dust(DustParticle),
}

impl Particle {
    pub fn is_alive(&self, current_tick: u64) -> bool {
        match self {
            Self::Ping(ping) => ping.is_alive(current_tick),
            Self::Dust(dust) => dust.is_alive(current_tick),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PingParticle {
    pos: DVec2,
    start_tick: u64,
    duration: u64,
}

impl PingParticle {
    pub fn new(pos: DVec2, start_tick: u64) -> Self {
        Self {
            pos,
            start_tick,
            duration: TICKS_PER_SECOND * 10,
        }
    }

    pub fn start_tick(&self) -> u64 {
        self.start_tick
    }

    pub fn pos(&self) -> DVec2 {
        self.pos
    }

    pub fn radius(&self) -> f32 {
        5.0
    }

    pub fn is_alive(&self, current_tick: u64) -> bool {
        current_tick < self.start_tick + self.duration
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DustParticle {
    start_tick: u64,
    pv: PV,
}

impl DustParticle {
    const DURATION_TICKS: u64 = 700;

    pub fn new(pos: Vec2, start_tick: u64) -> Self {
        let vel = randvec(0.02, 0.07);
        Self {
            pv: PV::from_f64(pos, vel),
            start_tick,
        }
    }

    pub fn is_alive(&self, current_tick: u64) -> bool {
        current_tick < self.start_tick + Self::DURATION_TICKS
    }

    pub fn pos(&self, tick: u64) -> Vec2 {
        if tick < self.start_tick {
            self.pv.pos.as_vec2()
        } else {
            let delta = tick - self.start_tick;
            let t = delta as f64 / TICKS_PER_SECOND as f64;
            (self.pv.pos + self.pv.vel * t).as_vec2()
        }
    }

    pub fn alpha(&self, current_tick: u64) -> f32 {
        1.0 - (current_tick - self.start_tick) as f32 / Self::DURATION_TICKS as f32
    }
}
