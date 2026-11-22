use bary_core::prelude::*;
use serde::{Deserialize, Serialize};

use crate::World;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Player {
    pub username: String,
    pub iso: Isometry2d,
    pub controls: Option<PlayerControls>,
}

impl Player {
    pub fn spawn(world: &mut World, username: String, iso: Isometry2d) -> BaryResult<()> {
        let id = world.spawner.spawn();
        let player = Player {
            username,
            iso,
            controls: None,
        };
        world.players.spawn(id, player);
        Ok(())
    }

    pub fn set_controls(world: &mut World, id: Ent, controls: PlayerControls) -> BaryResult<()> {
        let player = world.players.try_get_mut(id)?;
        player.controls = Some(controls);
        Ok(())
    }

    pub fn despawn(world: &mut World, id: Ent) -> BaryResult<Player> {
        world.players.despawn(id)
    }
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
