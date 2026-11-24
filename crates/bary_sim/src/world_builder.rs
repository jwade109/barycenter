use crate::*;
use bary_core::prelude::*;
use bary_factory::*;
use bary_parts::*;
use log::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Deserialize, Serialize)]
enum WorldBuilderCommand {
    LoadBlueprint(BlueprintId),
    SpawnShip(BlueprintId, String, Isometry2d),
    SpawnPart(String),
    SetVelocity(Isometry2d),
    ModifyWorld(WorldDelta),
    InsertDebugSource(PartCoord, Item),
    InsertPipe(PartCoord, PartCoord),
    SetRecipe(PartCoord, RecipeListing),
    SpawnAsteroid(Isometry2d, f32, u32),
    SpawnPlanet {
        name: String,
        pos: Isometry2d,
        vel: Isometry2d,
        radius: f64,
        mass: f64,
    },
    SpawnPlayer {
        username: String,
        pos: Isometry2d,
        vel: Isometry2d,
    },
    LoadInstruction(TimedInstruction),
    SetAttitudePD(f64, f64),
}

pub struct WorldBuilder {
    assets_dir: Option<String>,
    commands: Vec<WorldBuilderCommand>,
    cursor_grid: Option<Ent>,
    cursor_player: Option<Ent>,
    cursor_planet: Option<Ent>,
}

impl WorldBuilder {
    pub fn new() -> Self {
        Self {
            assets_dir: None,
            commands: Vec::new(),
            cursor_grid: None,
            cursor_player: None,
            cursor_planet: None,
        }
    }

    pub fn parse_file(mut self, path: impl AsRef<Path>) -> BaryResult<Self> {
        let s = std::fs::read_to_string(path)?;
        let commands: Vec<WorldBuilderCommand> = serde_yaml::from_str(&s)?;
        self.commands.extend(commands);
        Ok(self)
    }

    pub fn assets(mut self) -> Self {
        self.assets_dir = Some("./assets/".to_string());
        self
    }

    pub fn test_assets(mut self) -> Self {
        self.assets_dir = Some("../../assets/".to_string());
        self
    }

    pub fn blueprint(mut self, id: impl Into<BlueprintId>) -> Self {
        let cmd = WorldBuilderCommand::LoadBlueprint(id.into());
        self.commands.push(cmd);
        self
    }

    pub fn spawn(
        mut self,
        bp_id: impl Into<BlueprintId>,
        name: impl Into<String>,
        iso: impl Into<Isometry2d>,
    ) -> Self {
        let name = name.into();
        let cmd = WorldBuilderCommand::SpawnShip(bp_id.into(), name, iso.into());
        self.commands.push(cmd);
        self
    }

    pub fn set_velocity(mut self, vel: impl Into<Isometry2d>) -> Self {
        let cmd = WorldBuilderCommand::SetVelocity(vel.into());
        self.commands.push(cmd);
        self
    }

    pub fn insert_source(mut self, coord: impl Into<PartCoord>, item: Item) -> Self {
        let cmd = WorldBuilderCommand::InsertDebugSource(coord.into(), item);
        self.commands.push(cmd);
        self
    }

    pub fn set_recipe(mut self, coord: impl Into<PartCoord>, recipe: RecipeListing) -> Self {
        let cmd = WorldBuilderCommand::SetRecipe(coord.into(), recipe);
        self.commands.push(cmd);
        self
    }

    pub fn insert_pipe(mut self, a: impl Into<PartCoord>, b: impl Into<PartCoord>) -> Self {
        let cmd = WorldBuilderCommand::InsertPipe(a.into(), b.into());
        self.commands.push(cmd);
        self
    }

    pub fn waypoint(mut self, grid_name: &str, waypoint: impl Into<Isometry2d>) -> Self {
        let cmd = WorldDelta::SetWaypointByName {
            name: grid_name.to_string(),
            waypoint: waypoint.into(),
        };
        let cmd = WorldBuilderCommand::ModifyWorld(cmd);
        self.commands.push(cmd);
        self
    }

    pub fn command(mut self, action: WorldDelta) -> Self {
        let cmd = WorldBuilderCommand::ModifyWorld(action);
        self.commands.push(cmd);
        self
    }

    pub fn planet(
        mut self,
        name: impl Into<String>,
        p: impl Into<Isometry2d>,
        v: impl Into<Isometry2d>,
        mass: f64,
        r: f64,
    ) -> Self {
        let cmd = WorldBuilderCommand::SpawnPlanet {
            name: name.into(),
            pos: p.into(),
            vel: v.into(),
            radius: r,
            mass,
        };
        self.commands.push(cmd);
        self
    }

    pub fn asteroid(mut self, p: impl Into<Isometry2d>, r: f32, seed: u32) -> Self {
        let cmd = WorldBuilderCommand::SpawnAsteroid(p.into(), r, seed);
        self.commands.push(cmd);
        self
    }

    pub fn player(
        mut self,
        name: &str,
        pos: impl Into<Isometry2d>,
        vel: impl Into<Isometry2d>,
    ) -> Self {
        let cmd = WorldBuilderCommand::SpawnPlayer {
            username: name.to_string(),
            pos: pos.into(),
            vel: vel.into(),
        };
        self.commands.push(cmd);
        self
    }

    pub fn build(mut self) -> World {
        let mut world = World::empty();

        let parts = if let Some(assets_dir) = &self.assets_dir {
            let parts_dir = PathBuf::from(&assets_dir).join("parts");

            let ship_names_path = PathBuf::from(&assets_dir).join("ship_names.txt");
            world.ship_names = load_names_from_file(ship_names_path).unwrap_or(vec![]);

            let parts = load_parts_from_dir(&parts_dir).expect("Parts dir");

            for (_, part) in &parts {
                let id = world.spawner.spawn();
                world.prototypes.spawn(id, part.clone());
            }

            parts
        } else {
            BTreeMap::new()
        };

        let commands = self.commands.clone();

        for cmd in &commands {
            println!("{:?}", cmd);
        }

        for cmd in commands {
            match cmd {
                WorldBuilderCommand::LoadBlueprint(bpid) => {
                    if let Some(assets_dir) = self.assets_dir.as_ref() {
                        let id = world.spawner.spawn();
                        if let Ok(bp) = load_blueprint(&bpid, &assets_dir, &parts) {
                            let bp = NamedBlueprint {
                                id: bpid,
                                blueprint: bp,
                            };
                            info!(
                                "Loaded blueprint: {} v{} ({} parts)",
                                bp.id.0,
                                bp.id.1,
                                bp.blueprint.part_count()
                            );
                            world.blueprints.spawn(id, bp);
                        }
                    }
                }
                WorldBuilderCommand::SpawnShip(bp_id, name, iso) => {
                    let id = if name.is_empty() {
                        spawn_grid_with_random_name(&mut world, bp_id)
                    } else {
                        spawn_grid_with_bp_id(&mut world, &bp_id, &name)
                    };

                    match id {
                        Ok(id) => {
                            self.cursor_grid = Some(id);
                            _ = set_grid_pose(&mut world, id, iso);
                            if let Some(planet_id) = self.cursor_planet {
                                _ = set_as_orbital_parent(&mut world, id, planet_id);
                            }
                        }
                        Err(e) => {
                            error!("Failed to spawn grid: {e:?}");
                        }
                    }
                }
                WorldBuilderCommand::SpawnPart(name) => {
                    let delta = WorldDelta::SpawnPartInNewGrid(name);
                    _ = apply_delta(&mut world, delta);
                }
                WorldBuilderCommand::SetVelocity(vel) => {
                    if let Some(id) = self.cursor_grid {
                        _ = set_grid_vel(&mut world, id, vel);
                    }
                }
                WorldBuilderCommand::ModifyWorld(delta) => {
                    _ = apply_delta(&mut world, delta);
                }
                WorldBuilderCommand::InsertDebugSource(coord, item) => {
                    if let Some(grid_id) = self.cursor_grid {
                        let delta = WorldDelta::InsertPart {
                            grid_id,
                            coord,
                            rotation: Rotation::East,
                            layer: PartLayer::Plumbing,
                            name: "debug-source".to_string(),
                        };
                        _ = apply_delta(&mut world, delta);
                        let delta = WorldDelta::SetSourceItem {
                            grid_id,
                            coord,
                            item,
                        };
                        _ = apply_delta(&mut world, delta);
                    }
                }
                WorldBuilderCommand::InsertPipe(src, dst) => {
                    if let Some(grid_id) = self.cursor_grid {
                        let delta = WorldDelta::InsertPipe { grid_id, src, dst };
                        _ = apply_delta(&mut world, delta);
                    }
                }
                WorldBuilderCommand::SetRecipe(coord, recipe) => {
                    if let Some(grid_id) = self.cursor_grid {
                        let delta = WorldDelta::SetRecipe {
                            grid_id,
                            coord,
                            recipe,
                        };
                        _ = apply_delta(&mut world, delta);
                    }
                }
                WorldBuilderCommand::SpawnAsteroid(p, r, seed) => {
                    let delta = WorldDelta::SpawnAsteroid {
                        iso: p,
                        radius: r,
                        seed,
                    };
                    _ = apply_delta(&mut world, delta);
                }
                WorldBuilderCommand::SpawnPlayer { username, pos, vel } => {
                    let player_id = Player::spawn(&mut world, username, pos, vel);
                    if let Some(id) = self.cursor_planet {
                        _ = Player::set_orbital_parent(&mut world, player_id, Some(id));
                    }
                }
                WorldBuilderCommand::SpawnPlanet {
                    name,
                    pos,
                    vel,
                    mass,
                    radius,
                } => {
                    let id =
                        spawn_planet(&mut world, name, pos, vel, mass, radius, self.cursor_planet);
                    self.cursor_planet = Some(id);
                }
                WorldBuilderCommand::LoadInstruction(ti) => {
                    if let Some(grid_id) = self.cursor_grid {
                        _ = enqueue_grid_instruction(&mut world, grid_id, ti);
                    }
                }
                WorldBuilderCommand::SetAttitudePD(kp, kd) => {
                    if let Some(grid_id) = self.cursor_grid {
                        let delta = WorldDelta::SetGridAttitudePD(grid_id, kp, kd);
                        _ = apply_delta(&mut world, delta);
                    }
                }
            }
        }

        world
    }
}
