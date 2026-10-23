use crate::{World, WorldBuilder};
use bary_core::prelude::*;
use log::*;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub fn save_world(dir: impl AsRef<Path>, world: &World, overwrite: bool) -> BaryResult<()> {
    let dir = dir.as_ref();

    if std::fs::exists(dir)? {
        if overwrite {
            std::fs::remove_dir_all(dir)?;
        } else {
            return Err(BaryError::SaveAlreadyExists);
        }
    }

    std::fs::create_dir_all(dir)?;

    info!("Saving world to {}", dir.display());

    let s = bincode::serialize(&world.ticks).map_err(|_| BaryError::BincodeError)?;
    std::fs::write(dir.join("ticks.bin"), s)?;

    save(&world.spawner, &dir, "spawner.bin")?;
    save(&world.blueprints, &dir, "blueprints.bin")?;

    let s = bincode::serialize(&world.prototypes).map_err(|_| BaryError::BincodeError)?;
    std::fs::write(dir.join("prototypes.bin"), s)?;

    let s = bincode::serialize(&world.grids).map_err(|_| BaryError::BincodeError)?;
    std::fs::write(dir.join("grids.bin"), s)?;

    let s = bincode::serialize(&world.parts).map_err(|_| BaryError::BincodeError)?;
    std::fs::write(dir.join("parts.bin"), s)?;

    let s = bincode::serialize(&world.thrusters).map_err(|_| BaryError::BincodeError)?;
    std::fs::write(dir.join("thrusters.bin"), s)?;

    let s = bincode::serialize(&world.computers).map_err(|_| BaryError::BincodeError)?;
    std::fs::write(dir.join("computers.bin"), s)?;

    let s = bincode::serialize(&world.lights).map_err(|_| BaryError::BincodeError)?;
    std::fs::write(dir.join("lights.bin"), s)?;

    let s = bincode::serialize(&world.inventories).map_err(|_| BaryError::BincodeError)?;
    std::fs::write(dir.join("inventories.bin"), s)?;

    let s = bincode::serialize(&world.machines).map_err(|_| BaryError::BincodeError)?;
    std::fs::write(dir.join("machines.bin"), s)?;

    let s = bincode::serialize(&world.debug_portals).map_err(|_| BaryError::BincodeError)?;
    std::fs::write(dir.join("debug_portals.bin"), s)?;

    let s = bincode::serialize(&world.pipes).map_err(|_| BaryError::BincodeError)?;
    std::fs::write(dir.join("pipes.bin"), s)?;

    let s = bincode::serialize(&world.excavators).map_err(|_| BaryError::BincodeError)?;
    std::fs::write(dir.join("excavators.bin"), s)?;

    let s = bincode::serialize(&world.asteroids).map_err(|_| BaryError::BincodeError)?;
    std::fs::write(dir.join("asteroids.bin"), s)?;

    let s = bincode::serialize(&world.planets).map_err(|_| BaryError::BincodeError)?;
    std::fs::write(dir.join("planets.bin"), s)?;

    let s = comp(bincode::serialize(&world.terrain_chunks).map_err(|_| BaryError::BincodeError)?);
    std::fs::write(dir.join("terrain_chunks.bin"), s)?;

    let s = comp(bincode::serialize(&world.terrain_tiles).map_err(|_| BaryError::BincodeError)?);
    std::fs::write(dir.join("terrain_tiles.bin"), s)?;

    save(&world.players, dir, "players.bin")?;

    Ok(())
}

fn comp(bytes: Vec<u8>) -> Vec<u8> {
    let reader = std::io::Cursor::new(bytes);
    zstd::stream::encode_all(reader, 0).unwrap()
}

fn decomp(bytes: Vec<u8>) -> Vec<u8> {
    let reader = std::io::Cursor::new(bytes);
    zstd::stream::decode_all(reader).unwrap()
}

fn load<T: for<'a> Deserialize<'a>>(dir: &Path, name: &str) -> BaryResult<T> {
    let s = decomp(std::fs::read(dir.join(name))?);
    let comp = bincode::deserialize(&s).map_err(|_| BaryError::BincodeError)?;
    Ok(comp)
}

fn save<T: Serialize>(val: &T, dir: &Path, name: &str) -> BaryResult<()> {
    let s = comp(bincode::serialize(val).map_err(|_| BaryError::BincodeError)?);
    std::fs::write(dir.join(name), s)?;
    Ok(())
}

pub fn load_yaml_scenario(path: impl AsRef<Path>) -> BaryResult<World> {
    Ok(WorldBuilder::new().assets().parse_file(path).build())
}

pub fn load_world(dir: impl AsRef<Path>) -> BaryResult<World> {
    let dir = dir.as_ref();
    info!("Loading world from {}", dir.display());

    if dir.extension().map(|s| s.to_string_lossy().to_string()) == Some("yaml".to_string()) {
        return load_yaml_scenario(dir);
    }

    let mut world = World::empty();

    let s = std::fs::read(dir.join("ticks.bin"))?;
    world.ticks = bincode::deserialize(&s).map_err(|_| BaryError::BincodeError)?;

    world.spawner = load(dir, "spawner.bin")?;
    world.blueprints = load(dir, "blueprints.bin")?;

    let s = std::fs::read(dir.join("prototypes.bin"))?;
    world.prototypes = bincode::deserialize(&s).map_err(|_| BaryError::BincodeError)?;

    let s = std::fs::read(dir.join("inventories.bin"))?;
    world.inventories = bincode::deserialize(&s).map_err(|_| BaryError::BincodeError)?;

    let s = std::fs::read(dir.join("machines.bin"))?;
    world.machines = bincode::deserialize(&s).map_err(|_| BaryError::BincodeError)?;

    let s = std::fs::read(dir.join("debug_portals.bin"))?;
    world.debug_portals = bincode::deserialize(&s).map_err(|_| BaryError::BincodeError)?;

    let s = std::fs::read(dir.join("pipes.bin"))?;
    world.pipes = bincode::deserialize(&s).map_err(|_| BaryError::BincodeError)?;

    let s = std::fs::read(dir.join("grids.bin"))?;
    world.grids = bincode::deserialize(&s).map_err(|_| BaryError::BincodeError)?;

    let s = std::fs::read(dir.join("parts.bin"))?;
    world.parts = bincode::deserialize(&s).map_err(|_| BaryError::BincodeError)?;

    let s = std::fs::read(dir.join("lights.bin"))?;
    world.lights = bincode::deserialize(&s).map_err(|_| BaryError::BincodeError)?;

    let s = std::fs::read(dir.join("thrusters.bin"))?;
    world.thrusters = bincode::deserialize(&s).map_err(|_| BaryError::BincodeError)?;

    let s = std::fs::read(dir.join("computers.bin"))?;
    world.computers = bincode::deserialize(&s).map_err(|_| BaryError::BincodeError)?;

    let s = std::fs::read(dir.join("excavators.bin"))?;
    world.excavators = bincode::deserialize(&s).map_err(|_| BaryError::BincodeError)?;

    let s = std::fs::read(dir.join("asteroids.bin"))?;
    world.asteroids = bincode::deserialize(&s).map_err(|_| BaryError::BincodeError)?;

    let s = std::fs::read(dir.join("planets.bin"))?;
    world.planets = bincode::deserialize(&s).map_err(|_| BaryError::BincodeError)?;

    let s = decomp(std::fs::read(dir.join("terrain_chunks.bin"))?);
    world.terrain_chunks = bincode::deserialize(&s).map_err(|_| BaryError::BincodeError)?;

    let s = decomp(std::fs::read(dir.join("terrain_tiles.bin"))?);
    world.terrain_tiles = bincode::deserialize(&s).map_err(|_| BaryError::BincodeError)?;

    let s = decomp(std::fs::read(dir.join("terrain_tiles.bin"))?);
    world.terrain_tiles = bincode::deserialize(&s).map_err(|_| BaryError::BincodeError)?;

    world.players = load(dir, "players.bin")?;

    Ok(world)
}

pub fn list_saves_in_dir(dir: impl AsRef<Path>) -> Vec<PathBuf> {
    let mut ret = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries {
            if let Ok(e) = entry {
                ret.push(e.path());
            }
        }
    }
    ret
}
