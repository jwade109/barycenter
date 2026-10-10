use crate::*;
use bary_core::prelude::*;
use bary_orbital::{Asteroid, TerrainMaterial};
use early_returns::{ok_or_continue, some_or_continue};
use std::collections::BTreeMap;

pub fn spawn_asteroid(world: &mut World, ast: Asteroid, iso: Isometry2d) -> Ent {
    let parent = world.spawner.spawn();

    let rmax = (ast.max_radius() / TERRAIN_CHUNK_WIDTH_METERS as f32).ceil() as i32;

    let mut chunks = BTreeMap::new();
    let mut edges = Vec::new();

    for x in -rmax..rmax {
        for y in -rmax..rmax {
            let chunk_index = ChunkIndex(IVec2::new(x, y));
            let mut chunk = TerrainChunk::new(parent, chunk_index);

            let is_intersecting = chunk_index
                .bb()
                .corners()
                .iter()
                .any(|c| c.length() < ast.max_radius());

            if !is_intersecting {
                continue;
            }

            let mut tiles = BTreeMap::new();
            for j in 0..TILES_PER_CHUNK_SIDE {
                for k in 0..TILES_PER_CHUNK_SIDE {
                    let index = LocalTileIndex(I8Vec2::new(j as i8, k as i8));
                    let iso = chunk_index.origin_isometry() * index.center_isometry();

                    let Some(material) = ast.material_at(iso.translation.as_vec2()) else {
                        continue;
                    };

                    let is_edge = !index.bb().corners().iter().all(|c| {
                        let c = chunk_index.origin_isometry() * Isometry2d::from_pos(*c);
                        ast.contains(c.translation.as_vec2())
                    });

                    let mut tile = TerrainTile::new(parent, material);
                    tile.set_light_level(10);

                    let tile_id = world.spawner.spawn();
                    world.terrain_tiles.spawn(tile_id, tile);
                    tiles.insert(index, tile_id);

                    if tile.is_visible() {
                        chunk.visible_count += 1;
                    }

                    if is_edge {
                        edges.push((chunk_index, index));
                    }
                }
            }

            if tiles.is_empty() {
                continue;
            }

            chunk.tiles = tiles;

            let chunk_id = world.spawner.spawn();
            world.terrain_chunks.spawn(chunk_id, chunk);
            chunks.insert(chunk_index, chunk_id);
        }
    }

    let rock = BigRock { ast, iso, chunks };

    world.asteroids.spawn(parent, rock);

    parent
}

pub fn spawn_random_asteroid(world: &mut World, iso: Isometry2d, radius: f32, seed: u32) {
    let ast = Asteroid::random(radius, Some(seed));
    spawn_asteroid(world, ast, iso);
}

pub fn remove_terrain_tile(world: &mut World, ast_id: Ent, t: GlobalTileIndex) -> BaryResult<Ent> {
    let rock = world.asteroids.try_get_mut(ast_id)?;
    let (c, l) = t.to_cl();
    let chunk_id = *rock.chunks.get(&c).ok_or(BaryError::NoChunk)?;
    let chunk = world.terrain_chunks.try_get_mut(chunk_id)?;
    let tile_id = *chunk.tiles.get(&l).ok_or(BaryError::NoTile)?;

    world.terrain_tiles.despawn(tile_id)?;

    chunk.tiles.remove(&l);
    if chunk.tiles.is_empty() {
        world.terrain_chunks.despawn(chunk_id)?;
        rock.chunks.remove(&c);
        if rock.chunks.is_empty() {
            world.asteroids.despawn(ast_id)?;
        }
    }

    fully_reveal_terrain_tile(world, ast_id, t)?;

    Ok(tile_id)
}

pub fn add_terrain_tile(
    world: &mut World,
    ast_id: Ent,
    t: GlobalTileIndex,
    material: TerrainMaterial,
) -> BaryResult<Ent> {
    let (c, l) = t.to_cl();
    let rock = world.asteroids.try_get_mut(ast_id)?;
    let chunk_id = *rock.chunks.get(&c).ok_or(BaryError::NoChunk)?;
    let chunk = world.terrain_chunks.try_get_mut(chunk_id)?;

    if chunk.tiles.contains_key(&l) {
        return Err(BaryError::TileAlreadyExists);
    }

    let tile_id = world.spawner.spawn();
    let tile = TerrainTile::new(chunk_id, material);
    chunk.tiles.insert(l, tile_id);
    if tile.is_visible() {
        chunk.visible_count += 1;
    }

    world.terrain_tiles.spawn(tile_id, tile);

    Ok(tile_id)
}

pub fn fully_reveal_terrain_tile(
    world: &mut World,
    ast_id: Ent,
    t: GlobalTileIndex,
) -> BaryResult<()> {
    let rmax = 8;

    let rock = world.asteroids.try_get_mut(ast_id)?;

    for x in -rmax..=rmax {
        for y in -rmax..=rmax {
            // TODO(optimization) we can batch chunk lookups here, since
            // GlobalTileIndex will probably refer to the same chunk

            let d = I8Vec2::new(x, y);
            let t = GlobalTileIndex(t.0 + d.as_ivec2());
            let (c, l) = t.to_cl();
            let chunk_id = *some_or_continue!(rock.chunks.get(&c));
            let chunk = ok_or_continue!(world.terrain_chunks.try_get_mut(chunk_id));
            let r = d.as_vec2().length().round() as i8;
            let ll = if r < 8 { 8 - r as u8 } else { 0 };
            let tile_id = ok_or_continue!(chunk.tiles.get(&l).ok_or(BaryError::NoTile));
            let tile = world.terrain_tiles.try_get_mut(*tile_id)?;
            let visible = tile.is_visible();
            tile.set_light_level(tile.light_level().max(ll));
            if !visible && tile.is_visible() {
                chunk.visible_count += 1;
            }
        }
    }

    Ok(())
}

pub fn mine_terrain_tile(
    world: &mut World,
    ast_id: Ent,
    t: GlobalTileIndex,
) -> BaryResult<TerrainMaterial> {
    let rock = world.asteroids.try_get_mut(ast_id)?;

    let (c, l) = t.to_cl();

    let chunk_id = *rock.chunks.get(&c).ok_or(BaryError::NoChunk)?;
    let chunk = world.terrain_chunks.try_get(chunk_id)?;
    let tile_id = *chunk.tiles.get(&l).ok_or(BaryError::NoTile)?;
    let tile = world.terrain_tiles.try_get(tile_id)?;

    let iso = rock.iso * t.center_isometry();

    let dust = DustParticle::new(iso.translation.as_vec2(), world.ticks);
    world.particles.push(Particle::Dust(dust));

    Ok(tile.material())
}
