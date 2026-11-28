use crate::{World, WorldBuilder};
use bary_core::prelude::*;
use log::*;
use postcard::to_allocvec;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub fn save_world(
    dir: impl AsRef<Path>,
    world: &World,
    overwrite: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = dir.as_ref();

    if std::fs::exists(dir)? {
        if overwrite {
            std::fs::remove_dir_all(dir)?;
        } else {
            return Err(BaryError::SaveAlreadyExists.into());
        }
    }

    std::fs::create_dir_all(dir)?;

    info!("Saving world to {}", dir.display());

    let s = to_allocvec(&world.ticks)?;
    std::fs::write(dir.join("ticks.bin"), s)?;

    save(&world.spawner, dir, "spawner.bin")?;
    save(&world.blueprints, dir, "blueprints.bin")?;

    let s = to_allocvec(&world.prototypes)?;
    std::fs::write(dir.join("prototypes.bin"), s)?;

    let s = to_allocvec(&world.grids)?;
    std::fs::write(dir.join("grids.bin"), s)?;

    let s = to_allocvec(&world.parts)?;
    std::fs::write(dir.join("parts.bin"), s)?;

    let s = to_allocvec(&world.thrusters)?;
    std::fs::write(dir.join("thrusters.bin"), s)?;

    let s = to_allocvec(&world.computers)?;
    std::fs::write(dir.join("computers.bin"), s)?;

    let s = to_allocvec(&world.lights)?;
    std::fs::write(dir.join("lights.bin"), s)?;

    let s = to_allocvec(&world.inventories)?;
    std::fs::write(dir.join("inventories.bin"), s)?;

    let s = to_allocvec(&world.machines)?;
    std::fs::write(dir.join("machines.bin"), s)?;

    let s = to_allocvec(&world.debug_portals)?;
    std::fs::write(dir.join("debug_portals.bin"), s)?;

    let s = to_allocvec(&world.pipes)?;
    std::fs::write(dir.join("pipes.bin"), s)?;

    let s = to_allocvec(&world.excavators)?;
    std::fs::write(dir.join("excavators.bin"), s)?;

    let s = to_allocvec(&world.asteroids)?;
    std::fs::write(dir.join("asteroids.bin"), s)?;

    let s = to_allocvec(&world.planets)?;
    std::fs::write(dir.join("planets.bin"), s)?;

    let s = comp(to_allocvec(&world.terrain_chunks)?);
    std::fs::write(dir.join("terrain_chunks.bin"), s)?;

    let s = comp(to_allocvec(&world.terrain_tiles)?);
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

fn load<T: for<'a> Deserialize<'a>>(
    dir: &Path,
    name: &str,
) -> Result<T, Box<dyn std::error::Error>> {
    let s = decomp(std::fs::read(dir.join(name))?);
    let comp = postcard::from_bytes(&s)?;
    Ok(comp)
}

fn save<T: Serialize>(val: &T, dir: &Path, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let s = comp(to_allocvec(val)?);
    std::fs::write(dir.join(name), s)?;
    Ok(())
}

pub fn load_yaml_scenario(path: impl AsRef<Path>) -> Result<World, Box<dyn std::error::Error>> {
    Ok(WorldBuilder::new().assets().parse_file(path)?.build())
}

pub fn load_world(dir: impl AsRef<Path>) -> Result<World, Box<dyn std::error::Error>> {
    let dir = dir.as_ref();
    info!("Loading world from {}", dir.display());

    if dir.extension().map(|s| s.to_string_lossy().to_string()) == Some("yaml".to_string()) {
        return load_yaml_scenario(dir);
    }

    let mut world = World::empty();

    let s = std::fs::read(dir.join("ticks.bin"))?;
    world.ticks = postcard::from_bytes(&s)?;

    world.spawner = load(dir, "spawner.bin")?;
    world.blueprints = load(dir, "blueprints.bin")?;

    let s = std::fs::read(dir.join("prototypes.bin"))?;
    world.prototypes = postcard::from_bytes(&s)?;

    let s = std::fs::read(dir.join("inventories.bin"))?;
    world.inventories = postcard::from_bytes(&s)?;

    let s = std::fs::read(dir.join("machines.bin"))?;
    world.machines = postcard::from_bytes(&s)?;

    let s = std::fs::read(dir.join("debug_portals.bin"))?;
    world.debug_portals = postcard::from_bytes(&s)?;

    let s = std::fs::read(dir.join("pipes.bin"))?;
    world.pipes = postcard::from_bytes(&s)?;

    let s = std::fs::read(dir.join("grids.bin"))?;
    world.grids = postcard::from_bytes(&s)?;

    let s = std::fs::read(dir.join("parts.bin"))?;
    world.parts = postcard::from_bytes(&s)?;

    let s = std::fs::read(dir.join("lights.bin"))?;
    world.lights = postcard::from_bytes(&s)?;

    let s = std::fs::read(dir.join("thrusters.bin"))?;
    world.thrusters = postcard::from_bytes(&s)?;

    let s = std::fs::read(dir.join("computers.bin"))?;
    world.computers = postcard::from_bytes(&s)?;

    let s = std::fs::read(dir.join("excavators.bin"))?;
    world.excavators = postcard::from_bytes(&s)?;

    let s = std::fs::read(dir.join("asteroids.bin"))?;
    world.asteroids = postcard::from_bytes(&s)?;

    let s = std::fs::read(dir.join("planets.bin"))?;
    world.planets = postcard::from_bytes(&s)?;

    let s = decomp(std::fs::read(dir.join("terrain_chunks.bin"))?);
    world.terrain_chunks = postcard::from_bytes(&s)?;

    let s = decomp(std::fs::read(dir.join("terrain_tiles.bin"))?);
    world.terrain_tiles = postcard::from_bytes(&s)?;

    let s = decomp(std::fs::read(dir.join("terrain_tiles.bin"))?);
    world.terrain_tiles = postcard::from_bytes(&s)?;

    world.players = load(dir, "players.bin")?;

    Ok(world)
}

pub fn list_saves_in_dir(dir: impl AsRef<Path>) -> Vec<PathBuf> {
    let mut ret = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            ret.push(e.path());
        }
    }
    ret
}
