use std::collections::BTreeMap;

use bary_core::prelude::*;
use bary_orbital::{Asteroid, TerrainMaterial};
use serde::{Deserialize, Serialize};

pub const TERRAIN_TILE_WIDTH_METERS: f64 = 0.5;
pub const PIXELS_IN_TERRAIN_TILE: u8 = 10;
pub const TILES_PER_CHUNK_SIDE: u8 = 32;
pub const TERRAIN_CHUNK_WIDTH_METERS: f64 = TERRAIN_TILE_WIDTH_METERS * TILES_PER_CHUNK_SIDE as f64;
pub const TERRAIN_VARIANTS: u8 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalTileIndex(pub I8Vec2);

impl LocalTileIndex {
    pub fn origin_isometry(&self) -> Isometry2d {
        let bottom_left = self.0.as_dvec2() * TERRAIN_TILE_WIDTH_METERS as f64;
        Isometry2d::from_pos(bottom_left)
    }

    pub fn center_isometry(&self) -> Isometry2d {
        let bottom_left =
            (self.0.as_dvec2() + DVec2::splat(0.5)) * TERRAIN_TILE_WIDTH_METERS as f64;
        Isometry2d::from_pos(bottom_left)
    }

    pub fn top_left_isometry(&self) -> Isometry2d {
        let idx = self.0 + I8Vec2::Y;
        let top_left = idx.as_dvec2() * TERRAIN_TILE_WIDTH_METERS as f64;
        Isometry2d::from_pos(top_left)
    }

    pub fn center(&self) -> DVec2 {
        let bl = self.origin_isometry().translation;
        bl + DVec2::splat(TERRAIN_TILE_WIDTH_METERS as f64 / 2.0)
    }

    pub fn bb(&self) -> AABB {
        AABB::new(
            self.center(),
            DVec2::splat(TERRAIN_TILE_WIDTH_METERS as f64),
        )
    }
}

impl PartialOrd for LocalTileIndex {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        (self.0.x, self.0.y).partial_cmp(&(other.0.x, other.0.y))
    }
}

impl Ord for LocalTileIndex {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.0.x, self.0.y).cmp(&(other.0.x, other.0.y))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct GlobalTileIndex(pub IVec2);

impl PartialOrd for GlobalTileIndex {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        (self.0.x, self.0.y).partial_cmp(&(other.0.x, other.0.y))
    }
}

impl Ord for GlobalTileIndex {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.0.x, self.0.y).cmp(&(other.0.x, other.0.y))
    }
}

impl GlobalTileIndex {
    pub fn to_cl(&self) -> (ChunkIndex, LocalTileIndex) {
        let tpcs = TILES_PER_CHUNK_SIDE as i32;
        let cx = if self.0.x >= 0 {
            self.0.x / tpcs
        } else {
            (self.0.x + 1 - tpcs) / tpcs
        };
        let cy = if self.0.y >= 0 {
            self.0.y / tpcs
        } else {
            (self.0.y + 1 - tpcs) / tpcs
        };

        let chunk = ChunkIndex((cx, cy).into());
        let l = self.0 - chunk.0 * TILES_PER_CHUNK_SIDE as i32;
        let local = LocalTileIndex(l.as_i8vec2());
        (chunk, local)
    }

    pub fn origin_isometry(&self) -> Isometry2d {
        Isometry2d::from_pos(self.0.as_dvec2() * TERRAIN_TILE_WIDTH_METERS)
    }

    pub fn center_isometry(&self) -> Isometry2d {
        Isometry2d::from_pos((self.0.as_dvec2() + DVec2::splat(0.5)) * TERRAIN_TILE_WIDTH_METERS)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct ChunkIndex(pub IVec2);

impl PartialOrd for ChunkIndex {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        (self.0.x, self.0.y).partial_cmp(&(other.0.x, other.0.y))
    }
}

impl Ord for ChunkIndex {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.0.x, self.0.y).cmp(&(other.0.x, other.0.y))
    }
}

impl ChunkIndex {
    pub fn origin_isometry(&self) -> Isometry2d {
        let bottom_left = self.0.as_dvec2() * TERRAIN_CHUNK_WIDTH_METERS as f64;
        Isometry2d::from_pos(bottom_left)
    }

    pub fn top_left_isometry(&self) -> Isometry2d {
        let idx = self.0 + IVec2::Y;
        let top_left = idx.as_dvec2() * TERRAIN_CHUNK_WIDTH_METERS as f64;
        Isometry2d::from_pos(top_left)
    }

    pub fn center(&self) -> DVec2 {
        let bl = self.origin_isometry().translation;
        bl + DVec2::splat(TERRAIN_CHUNK_WIDTH_METERS as f64 / 2.0)
    }

    pub fn center_isometry(&self) -> Isometry2d {
        Isometry2d::from_pos(self.center())
    }

    pub fn bb(&self) -> AABB {
        AABB::new(
            self.center(),
            DVec2::splat(TERRAIN_CHUNK_WIDTH_METERS as f64),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainIndex(pub IVec2, pub I8Vec2);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BigRock {
    pub iso: Isometry2d,
    pub ast: Asteroid,
    pub chunks: BTreeMap<ChunkIndex, Ent>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TerrainChunk {
    pub parent: Ent,
    pub index: ChunkIndex,
    pub tiles: BTreeMap<LocalTileIndex, Ent>,
    pub visible_count: usize,
}

impl TerrainChunk {
    pub fn new(parent: Ent, index: ChunkIndex) -> Self {
        Self {
            parent,
            index,
            tiles: BTreeMap::new(),
            visible_count: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct TerrainTile {
    parent: Ent,
    material: TerrainMaterial,
    variant: usize,
    light_level: u8,
}

impl TerrainTile {
    pub fn new(parent: Ent, material: TerrainMaterial) -> Self {
        Self {
            parent,
            material,
            variant: randint(0, TERRAIN_VARIANTS as i32) as usize,
            light_level: 0,
        }
    }

    pub fn parent(&self) -> Ent {
        self.parent
    }

    pub fn material(&self) -> TerrainMaterial {
        self.material
    }

    pub fn variant(&self) -> usize {
        self.variant
    }

    pub fn light_level(&self) -> u8 {
        self.light_level
    }

    pub fn set_light_level(&mut self, ll: u8) {
        self.light_level = ll;
    }

    pub fn is_visible(&self) -> bool {
        self.light_level > 0
    }
}
