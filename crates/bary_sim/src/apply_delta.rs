use crate::*;
use bary_core::prelude::*;
use bary_parts::*;
use log::*;

#[must_use]
pub fn apply_delta(world: &mut World, delta: WorldDelta) -> BaryResult<()> {
    debug!("Applying delta {:?} at tick {}", delta, world.ticks);
    match delta {
        WorldDelta::ToggleTracking(grid_id) => {
            _ = toggle_tracking(world, grid_id);
            Ok(())
        }
        WorldDelta::Explode(loc) => {
            explode_grid_at(loc, world);
            Ok(())
        }
        WorldDelta::ClearAll => {
            *world = World::empty();
            Ok(())
        }
        WorldDelta::Ping(pos) => {
            let particle = PingParticle::new(pos, world.ticks);
            world.particles.push(Particle::Ping(particle));
            Ok(())
        }
        WorldDelta::SpawnShipAt(name, bp_id, iso) => {
            let grid_id = spawn_grid_with_bp_id(world, &bp_id, &name)?;
            set_grid_pose(world, grid_id, iso)?;
            Ok(())
        }
        WorldDelta::FastForwardTo(tick) => {
            if world.ticks < tick {
                let delta = tick - world.ticks;
                warn!("Fast forwarding by {} ticks", delta);
                while world.ticks < tick {
                    update_world(world);
                }
            } else if tick < world.ticks {
                let delta = world.ticks - tick;
                warn!("Already ahead of fast forward directive by {} ticks", delta);
            }
            Ok(())
        }
        WorldDelta::SetWaypoint { grid_id, waypoint } => {
            _ = set_primary_computer_waypoint(grid_id, waypoint, world);
            _ = set_primary_computer_state(grid_id, true, world);
            Ok(())
        }
        WorldDelta::SetWaypointByName { name, waypoint } => {
            if let Some(grid_id) = get_grid_by_name(&world.grids, &name) {
                _ = set_primary_computer_waypoint(grid_id, waypoint, world);
                _ = set_primary_computer_state(grid_id, true, world);
                _ = toggle_tracking(world, grid_id);
            } else {
                warn!("Failed to find grid with name {name}");
            }
            Ok(())
        }
        WorldDelta::DespawnGrid(grid_id) => {
            _ = despawn_grid(world, grid_id);
            Ok(())
        }
        WorldDelta::InsertPart {
            grid_id,
            name,
            coord,
            rotation,
            layer,
        } => {
            let proto_id = get_proto_by_name(&world.prototypes, &name)
                .ok_or(BaryError::NoProtoWithName(name.clone()))?;
            let proto = world.prototypes.try_get(proto_id)?;
            let region = GridRegion::new(coord, rotation, proto.dims);

            let instance = PartInstance {
                name,
                layer,
                region,
            };
            _ = insert_part(grid_id, world, &instance, true);
            Ok(())
        }
        WorldDelta::DestroyPartAt { loc, layer } => {
            if let Some(layer) = layer {
                destroy_part_at_layer(world, loc, layer)?;
            } else {
                destroy_top_part_at(world, loc)?;
            }
            Ok(())
        }
        WorldDelta::SetSourceItem {
            grid_id,
            coord,
            item,
        } => {
            let grid = world.grids.try_get(grid_id)?;
            let occ = grid.get_parts_at(coord).cloned().unwrap_or_default();
            let part_id = occ
                .at_layer(PartLayer::Plumbing)
                .ok_or(BaryError::NoPartsInLayer(PartLayer::Plumbing))?;
            let part = world.debug_portals.try_get_mut(part_id)?;
            if let PortalState::Source(old_item) = &mut part.state {
                *old_item = Some(item);
            }
            Ok(())
        }
        WorldDelta::InsertPipe { grid_id, src, dst } => {
            _ = insert_pipe(grid_id, src, dst, world);
            Ok(())
        }
        WorldDelta::SetRecipe {
            grid_id,
            coord,
            recipe,
        } => {
            let grid = world.grids.try_get(grid_id)?;
            let occ = grid.get_parts_at(coord).cloned().unwrap_or_default();
            let part_id = occ
                .at_layer(PartLayer::Internal)
                .ok_or(BaryError::NoPartsInLayer(PartLayer::Internal))?;
            let machine = world.machines.try_get_mut(part_id)?;
            machine.set_recipe(recipe);
            Ok(())
        }
        WorldDelta::SpawnAsteroid { iso, radius, seed } => {
            spawn_random_asteroid(world, iso, radius, seed);
            Ok(())
        }
        WorldDelta::SpawnPlanet {
            name,
            iso,
            mass,
            radius,
        } => {
            spawn_planet(world, name, iso, mass, radius);
            Ok(())
        }
        WorldDelta::RemoveTerrainTile { asteroid, tile } => {
            remove_terrain_tile(world, asteroid, tile)?;
            Ok(())
        }
        WorldDelta::AddTerrainTile {
            asteroid,
            tile,
            material,
        } => {
            add_terrain_tile(world, asteroid, tile, material)?;
            Ok(())
        }
        WorldDelta::FullyRevealTerrainTile { asteroid, tile } => {
            fully_reveal_terrain_tile(world, asteroid, tile)?;
            Ok(())
        }
        WorldDelta::GoToAsteroid { grid_id, ast_id } => {
            grid_set_waypoint_to_asteroid_center(world, grid_id, ast_id)?;
            Ok(())
        }
        WorldDelta::PlayerPilotingEnterGrid { player_id, grid_id } => {
            set_player_piloting_grid(world, player_id, grid_id)?;
            Ok(())
        }
        WorldDelta::PlayerPilotingExitGrid(player_id) => {
            player_exit_grid(world, player_id)?;
            Ok(())
        }
        WorldDelta::SpawnPlayer(username, iso) => {
            _ = spawn_player(world, username, iso);
            Ok(())
        }
        WorldDelta::SetPlayerPosition(id, iso) => {
            _ = set_player_position(world, id, iso);
            Ok(())
        }
        WorldDelta::SetPlayerCursorPosition(id, pos) => {
            _ = set_player_cursor_position(world, id, pos);
            Ok(())
        }
        WorldDelta::MergeGrids(parent, child, offset, rotation) => {
            merge_grids(world, parent, child, offset, rotation)
        }
        WorldDelta::RenameGrid(grid_id, name) => {
            rename_grid(world, grid_id, name)?;
            Ok(())
        }
        WorldDelta::SetGridBlueprint(grid_id, bp_id) => {
            set_grid_blueprint_id(world, grid_id, bp_id)?;
            Ok(())
        }
        WorldDelta::SetMachineState(machine_id, state) => {
            set_machine_state(world, machine_id, state)?;
            Ok(())
        }
        WorldDelta::SetThrusterState(thruster_id, state) => {
            let grid_id = set_thruster_state(thruster_id, world, state)?;
            update_single_grid_acceleration(
                grid_id,
                &mut world.grids,
                &world.thrusters,
                &world.parts,
            )?;
            Ok(())
        }
        WorldDelta::SetComputerState(cpu_id, state) => {
            set_cpu_state(&mut world.computers, cpu_id, state)?;
            Ok(())
        }
        WorldDelta::SetGridInstruction(grid_id, ti) => {
            set_grid_instruction(world, grid_id, ti)?;
            Ok(())
        }
    }
}
