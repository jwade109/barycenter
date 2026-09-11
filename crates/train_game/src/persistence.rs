use crate::{event_bus::{EventBus, TrainEvent}, node::*, track::*, world::World};
use bary_core::prelude::{Components, Ent, EntitySpawner};
use glam::DVec2;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};

fn load<T: for<'a> Deserialize<'a>>(dir: impl AsRef<Path>, name: &str) -> Option<T> {
    let dir = dir.as_ref();
    let s = std::fs::read_to_string(dir.join(name)).ok()?;
    let comp = serde_yaml::from_str(&s).ok()?;
    Some(comp)
}

fn save<T: Serialize>(val: &T, dir: impl AsRef<Path>, name: &str) -> Option<()> {
    let dir = dir.as_ref();
    let s = serde_yaml::to_string(val).ok()?;
    std::fs::write(dir.join(name), s).ok()?;
    Some(())
}

#[derive(Deserialize, Serialize)]
struct TrackGeometry {
    nodes: Components<DVec2>,
    tracks: Components<Vec<Ent>>,
}

pub fn save_world(world: &World, path: impl AsRef<Path>) -> Option<()> {
    let path = path.as_ref();
    _ = std::fs::create_dir(path);

    let mut spawner = EntitySpawner::default();
    let mut mapping = BTreeMap::new();
    let mut nodes = Components::default();
    let mut tracks = Components::default();

    for (old_id, node) in world.nodes.iter() {
        let new_id = spawner.spawn();
        mapping.insert(*old_id, new_id);
        nodes.spawn(new_id, node.pos());
    }

    for (_, track) in world.segments.iter() {
        let new_id = spawner.spawn();
        let new_ids = track
            .nodes
            .clone()
            .into_iter()
            .map(|old_id| *mapping.get(&old_id).unwrap())
            .collect();
        tracks.spawn(new_id, new_ids);
    }

    let geometry = TrackGeometry { nodes, tracks };

    save(&geometry, path, "geometry.yaml")?;

    Some(())
}

pub fn load_world(world: &mut World, events: &mut EventBus<TrainEvent>, path: impl AsRef<Path>) -> Option<()> {
    let path = path.as_ref();

    let geometry: TrackGeometry = load(path, "geometry.yaml")?;
    let mut mapping = BTreeMap::new();

    world.nodes.clear();
    world.segments.clear();
    world.chunks.clear();
    world.chunk_map.clear();

    for (stored_id, pos) in geometry.nodes.iter() {
        let new_id = spawn_new_node(world, events, *pos);
        mapping.insert(*stored_id, new_id);
    }

    for (_, stored_ids) in geometry.tracks.iter() {
        let new_ids = stored_ids
            .iter()
            .map(|id| *mapping.get(id).unwrap())
            .collect();
        spawn_new_track(world, events, new_ids);
    }

    Some(())
}
