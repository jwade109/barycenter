use bary_core::prelude::*;
use bary_parts::ThrusterPrototype;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Thruster {
    is_on: bool,
    is_rcs: bool,
    thrust: f32,
    /// the entity that last modified this thruster's state
    last_controlled_by: Option<Ent>,
}

impl Thruster {
    pub fn from_proto(proto: &ThrusterPrototype) -> Self {
        Thruster {
            is_on: false,
            is_rcs: proto.is_rcs,
            // TODO(gross)
            thrust: proto.thrust as f32,
            last_controlled_by: None,
        }
    }

    pub fn rcs(thrust: f32) -> Self {
        Self {
            is_on: false,
            is_rcs: true,
            thrust,
            last_controlled_by: None,
        }
    }

    pub fn linear(thrust: f32) -> Self {
        Self {
            is_on: false,
            is_rcs: false,
            thrust,
            last_controlled_by: None,
        }
    }

    pub fn is_rcs(&self) -> bool {
        self.is_rcs
    }

    pub fn is_on(&self) -> bool {
        self.is_on
    }

    pub fn thrust(&self) -> f32 {
        self.thrust
    }

    pub fn set_state(&mut self, is_on: bool, identity: Option<Ent>) -> bool {
        let changed = self.is_on == is_on;
        self.is_on = is_on;
        self.last_controlled_by = identity;
        changed
    }
}
