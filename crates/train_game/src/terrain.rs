use crate::{
    event_bus::{EventBus, TrainEvent},
    render_world::RenderWorld,
    world::World,
};
use bary_core::prelude::{Ent, Isometry2d, linspace_f64, vfloor_f64};
use glam::{DVec2, IVec2};
use log::warn;
use noise::{NoiseFn, Perlin};
use rend::TextureHandle;
use std::collections::BTreeSet;

pub const TERRAIN_CHUNK_WIDTH_METERS: f64 = 3000.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkIndex(IVec2);

impl PartialOrd for ChunkIndex {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let a = (self.0.x, self.0.y);
        let b = (other.0.x, other.0.y);
        a.partial_cmp(&b)
    }
}

impl Ord for ChunkIndex {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let a = (self.0.x, self.0.y);
        let b = (other.0.x, other.0.y);
        a.cmp(&b)
    }
}

impl From<IVec2> for ChunkIndex {
    fn from(value: IVec2) -> Self {
        Self(value)
    }
}

impl ChunkIndex {
    pub const ZERO: Self = Self(IVec2::ZERO);

    pub fn new(index: impl Into<IVec2>) -> Self {
        Self(index.into())
    }

    pub fn isometry(&self) -> Isometry2d {
        let pos = TERRAIN_CHUNK_WIDTH_METERS * self.0.as_dvec2();
        pos.into()
    }

    pub fn as_ivec2(&self) -> IVec2 {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct TerrainChunk {
    index: ChunkIndex,
    tracks: BTreeSet<Ent>,
    nodes: BTreeSet<Ent>,
    height: [f32; 4],
    pub trees: Vec<DVec2>,
    pub texture: Option<TextureHandle>,
}

fn height_func(pos: DVec2) -> f64 {
    let perlin = Perlin::new(8525);

    let x = pos.x;
    let z = pos.y;

    let y1 = perlin.get([x as f64 / 50.0, 0.5, z as f64 / 50.0 + 0.5]);
    let y2 = perlin.get([x as f64 / 500.0 + 0.5, 0.5, z as f64 / 500.0 + 0.5]);
    let y3 = perlin.get([x as f64 / 3000.0, 0.5, z as f64 / 3000.0 + 0.5]);
    let y4 = perlin.get([x as f64 / 10000.0, 0.5, z as f64 / 10000.0 + 0.5]);
    return y1 * 0.5 + y2 * 5.0 + y3 * 10.0 + y4 * 30.0 + 10.0;
}

impl TerrainChunk {
    pub fn new(index: impl Into<ChunkIndex>) -> Self {
        let index = index.into();
        let a = index.isometry().tr();
        let b = a + DVec2::X * TERRAIN_CHUNK_WIDTH_METERS;
        let c = a + DVec2::splat(TERRAIN_CHUNK_WIDTH_METERS);
        let d = a + DVec2::Y * TERRAIN_CHUNK_WIDTH_METERS;

        let trees = (0..300)
            .filter_map(|_| {
                let x = bary_core::prelude::rand(0.0, TERRAIN_CHUNK_WIDTH_METERS as f32) as f64;
                let y = bary_core::prelude::rand(0.0, TERRAIN_CHUNK_WIDTH_METERS as f32) as f64;
                let p = DVec2::new(x, y);
                let h = height_func(index.isometry().tr() + p);
                (h > 0.0 && h < 17.0).then_some(p)
            })
            .collect();

        Self {
            index,
            tracks: BTreeSet::new(),
            nodes: BTreeSet::new(),
            height: [
                height_func(a) as f32,
                height_func(b) as f32,
                height_func(c) as f32,
                height_func(d) as f32,
            ],
            trees,
            texture: None,
        }
    }

    pub fn index(&self) -> ChunkIndex {
        self.index
    }

    pub fn add_track(&mut self, track_id: Ent) {
        self.tracks.insert(track_id);
    }

    pub fn add_node(&mut self, node_id: Ent) {
        self.nodes.insert(node_id);
    }

    pub fn remove_track(&mut self, track_id: Ent) {
        self.tracks.remove(&track_id);
    }

    pub fn remove_node(&mut self, node_id: Ent) {
        self.nodes.remove(&node_id);
    }

    pub fn nodes(&self) -> &BTreeSet<Ent> {
        &self.nodes
    }

    pub fn tracks(&self) -> &BTreeSet<Ent> {
        &self.tracks
    }

    pub fn isometry(&self) -> Isometry2d {
        self.index.isometry()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty() && self.tracks.is_empty()
    }

    pub fn height(&self) -> [f32; 4] {
        self.height
    }
}

pub fn get_chunk_index(pos: impl Into<DVec2>) -> ChunkIndex {
    ChunkIndex::new(vfloor_f64(pos.into() / TERRAIN_CHUNK_WIDTH_METERS))
}

pub fn spawn_new_chunk(
    world: &mut World,
    events: &mut EventBus<TrainEvent>,
    index: impl Into<ChunkIndex>,
) -> Option<Ent> {
    let index = index.into();
    if world.chunk_map.contains_key(&index) {
        return None;
    }

    let chunk = TerrainChunk::new(index);

    let id = world.spawner.spawn();
    world.chunks.spawn(id, chunk);
    world.chunk_map.insert(index, id);

    events.enqueue(TrainEvent::ChunkUpdate(id));

    Some(id)
}

pub fn ensure_chunk_exists(
    world: &mut World,
    events: &mut EventBus<TrainEvent>,
    index: ChunkIndex,
) {
    for x in -2..=2 {
        for y in -2..=2 {
            let idx = index.as_ivec2() + IVec2::new(x, y);
            let idx = ChunkIndex::new(idx);
            spawn_new_chunk(world, events, idx);
        }
    }
}

#[allow(unused)]
pub fn remove_chunk_if_empty(world: &mut World, id: Ent, index: ChunkIndex) -> Option<()> {
    let chunk = world.chunks.get(id)?;
    if !chunk.is_empty() {
        return Some(());
    }

    _ = world.chunks.despawn(id);
    world.chunk_map.remove(&index);

    Some(())
}

pub fn chunk_register_track(
    world: &mut World,
    events: &mut EventBus<TrainEvent>,
    index: ChunkIndex,
    track_id: Ent,
) -> Option<()> {
    ensure_chunk_exists(world, events, index);
    let chunk_id = world.chunk_map.get(&index)?;
    let chunk = world.chunks.try_get_mut(*chunk_id).ok()?;
    chunk.add_track(track_id);
    Some(())
}

pub fn chunk_deregister_track(world: &mut World, index: ChunkIndex, track_id: Ent) -> Option<()> {
    let chunk_id = world.chunk_map.get(&index)?;
    let chunk = world.chunks.try_get_mut(*chunk_id).ok()?;
    chunk.remove_track(track_id);
    // remove_chunk_if_empty(world, *chunk_id, index);
    Some(())
}

pub fn chunk_register_node(
    world: &mut World,
    events: &mut EventBus<TrainEvent>,
    index: ChunkIndex,
    node_id: Ent,
) -> Option<()> {
    ensure_chunk_exists(world, events, index);
    let chunk_id = world.chunk_map.get(&index)?;
    let chunk = world.chunks.try_get_mut(*chunk_id).ok()?;
    chunk.add_node(node_id);
    Some(())
}

pub fn chunk_deregister_node(world: &mut World, index: ChunkIndex, node_id: Ent) -> Option<()> {
    let chunk_id = world.chunk_map.get(&index)?;
    let chunk = world.chunks.try_get_mut(*chunk_id).ok()?;
    chunk.remove_node(node_id);
    // remove_chunk_if_empty(world, *chunk_id, index);
    Some(())
}
