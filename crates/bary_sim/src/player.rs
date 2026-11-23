use bary_core::prelude::*;
use early_returns::some_or_return;
use serde::{Deserialize, Serialize};

use crate::{NOMINAL_DT, Planet, World};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Player {
    pub is_connected: bool,
    pub username: String,
    pub pos: Isometry2d,
    pub vel: Isometry2d,
    pub controls: Option<PlayerControls>,
    pub orbital_parent: Option<Ent>,
}

impl Player {
    pub fn spawn(world: &mut World, username: String, pos: Isometry2d, vel: Isometry2d) -> Ent {
        let id = world.spawner.spawn();
        let player = Player {
            is_connected: true,
            username,
            pos,
            vel,
            controls: None,
            orbital_parent: None,
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

    pub fn set_orbital_parent(
        world: &mut World,
        player_id: Ent,
        orbital_parent: Option<Ent>,
    ) -> BaryResult<()> {
        let player = world.players.try_get_mut(player_id)?;
        player.orbital_parent = orbital_parent;
        Ok(())
    }

    fn update_orbital_dynamics(&mut self, planets: &Components<Planet>) {
        let planet_id = some_or_return!(self.orbital_parent);
        let planet = some_or_return!(planets.get(planet_id));
        let v = self.pos.translation - planet.pos.translation;
        let accel = planet.gravity(v);
        self.vel.translation += accel * NOMINAL_DT;
    }

    fn update(&mut self, planets: &Components<Planet>) {
        self.update_orbital_dynamics(planets);

        let ctrl = self.controls.unwrap_or_default();
        let body_fixed_dir = ctrl.to_dir();
        let world_dir = rotate_f64(body_fixed_dir, self.pos.rotation);
        let acc = Isometry2d::new(world_dir * 20.0, ctrl.a.to_i8() as f64 * 6.0);

        self.vel += acc * NOMINAL_DT;
        self.pos += self.vel * NOMINAL_DT;

        self.pos.rotation = wrap_pi_npi_f64(self.pos.rotation);

        self.vel.rotation *= 0.97;
    }

    /// Updates all players.
    pub(crate) fn update_all(world: &mut World) {
        for player in world.players.values_mut() {
            player.update(&world.planets);
        }
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
