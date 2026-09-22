use crate::{
    event_bus::{EventBus, TrainEvent},
    render_state::RenderState,
    render_world::RenderWorld,
    world::World,
};
use bary_core::prelude::*;
use early_returns::{ok_or_continue, some_or_continue};
use glam::{DVec2, DVec4, IVec2, Vec4};
use log::warn;
use noise::{NoiseFn, Perlin};
use rend::{
    Color, FullVertex, Mesh, TextureHandle, make_quad_01, mesh_from_vi, quad_indices_to_tris,
};
use std::collections::BTreeSet;

pub const TERRAIN_CHUNK_WIDTH_METERS: f64 = 1000.0;

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

#[derive(Debug, Clone, Copy)]
pub struct Tree {
    pub pos: DVec2,
    pub radius: f64,
    pub color: Color,
}

#[derive(Debug, Clone)]
pub struct TerrainChunk {
    index: ChunkIndex,
    tracks: BTreeSet<Ent>,
    nodes: BTreeSet<Ent>,
    pub trees: Vec<Tree>,
    pub terrain_mesh: Option<Ent>,
    pub tree_mesh: Option<Ent>,
}

fn height_func(pos: DVec2) -> f64 {
    let perlin = Perlin::new(8525);

    let x = pos.x;
    let z = pos.y;

    let y1 = perlin.get([x as f64 / 50.0 + 0.5, 0.5, z as f64 / 50.0 + 0.5]);
    let y2 = perlin.get([x as f64 / 500.0 + 0.5, 0.5, z as f64 / 500.0 + 0.5]);
    let y3 = perlin.get([x as f64 / 3000.0 + 0.5, 0.5, z as f64 / 3000.0 + 0.5]);
    return y1 * 0.2 + y2 * 5.0 + y3 * 10.0;
}

impl TerrainChunk {
    pub fn new(index: impl Into<ChunkIndex>) -> Self {
        let index = index.into();
        let a = index.isometry().tr();
        let b = a + DVec2::X * TERRAIN_CHUNK_WIDTH_METERS;
        let c = a + DVec2::splat(TERRAIN_CHUNK_WIDTH_METERS);
        let d = a + DVec2::Y * TERRAIN_CHUNK_WIDTH_METERS;

        Self {
            index,
            tracks: BTreeSet::new(),
            nodes: BTreeSet::new(),
            trees: Vec::new(),
            terrain_mesh: None,
            tree_mesh: None,
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

    pub fn height_func(p: DVec2) -> f64 {
        height_func(p)
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

    // events.enqueue(TrainEvent::ChunkUpdate(id));

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

pub fn regenerate_trees(world: &mut World, index: ChunkIndex) -> Option<()> {
    let chunk_id = world.chunk_map.get(&index)?;
    let chunk = world.chunks.try_get_mut(*chunk_id).ok()?;

    if !chunk.trees.is_empty() {
        return Some(());
    }

    chunk.trees = (0..16000)
        .filter_map(|_| {
            let x = rand(0.0, TERRAIN_CHUNK_WIDTH_METERS as f32) as f64;
            let y = rand(0.0, TERRAIN_CHUNK_WIDTH_METERS as f32) as f64;
            let r = rand(2.0, 8.0) as f64;
            let p = index.isometry().tr() + DVec2::new(x, y);
            let h = height_func(p);
            let color = Color::hsl(
                rand(0.02, 0.3) as f64,
                rand(0.4, 0.7) as f64,
                rand(0.2, 0.3) as f64,
                0.8,
            );

            let shore: f64 = h / 4.0;
            let flats: f64 = 1.0;
            let slopes: f64 = (7.0 - h) / 5.0;

            let prob = shore.min(flats.min(slopes)).clamp(0.0, 1.0);

            chance(prob as f32).then_some(Tree {
                pos: p,
                radius: r,
                color,
            })
        })
        .collect();

    Some(())
}

pub fn handle_regen_trees_events(world: &mut World, events: &EventBus<TrainEvent>) {
    for event in events.iter() {
        if let TrainEvent::RegenerateTrees(index) = event {
            regenerate_trees(world, *index);
        }
    }
}

pub fn get_quadtile(pos: DVec2) -> [ChunkIndex; 4] {
    let mut root = get_chunk_index(pos);

    let normalized = (pos - root.isometry().tr()) / TERRAIN_CHUNK_WIDTH_METERS;

    if normalized.x < 0.5 {
        root = ChunkIndex(root.as_ivec2() - IVec2::X);
    }
    if normalized.y < 0.5 {
        root = ChunkIndex(root.as_ivec2() - IVec2::Y);
    }

    let b = ChunkIndex::new(root.as_ivec2() + IVec2::X);
    let c = ChunkIndex::new(root.as_ivec2() + IVec2::Y);
    let d = ChunkIndex::new(root.as_ivec2() + IVec2::ONE);

    [root, b, c, d]
}

pub fn update_chunk_texture(
    rs: &RenderState,
    rw: &mut RenderWorld,
    world: &mut World,
    index: ChunkIndex,
) -> Option<()> {
    let chunk_id = world.chunk_map.get(&index)?;
    let chunk = world.chunks.get(*chunk_id)?;

    if chunk.terrain_mesh.is_some() {
        return Some(());
    }

    let id = rw.spawner.spawn();

    warn!("Spawning texture for chunk {:?}", chunk.index());

    let chunk = world.chunks.try_get_mut(*chunk_id).ok()?;

    let mesh = make_rough_ground_plane(&rs.renderer.device, chunk.isometry().tr(), 100);

    let id = rw.spawn_mesh(mesh);
    chunk.terrain_mesh = Some(id);

    if !chunk.trees.is_empty() {
        let mesh = make_tree_mesh(&rs.renderer.device, chunk);
        let id = rw.spawn_mesh(mesh);
        chunk.tree_mesh = Some(id);
    }

    Some(())
}

pub fn make_tree_mesh(device: &wgpu::Device, chunk: &TerrainChunk) -> Mesh {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let da = PI_64 * 2.0 / 3.0;
    for tree in &chunk.trees {
        let offset = tree.pos - chunk.isometry().tr();
        let z = rand(3.0, 7.0);
        let r = rand(3.0, 8.0) as f64;
        for _ in 0..2 {
            let a0 = rand(0.0, PI * 2.0) as f64;
            let root = indices.len() as u16;
            indices.extend([root, root + 1, root + 2]);
            for i in 0..3 {
                let a = a0 + i as f64 * da;
                let p = offset + rotate_f64(DVec2::X, a) * r;
                let position = glm::Vec3::new(p.x as f32, p.y as f32, z);
                let color = glm::Vec4::new(0.6, 0.4, 0.6, 1.0);
                let tex_coord = glm::Vec2::new(0.0, 0.0);
                let vertex = FullVertex::new(position, color, tex_coord);
                vertices.push(vertex);
            }
        }
    }

    mesh_from_vi::<FullVertex>(device, &vertices, &indices)
}

pub fn make_rough_ground_plane(device: &wgpu::Device, origin: DVec2, n_quads: u16) -> Mesh {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let n_quads_x = n_quads;
    let n_quads_y = n_quads;

    let perlin = Perlin::new(1);

    for xi in linspace_f64(0.0, TERRAIN_CHUNK_WIDTH_METERS, n_quads_x as usize + 1) {
        for yi in linspace_f64(0.0, TERRAIN_CHUNK_WIDTH_METERS, n_quads_y as usize + 1) {
            let p = origin + DVec2::new(xi as f64, yi as f64);
            let z = TerrainChunk::height_func(p);
            let position = glm::Vec3::new(xi as f32, yi as f32, z as f32);
            let color = glm::Vec4::new(0.2, 0.6, 1.0, 1.0);
            let tex_coord = glm::Vec2::new(0.0, 0.0);
            let v = FullVertex::new(position, color, tex_coord);
            vertices.push(v);
        }
    }

    for x in 0..n_quads_x {
        for y in 0..n_quads_y {
            let stride = n_quads_y + 1;
            let b = x + y * (n_quads_y + 1);

            let p1 = b;
            let p2 = b + 1;
            let p3 = b + stride + 1;
            let p4 = b + stride;

            indices.extend(quad_indices_to_tris(p4, p3, p2, p1));
        }
    }

    mesh_from_vi(device, &vertices, &indices)
}
