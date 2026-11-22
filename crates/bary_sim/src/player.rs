use bary_core::prelude::*;
use serde::{Deserialize, Serialize};

use crate::World;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Player {
    pub is_connected: bool,
    pub username: String,
    pub iso: Isometry2d,
    pub controls: Option<PlayerControls>,
}

impl Player {
    pub fn spawn(world: &mut World, username: String, iso: Isometry2d) -> Ent {
        let id = world.spawner.spawn();
        let player = Player {
            is_connected: true,
            username,
            iso,
            controls: None,
        };
        world.players.spawn(id, player);
        id
    }

    /// Sets the given player's persistent control inputs.
    /// Control inputs define how a player changes state as time
    /// advances, rather than in instantaneous events.
    /// If the player is not connected, will also mark as connected.
    pub fn set_controls(world: &mut World, id: Ent, controls: PlayerControls) -> BaryResult<()> {
        let player = world.players.try_get_mut(id)?;
        player.is_connected = true;
        player.controls = Some(controls);
        Ok(())
    }

    /// Removes a player from the world.
    pub fn despawn(world: &mut World, id: Ent) -> BaryResult<Player> {
        world.players.despawn(id)
    }

    /// Marks a player as "connected". Connected players have full agency
    /// within the simulated world.
    pub fn connect(world: &mut World, id: Ent) -> BaryResult<()> {
        let player = world.players.try_get_mut(id)?;
        player.is_connected = true;
        player.controls = None;
        Ok(())
    }

    /// Marks a player as "disconnected". Disconnected players have a reduced
    /// role in the world (and may be entirely incorporeal),
    /// but are still considered extant entities.
    pub fn disconnect(world: &mut World, id: Ent) -> BaryResult<()> {
        let player = world.players.try_get_mut(id)?;
        player.is_connected = false;
        player.controls = None;
        Ok(())
    }

    /// Finds an arbitrary player with the given username, if one exists.
    pub fn find_by_name(world: &World, username: &str) -> Option<Ent> {
        world
            .players
            .iter()
            .find_map(|(id, player)| (player.username == username).then_some(*id))
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
    pub a: JoystickPosition,
}

impl PlayerControls {
    pub fn to_dir(&self) -> DVec2 {
        let x = self.x.to_i8();
        let y = self.y.to_i8();

        DVec2::new(x as f64, y as f64).normalize_or_zero()
    }
}
