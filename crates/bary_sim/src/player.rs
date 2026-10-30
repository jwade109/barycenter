use bary_core::prelude::*;
use bary_orbital::VehicleControl;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum PlayerState {
    Flying(Isometry2d),
    PilotingGrid(Ent, VehicleControl),
}

impl std::fmt::Display for PlayerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Flying(_) => write!(f, "Flying around"),
            Self::PilotingGrid(id, _) => write!(f, "Piloting {}", id),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Player {
    pub name: String,
    pub cursor_world_position: Option<Vec2>,
    pub state: PlayerState,
}

impl Player {
    pub fn driving_grid(&self) -> Option<Ent> {
        match self.state {
            PlayerState::PilotingGrid(id, _) => Some(id),
            _ => None,
        }
    }

    pub fn set_position(&mut self, iso: Isometry2d) {
        self.state = PlayerState::Flying(iso);
    }

    pub fn is_driving(&self) -> bool {
        matches!(self.state, PlayerState::PilotingGrid(_, _))
    }

    pub fn is_flying(&self) -> bool {
        matches!(self.state, PlayerState::Flying(_))
    }
}
