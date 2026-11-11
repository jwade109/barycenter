use bary_core::prelude::*;
use bary_parts::ThrusterPrototype;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Thruster {
    throttle: f32,
    is_rcs: bool,
    thrust: f32,
    /// the entity that last modified this thruster's state
    last_controlled_by: Option<Ent>,
}

impl Thruster {
    pub fn from_proto(proto: &ThrusterPrototype) -> Self {
        Thruster {
            throttle: 0.0,
            is_rcs: proto.is_rcs,
            // TODO(gross)
            thrust: proto.thrust as f32,
            last_controlled_by: None,
        }
    }

    pub fn rcs(thrust: f32) -> Self {
        Self {
            throttle: 0.0,
            is_rcs: true,
            thrust,
            last_controlled_by: None,
        }
    }

    pub fn linear(thrust: f32) -> Self {
        Self {
            throttle: 0.0,
            is_rcs: false,
            thrust,
            last_controlled_by: None,
        }
    }

    pub fn is_rcs(&self) -> bool {
        self.is_rcs
    }

    pub fn is_on(&self) -> bool {
        self.throttle > 0.0
    }

    pub fn throttle(&self) -> f32 {
        self.throttle
    }

    pub fn current_thrust(&self) -> f32 {
        self.thrust * self.throttle.clamp(0.0, 1.0)
    }

    pub fn max_thrust(&self) -> f32 {
        self.thrust
    }

    pub fn set_state(&mut self, throttle: f32, identity: Option<Ent>) -> bool {
        let changed = self.throttle != throttle;
        self.throttle = throttle;
        self.last_controlled_by = identity;
        changed
    }
}
