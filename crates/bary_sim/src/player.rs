use bary_core::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Player {
    pub username: String,
    pub iso: Isometry2d,
    pub controls: Option<PlayerControls>,
}

#[derive(Debug, Default, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum JoystickPosition {
    Negative,
    #[default]
    Zero,
    Positive,
}

impl JoystickPosition {
    pub fn to_i8(&self) -> i8 {
        match self {
            Self::Negative => -1,
            Self::Zero => 0,
            Self::Positive => 1,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct PlayerControls {
    pub x: JoystickPosition,
    pub y: JoystickPosition,
}

impl PlayerControls {
    pub fn to_dir(&self) -> DVec2 {
        let x = self.x.to_i8();
        let y = self.y.to_i8();

        DVec2::new(x as f64, y as f64).normalize_or_zero()
    }
}
