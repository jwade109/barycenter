use crate::*;
use bary_core::prelude::*;
use bary_factory::*;
use bary_orbital::{TerrainMaterial, VehicleControl};
use bary_parts::*;
use early_returns::*;
use log::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

pub fn get_random_ship_name(names: &Vec<String>) -> String {
    if names.is_empty() {
        return String::new();
    }
    let idx = randint(0, names.len() as i32) as usize;
    names[idx].clone()
}

pub fn load_names_from_file(
    filename: impl AsRef<Path>,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(std::fs::read_to_string(filename)?
        .lines()
        .filter_map(|s| (!s.is_empty()).then(|| s.to_string()))
        .collect())
}

/// Spawns an empty vehicle grid.
pub fn spawn_empty_grid_c(
    spawner: &mut EntitySpawner,
    grids: &mut Components<VehicleGrid>,
    name: impl Into<String>,
) -> Ent {
    let name = name.into();
    debug!("Spawning empty grid with name {}", name);
    let grid = VehicleGrid::with_name(name, None);
    let id = spawner.spawn();
    grids.spawn(id, grid);
    id
}

/// Spawns an empty grid with the given name.
/// Exclusive version of [`super::spawn_empty_grid`].
pub fn spawn_empty_grid(world: &mut World, name: impl Into<String>) -> Ent {
    spawn_empty_grid_c(&mut world.spawner, &mut world.grids, name)
}

pub fn get_grid_physical_props(
    grid: &VehicleGrid,
    parts: &Components<Part>,
) -> BaryResult<(Mass, DVec2)> {
    let mut total_mass = Mass::ZERO;
    for part_id in &grid.parts {
        let part = parts.try_get(*part_id)?;
        total_mass += part.mass;
    }
    let mut com = DVec2::ZERO;
    for part_id in &grid.parts {
        let part = parts.try_get(*part_id)?;
        let center = part.region.center_isometry();
        let mass_portion = part.mass.to_kg_f64() / total_mass.to_kg_f64();
        com += center.translation * mass_portion;
    }
    Ok((total_mass, com))
}

/// Sets the state of a given thruster.
/// Does not modify the corresponding grid's acceleration,
/// but returns the ID of the grid that the thruster belongs to.
pub fn set_thruster_state(thruster_id: Ent, world: &mut World, new_state: bool) -> BaryResult<Ent> {
    let part = world.parts.try_get(thruster_id)?;
    let thruster = world.thrusters.try_get_mut(thruster_id)?;
    thruster.set_state(new_state, None);
    Ok(part.grid_id)
}

pub fn get_top_part_at(world: &World, loc: GridLocation) -> BaryResult<Ent> {
    let grid = world.grids.try_get(loc.grid_id)?;
    grid.get_parts_at(loc.coord)
        .map(|occ| occ.top())
        .flatten()
        .ok_or(BaryError::NoPartsAt(loc.coord))
}

pub fn update_grid_physical_props(
    grid: &mut VehicleGrid,
    parts: &mut Components<Part>,
) -> BaryResult<()> {
    grid.occupancy.clear();
    grid.update_bounds();

    for part_id in grid.parts.clone() {
        let part = parts.try_get_mut(part_id)?;
        grid.mark_occupied(part.region, part.layer, part_id);
    }

    let old_com = grid.center_of_mass;

    let (mass, com) = get_grid_physical_props(grid, parts)?;

    let delta = com - old_com;

    info!("Delta COM: {} - {} = {}", com, old_com, delta);

    grid.particle_location.translation += rotate_f64(delta, grid.particle_location.rotation);
    grid.parts_mass = mass;
    grid.center_of_mass = com;

    Ok(())
}

pub fn despawn_all_vehicles(world: &mut World) -> usize {
    let ret = world.grids.len();
    world.grids.clear();
    world.parts.clear();
    world.lights.clear();
    world.computers.clear();
    world.thrusters.clear();
    ret
}

/// Produces the entity ID corresponding to a grid's primary CPU,
/// which by convention is just the first element in the computer index.
pub fn get_primary_cpu_id(grid_id: Ent, grids: &Components<VehicleGrid>) -> BaryResult<Ent> {
    let grid = grids.try_get(grid_id)?;
    Ok(*grid.computers.first().ok_or(BaryError::NoPrimaryComputer)?)
}

pub fn get_thruster_levels(
    grid_id: Ent,
    grids: &Components<VehicleGrid>,
    thrusters: &Components<Thruster>,
) -> BaryResult<Vec<(Ent, bool)>> {
    let grid = grids.try_get(grid_id)?;
    let mut results = Vec::new();
    for thruster_id in &grid.thrusters {
        let thruster = thrusters.try_get(*thruster_id)?;
        results.push((*thruster_id, thruster.is_on()));
    }
    Ok(results)
}

/// Updates the `body_frame_forces` field of a particular
/// vehicle grid specified by the entity ID.
pub fn update_single_grid_acceleration(
    grid_id: Ent,
    grids: &mut Components<VehicleGrid>,
    thrusters: &Components<Thruster>,
    parts: &Components<Part>,
) -> BaryResult<()> {
    let grid = grids.try_get_mut(grid_id)?;
    grid.body_frame_forces = Isometry2d::ZERO;

    for thruster_id in &grid.thrusters {
        let thruster = thrusters.try_get(*thruster_id)?;

        if !thruster.is_on() {
            continue;
        }

        let part = parts.try_get(*thruster_id)?;

        let center_of_thrust = part.region.center_isometry().translation;
        let rotation = part.region.rot();
        let wrench = body_frame_wrench(
            thruster.thrust(),
            center_of_thrust,
            rotation,
            grid.center_of_mass,
        );
        grid.body_frame_forces.translation += wrench.translation;
        grid.body_frame_forces.rotation += wrench.rotation;
    }

    Ok(())
}

pub fn sys_update_grid_acceleration_c(
    dirty_set: BTreeSet<Ent>,
    grids: &mut Components<VehicleGrid>,
    thrusters: &Components<Thruster>,
    parts: &Components<Part>,
) {
    for grid_id in dirty_set {
        if let Err(e) = update_single_grid_acceleration(grid_id, grids, thrusters, parts) {
            error!("Failed to update grid accel: {e:?}");
        }
    }
}

pub fn update_grid_physical_props_by_id(
    grid_id: Ent,
    grids: &mut Components<VehicleGrid>,
    parts: &mut Components<Part>,
) -> BaryResult<()> {
    let grid = grids.try_get_mut(grid_id)?;
    update_grid_physical_props(grid, parts)
}

pub fn get_grid_physical_props_by_id(
    grid_id: Ent,
    grids: &Components<VehicleGrid>,
    parts: &Components<Part>,
) -> BaryResult<(Mass, DVec2)> {
    let grid = grids.try_get(grid_id)?;
    get_grid_physical_props(grid, parts)
}

pub fn set_grid_pose(world: &mut World, grid_id: Ent, iso: Isometry2d) -> BaryResult<()> {
    info!("Setting isometry of grid {} to {:?}", grid_id, iso);
    let grid = world.grids.try_get_mut(grid_id)?;
    grid.particle_location = iso;
    Ok(())
}

pub fn set_grid_vel(world: &mut World, grid_id: Ent, vel: Isometry2d) -> BaryResult<()> {
    info!("Setting velocity of grid {} to {:?}", grid_id, vel);
    let grid = world.grids.try_get_mut(grid_id)?;
    grid.velocity = vel;
    Ok(())
}

pub fn set_as_orbital_parent(world: &mut World, grid_id: Ent, planet_id: Ent) -> BaryResult<()> {
    info!(
        "Setting {} as the orbital parent of grid {}",
        planet_id, grid_id
    );

    let grid = world.grids.try_get_mut(grid_id)?;
    grid.orbital_parent = Some(planet_id);
    Ok(())
}

/// Inserts a part into an existing grid.
pub fn insert_part(
    grid_id: Ent,
    world: &mut World,
    instance: &PartInstance,
    update_props: bool,
) -> BaryResult<Ent> {
    let grid = world.grids.try_get_mut(grid_id)?;

    if !grid.can_insert_part(instance.region, instance.layer) {
        warn!("Can't insert part!");
        return Err(BaryError::GridSpaceOccupied);
    }

    let proto_id =
        get_proto_by_name(&world.prototypes, &instance.name).ok_or(BaryError::BadPartName)?;
    let proto = world.prototypes.try_get(proto_id)?;

    let part = Part {
        region: instance.region,
        layer: instance.layer(),
        mass: proto.mass,
        prototype: proto_id,
        grid_id,
        classification: proto.classification(),
    };

    let part_id = world.spawner.spawn();

    grid.parts.insert(part_id);
    world.parts.spawn(part_id, part);

    grid.mark_occupied(instance.region, instance.layer(), part_id);

    if let Some(inv) = &proto.inventory_data {
        let slots = inv
            .slots
            .iter()
            .map(|data| {
                InvSlot::new(
                    Volume::liters_f32(data.volume_liters),
                    data.filter.clone(),
                    data.is_fluid.unwrap_or(false),
                    (data.min.into(), data.max.into()),
                )
            })
            .collect();

        let inventory = Inventory::from_slots(slots);
        world.inventories.spawn(part_id, inventory);
    }
    if let Some(data) = &proto.machine_data {
        let machine = data.clone().into_machine();
        world.machines.spawn(part_id, machine);
    }
    if let Some(proto) = &proto.thruster_data {
        let thruster = Thruster::from_proto(proto);
        world.thrusters.spawn(part_id, thruster);
        grid.thrusters.insert(part_id);
    }
    if let Some(_data) = &proto.computer_data {
        let cpu = Computer::new(proto_id);
        world.computers.spawn(part_id, cpu);
        grid.computers.insert(part_id);
    }
    if let Some(data) = &proto.thruster_data {
        if data.is_rcs {
            let light_idx = world.lights.len();
            let light = Light::new(light_idx as u32);
            world.lights.spawn(part_id, light);
            grid.lights.insert(part_id);
        }
    }
    if let Some(debug) = &proto.debug_portal_data {
        let portal = DebugPortal::from_proto(*debug);
        world.debug_portals.spawn(part_id, portal);
    }
    if let Some(ex) = &proto.excavator_data {
        let excavator = Excavator::from_proto(ex);
        world.excavators.spawn(part_id, excavator);
    }

    if update_props {
        update_grid_physical_props_by_id(grid_id, &mut world.grids, &mut world.parts)?;
    }

    Ok(part_id)
}

/// Produces whatever prototype has the given name, if any.
pub fn get_proto_by_name(prototypes: &Components<PartPrototype>, name: &str) -> Option<Ent> {
    prototypes
        .iter()
        .find(|(_, proto)| proto.part_name() == name)
        .map(|e| *e.0)
}

/// Removes a part from its parent grid, updating any relevant quantities
/// about that grid. This does not perform an integrity check, and might
/// leave the parent grid in a state where it should be split up into
/// several grids.
///
/// TODO(bug) It also doesn't update the grid's acceleration, so if a thruster
/// is removed while it's firing that acceleration will remain until the grid's
/// acceleration is recalculated.
///
/// Returns the grid which was modified, if any.
///
/// TODO(testing) very, VERY important to test!
pub fn destroy_part_without_integrity_check(
    world: &mut World,
    part_id: Ent,
    update_props: bool,
) -> BaryResult<(PartInstance, Ent)> {
    let part = world.parts.try_get(part_id)?;
    let grid_id = part.grid_id;
    let proto = world.prototypes.try_get(part.prototype)?;
    let grid = world.grids.try_get_mut(grid_id)?;

    let instance = PartInstance::new(&proto.name, proto.layer, part.region);

    if grid.thrusters.contains(&part_id) {
        world.thrusters.despawn(part_id)?;
    }
    if grid.computers.contains(&part_id) {
        world.computers.despawn(part_id)?;
    }
    if grid.lights.contains(&part_id) {
        world.lights.despawn(part_id)?;
    }

    world
        .pipes
        .retain(|_id, pipe| pipe.src.part_id != part_id && pipe.dst.part_id != part_id);

    grid.remove_from_index(part_id);
    grid.parts_mass -= part.mass;

    world.parts.despawn(part_id)?;

    if grid.parts.is_empty() {
        world.grids.despawn(grid_id)?;
    }

    if update_props {
        update_grid_physical_props_by_id(grid_id, &mut world.grids, &mut world.parts)?;
    }

    Ok((instance, grid_id))
}

pub fn duplicate_part_to_new_grid(world: &mut World, part_id: Ent) -> BaryResult<Ent> {
    let part = world.parts.try_get(part_id)?;
    let grid = world.grids.try_get(part.grid_id)?;
    let new_name = format!("{}-debris", grid.name);
    let new_part_pose = grid.origin() * part.region.origin_isometry();
    let new_grid_vel = grid.velocity + Isometry2d::new(randvec(1.0, 3.0), rand(-0.1, 0.1));
    let proto = world.prototypes.try_get(part.prototype)?;
    let instance = PartInstance {
        name: proto.name.clone(),
        layer: proto.layer,
        region: GridRegion::new((0, 0), Rotation::East, proto.dims),
    };
    let new_grid_id = spawn_empty_grid(world, new_name);
    set_grid_pose(world, new_grid_id, new_part_pose)?;
    set_grid_vel(world, new_grid_id, new_grid_vel)?;
    let new_part_id = insert_part(new_grid_id, world, &instance, true)?;
    Ok(new_part_id)
}

pub fn detach_part_from_parent(world: &mut World, part_id: Ent) -> BaryResult<Ent> {
    let part = world.parts.try_get(part_id)?;
    let grid_id = part.grid_id;
    let new_part_id = duplicate_part_to_new_grid(world, part_id)?;
    destroy_part_without_integrity_check(world, part_id, true)?;
    split_grid_if_necessary(world, grid_id)?;
    Ok(new_part_id)
}

pub fn split_grid_if_necessary(world: &mut World, grid_id: Ent) -> BaryResult<Vec<Ent>> {
    let grid = world.grids.try_get(grid_id)?;
    let islands = grid.calculate_islands();
    if islands.is_empty() {
        // TODO debatable
        return Ok(vec![]);
    }
    if islands.len() == 1 {
        return Ok(vec![]);
    }

    let mut key_parts = BTreeMap::new();
    for island in &islands {
        let Some(key_part) = island.first() else {
            continue;
        };
        let part = world.parts.try_get(*key_part)?;
        let pose = grid.origin() * part.region.origin_isometry();
        debug!("Position of {}: {:?}", key_part, pose);
        key_parts.insert(*key_part, pose);
    }

    let mut ids = Vec::new();
    let rebuilt = rebuild_index_from_islands(grid, &islands, &world.parts)?;

    for (i, r) in rebuilt.into_iter().enumerate() {
        if i == 0 {
            world.grids.update(grid_id, r);
            ids.push(grid_id);
        } else {
            let id = world.spawner.spawn();
            for part_id in &r.parts {
                if let Ok(part) = world.parts.try_get_mut(*part_id) {
                    part.grid_id = id;
                }
            }
            world.grids.spawn(id, r);
            ids.push(id);
        }
    }

    for id in &ids {
        update_grid_physical_props_by_id(*id, &mut world.grids, &mut world.parts)?;
    }

    for island in &islands {
        let Some(key_part) = island.first() else {
            continue;
        };
        let Some(old_pose) = key_parts.get(key_part) else {
            continue;
        };
        let part = world.parts.try_get(*key_part)?;
        let grid = world.grids.try_get_mut(part.grid_id)?;
        let pose = grid.origin() * part.region.origin_isometry();
        let delta = pose.translation - old_pose.translation;
        info!("Position of {}: {:?}, was {:?}", key_part, pose, old_pose);
        grid.particle_location.translation -= delta;
    }

    Ok(ids)
}

pub fn rebuild_index_from_island(
    src: &VehicleGrid,
    island: &BTreeSet<Ent>,
    parts: &Components<Part>,
    name: String,
) -> BaryResult<VehicleGrid> {
    let mut dst = VehicleGrid::with_name(name, None);
    dst.particle_location = src.particle_location;
    dst.velocity = src.velocity;
    for part_id in island.iter().map(|i| *i) {
        let part = parts.try_get(part_id)?;
        dst.parts_mass += part.mass;
        dst.parts.insert(part_id);
        dst.mark_occupied(part.region, part.layer, part_id);
        if src.thrusters.contains(&part_id) {
            dst.thrusters.insert(part_id);
        }
        if src.computers.contains(&part_id) {
            dst.computers.insert(part_id);
        }
        if src.lights.contains(&part_id) {
            dst.lights.insert(part_id);
        }
    }

    Ok(dst)
}

pub fn rebuild_index_from_islands(
    src: &VehicleGrid,
    islands: &Vec<BTreeSet<Ent>>,
    parts: &Components<Part>,
) -> BaryResult<Vec<VehicleGrid>> {
    islands
        .iter()
        .map(|island| rebuild_index_from_island(src, island, parts, src.name.clone()))
        .collect()
}

pub fn set_player_piloting_grid(world: &mut World, player_id: Ent, grid_id: Ent) -> BaryResult<()> {
    let player = world.players.try_get_mut(player_id)?;
    player.state = PlayerState::PilotingGrid(grid_id, VehicleControl::NULLOPT);
    Ok(())
}

pub fn explode_grid_at(loc: GridLocation, world: &mut World) {
    let p = loc.coord.inner();
    let r = 2;
    for x in p.x - r..=p.x + r {
        for y in p.y - r..=p.y + r {
            let mut loc = loc;
            loc.coord.0 = (x, y).into();
            _ = destroy_top_part_at(world, loc);
        }
    }
}

pub fn toggle_tracking(world: &mut World, grid_id: Ent) -> BaryResult<bool> {
    let tracking = if world.tracking.has_entity(grid_id) {
        world.tracking.despawn(grid_id)?;
        info!("Removed tracking for grid {}", grid_id);
        false
    } else {
        world.tracking.spawn(grid_id, Tracker::default());
        info!("Enabled tracking for grid {}", grid_id);
        true
    };
    Ok(tracking)
}

pub fn grid_set_waypoint_to_asteroid_center(
    world: &mut World,
    grid_id: Ent,
    ast_id: Ent,
) -> BaryResult<()> {
    let ast = world.asteroids.try_get(ast_id)?;
    set_primary_computer_waypoint(grid_id, ast.iso, world)?;
    set_primary_computer_state_c(grid_id, true, &world.grids, &mut world.computers)?;
    Ok(())
}

/// Sets the waypoint field of the primary computer,
/// if the provided grid has one. If it does, the ID of the primary
/// computer will be returned.
/// TODO(testing) test this.
pub fn set_primary_computer_waypoint(
    grid_id: Ent,
    waypoint: impl Into<Isometry2d>,
    world: &mut World,
) -> BaryResult<Ent> {
    super::set_primary_computer_waypoint_c(grid_id, waypoint, &world.grids, &mut world.computers)
}

/// Sets the waypoint field of the primary computer,
/// if the provided grid has one. If it does, the ID of the primary
/// computer will be returned.
/// TODO(testing) test this.
pub fn set_primary_computer_waypoint_c(
    grid_id: Ent,
    waypoint: impl Into<Isometry2d>,
    grids: &Components<VehicleGrid>,
    computers: &mut Components<Computer>,
) -> BaryResult<Ent> {
    let primary_cpu_id = get_primary_cpu_id(grid_id, grids)?;
    let computer = computers.try_get_mut(primary_cpu_id)?;
    let command = TimedInstruction::perp(Instruction::HoldPosition(waypoint.into()));
    computer.clear_instruction_queue();
    computer.enqueue_instruction(command);
    Ok(primary_cpu_id)
}

pub fn set_cpu_state(
    computers: &mut Components<Computer>,
    cpu_id: Ent,
    new_state: bool,
) -> BaryResult<()> {
    let computer = computers.try_get_mut(cpu_id)?;
    computer.set_state(new_state);
    Ok(())
}

pub fn set_grid_as_drifting(world: &mut World, cpu_id: Ent) -> BaryResult<()> {
    let part = world.parts.try_get(cpu_id)?;
    set_cpu_state(&mut world.computers, cpu_id, false)?;
    set_all_thrusters(part.grid_id, false, world)?;
    Ok(())
}

pub fn set_cpu_instruction(world: &mut World, cpu_id: Ent, ti: TimedInstruction) -> BaryResult<()> {
    let cpu = world.computers.try_get_mut(cpu_id)?;
    cpu.clear_instruction_queue();
    cpu.enqueue_instruction(ti);
    Ok(())
}

pub fn set_grid_instruction(
    world: &mut World,
    grid_id: Ent,
    ti: TimedInstruction,
) -> BaryResult<()> {
    let cpu_id = get_primary_cpu_id(grid_id, &world.grids)?;
    set_cpu_instruction(world, cpu_id, ti)
}

/// Turns the primary computer of the given grid on or off,
/// returning the entity ID of the computer if it was found.
/// TODO(testing) test this.
pub fn set_primary_computer_state_c(
    grid_id: Ent,
    new_state: bool,
    grids: &Components<VehicleGrid>,
    computers: &mut Components<Computer>,
) -> BaryResult<Ent> {
    let primary_cpu_id = get_primary_cpu_id(grid_id, grids)?;
    set_cpu_state(computers, primary_cpu_id, new_state)?;
    Ok(primary_cpu_id)
}

/// Turns the primary computer of the given grid on or off,
/// returning the entity ID of the computer if it was found.
pub fn set_primary_computer_state(
    grid_id: Ent,
    new_state: bool,
    world: &mut World,
) -> BaryResult<Ent> {
    set_primary_computer_state_c(grid_id, new_state, &world.grids, &mut world.computers)
}

pub fn destroy_top_part_at(
    world: &mut World,
    loc: GridLocation,
) -> BaryResult<(PartInstance, Ent, Vec<Ent>)> {
    let top_part = get_top_part_at(world, loc)?;
    destroy_part(world, top_part)
}

pub fn destroy_part_at_layer(
    world: &mut World,
    loc: GridLocation,
    layer: PartLayer,
) -> BaryResult<(PartInstance, Ent, Vec<Ent>)> {
    let part_id = get_part_at(world, loc, layer)?;
    destroy_part(world, part_id)
}

pub fn destroy_part(world: &mut World, part_id: Ent) -> BaryResult<(PartInstance, Ent, Vec<Ent>)> {
    let (instance, grid_id) = destroy_part_without_integrity_check(world, part_id, true)?;
    let grids = split_grid_if_necessary(world, grid_id)?;
    Ok((instance, grid_id, grids))
}

pub fn get_part_at(world: &World, loc: GridLocation, layer: PartLayer) -> BaryResult<Ent> {
    let grid = world.grids.try_get(loc.grid_id)?;
    let occ = grid
        .get_parts_at(loc.coord)
        .ok_or(BaryError::NoPartsAt(loc.coord))?;
    occ.at_layer(layer).ok_or(BaryError::NoPartsInLayer(layer))
}

pub fn detach_top_part_at(world: &mut World, grid_id: Ent, coord: PartCoord) -> BaryResult<Ent> {
    warn!("Detaching top part at {} in grid {}", coord, grid_id);

    let grid = world.grids.try_get(grid_id)?;
    let top_part = grid
        .get_parts_at(coord)
        .map(|occ| occ.top())
        .flatten()
        .ok_or(BaryError::NoPartsAt(coord))?;

    debug!("Top part is {}", top_part);

    detach_part_from_parent(world, top_part)?;

    Ok(top_part)
}

pub fn get_grid_origin(grids: &Components<VehicleGrid>, grid_id: Ent) -> Option<Isometry2d> {
    let grid = grids.try_get(grid_id).ok()?;
    Some(grid.origin())
}

pub fn grid_pose(grids: &Components<VehicleGrid>, grid_id: Ent) -> Option<Isometry2d> {
    let grid = grids.try_get(grid_id).ok()?;
    Some(grid.particle_location)
}

pub fn grid_vel(grids: &Components<VehicleGrid>, grid_id: Ent) -> Option<Isometry2d> {
    let grid = grids.try_get(grid_id).ok()?;
    Some(grid.velocity)
}

pub fn part_pose(
    grids: &Components<VehicleGrid>,
    parts: &Components<Part>,
    part_id: Ent,
) -> BaryResult<Isometry2d> {
    let part = parts.try_get(part_id)?;
    let grid = grids.try_get(part.grid_id)?;
    Ok(grid.origin() * part.region.center_isometry())
}

pub fn gridloc_pose(grids: &Components<VehicleGrid>, loc: GridLocation) -> BaryResult<Isometry2d> {
    let grid = grids.try_get(loc.grid_id)?;
    Ok(grid.origin().offset(loc.coord.to_meters()))
}

pub fn despawn_grid(world: &mut World, grid_id: Ent) -> BaryResult<()> {
    let grid = world.grids.despawn(grid_id)?;
    for id in grid.parts {
        world.parts.despawn(id)?;
    }
    for id in grid.thrusters {
        world.thrusters.despawn(id)?;
    }
    for id in grid.computers {
        world.computers.despawn(id)?;
    }
    for id in grid.lights {
        world.lights.despawn(id)?;
    }
    for id in grid.pipes {
        world.pipes.despawn(id)?;
    }
    Ok(())
}

pub fn sum_part_masses(
    grids: &Components<VehicleGrid>,
    parts: &Components<Part>,
    prototypes: &Components<PartPrototype>,
    grid_id: Ent,
) -> BaryResult<(Mass, Vec2)> {
    let grid = grids.try_get(grid_id)?;
    let mut sum = Mass::ZERO;
    let mut com = DVec2::ZERO;
    for part_id in &grid.parts {
        let part = parts.try_get(*part_id)?;
        let proto = prototypes.try_get(part.prototype)?;
        sum += proto.mass;
        let center = part.region.center_isometry().translation;
        com += center;
    }
    if !sum.is_zero() {
        com /= sum.to_kg_f64();
    }
    Ok((sum, com.as_vec2()))
}

pub fn sum_part_masses_w(world: &World, grid_id: Ent) -> BaryResult<(Mass, Vec2)> {
    sum_part_masses(&world.grids, &world.parts, &world.prototypes, grid_id)
}

pub fn get_blueprint_by_id<'a>(
    blueprints: &'a Components<NamedBlueprint>,
    id: &BlueprintId,
) -> Option<&'a Blueprint> {
    let result = blueprints.values().find(|bp| &bp.id == id);

    if result.is_none() {
        error!("Failed to get blueprint with ID {:?}", id);
    }

    result.map(|e| &e.blueprint)
}

pub fn spawn_player(world: &mut World, username: String, iso: Isometry2d) -> BaryResult<Ent> {
    let already_exists = world.players.values().any(|player| player.name == username);

    if already_exists {
        return Err(BaryError::PlayerAlreadyExists(username));
    }

    let player = Player {
        name: username,
        cursor_world_position: None,
        state: PlayerState::Flying(iso),
    };
    let id = world.spawner.spawn();
    world.players.spawn(id, player);
    Ok(id)
}

pub fn set_player_position(world: &mut World, player_id: Ent, iso: Isometry2d) -> BaryResult<()> {
    let player = world.players.try_get_mut(player_id)?;
    player.set_position(iso);
    Ok(())
}

pub fn set_player_cursor_position(
    world: &mut World,
    player_id: Ent,
    pos: Option<Vec2>,
) -> BaryResult<()> {
    let player = world.players.try_get_mut(player_id)?;
    player.cursor_world_position = pos;
    Ok(())
}

pub fn player_exit_grid(world: &mut World, player_id: Ent) -> BaryResult<()> {
    let player = world.players.try_get_mut(player_id)?;
    let grid_id = player
        .driving_grid()
        .ok_or(BaryError::PlayerNotDriving(player_id))?;
    let grid = world.grids.try_get(grid_id)?;
    player.set_position(grid.particle_location);
    Ok(())
}

pub fn get_part_at_layer(
    grid: &VehicleGrid,
    coord: PartCoord,
    layer: PartLayer,
) -> BaryResult<Ent> {
    grid.get_parts_at(coord)
        .ok_or(BaryError::NoPartsAt(coord))?
        .at_layer(PartLayer::Internal)
        .ok_or(BaryError::NoPartsInLayer(layer))
}

pub fn calculate_pipe_joint_c(
    loc: GridLocation,
    grids: &Components<VehicleGrid>,
    parts: &Components<Part>,
    inventories: &Components<Inventory>,
) -> BaryResult<PipeJoint> {
    let grid = grids.try_get(loc.grid_id)?;
    let part_id = get_part_at_layer(grid, loc.coord, PartLayer::Internal)?;
    let part_a = parts.try_get(part_id)?;
    let local = part_a.region.to_local(loc.coord);
    let src_inv = inventories
        .try_get(part_id)
        .map_err(|_| BaryError::PartHasNoInv(part_id))?;
    let slot = src_inv
        .get_slot_at(local)
        .ok_or(BaryError::NoInvAt(loc.coord))?;

    Ok(PipeJoint {
        part_id,
        offset: local,
        slot,
    })
}

pub fn insert_pipe_at_c(
    grid_id: Ent,
    src: PartCoord,
    dst: PartCoord,
    spawner: &mut EntitySpawner,
    grids: &mut Components<VehicleGrid>,
    parts: &Components<Part>,
    inventories: &Components<Inventory>,
    pipes: &mut Components<Pipe>,
) -> BaryResult<Ent> {
    if src == dst {
        return Err(BaryError::ZeroPipeExtent);
    }

    let src_loc = GridLocation::new(grid_id, src);
    let dst_loc = GridLocation::new(grid_id, dst);

    let src_joint = calculate_pipe_joint_c(src_loc, grids, parts, inventories)?;
    let dst_joint = calculate_pipe_joint_c(dst_loc, grids, parts, inventories)?;

    let grid = grids.try_get_mut(grid_id)?;

    if src_joint.part_id == dst_joint.part_id && src_joint.slot == dst_joint.slot {
        return Err(BaryError::SameInvSlot(src_joint.part_id, src_joint.slot));
    }

    let pipe = Pipe {
        src: src_joint,
        dst: dst_joint,
        status: MachineStatus::Off,
    };

    let id = spawner.spawn();
    pipes.spawn(id, pipe);

    grid.pipes.insert(id);

    Ok(id)
}

pub fn insert_pipe(
    grid_id: Ent,
    src: PartCoord,
    dst: PartCoord,
    world: &mut World,
) -> BaryResult<Ent> {
    insert_pipe_at_c(
        grid_id,
        src,
        dst,
        &mut world.spawner,
        &mut world.grids,
        &world.parts,
        &world.inventories,
        &mut world.pipes,
    )
}

pub fn update_world(world: &mut World) -> DebugTimers {
    world.ticks += 1;

    let mut timers = DebugTimers::default();
    timers.ticks += 1;

    {
        let _timer = timers.scope("grid_motion");

        sys_update_ring_particles(&mut world.particles, world.ticks);
        let dirty_set = sys_update_thrusters(
            &mut world.thrusters,
            &world.grids,
            &world.parts,
            &world.computers,
        );

        world.grid_acceleration_updates += dirty_set.len() as u64;
        sys_update_grid_acceleration_c(dirty_set, &mut world.grids, &world.thrusters, &world.parts);
        sys_update_computers(&mut world.computers, &world.parts, &world.grids);
        sys_propagate_planets(&mut world.planets);
        sys_propagate_grid_rigid_bodies(&mut world.grids);
        sys_propagate_n_body_accel(&mut world.grids, &world.planets);
    }

    {
        let _timer = timers.scope("update_trackers");

        sys_update_trackers(&mut world.tracking, &world.grids, world.ticks);
    }

    {
        let _timer = timers.scope("update_pipes");

        sys_update_pipes(&mut world.inventories, &mut world.pipes);
    }

    {
        let _timer = timers.scope("fill_inventories");

        sys_fill_inventories_attached_to_debug_sources(world);
    }

    {
        let _timer = timers.scope("update_machines");

        sys_update_machines(world);
    }

    {
        let _timer = timers.scope("terrain_mining");

        sys_mine_tiles(world);
    }

    {
        let _timer = timers.scope("radio_transmissions");

        sys_update_transmissions(world);
    }

    timers
}

pub fn get_slot_mut_c<'a>(
    loc: GridLocation,
    grids: &Components<VehicleGrid>,
    parts: &Components<Part>,
    inventories: &'a mut Components<Inventory>,
) -> BaryResult<&'a mut InvSlot> {
    let (part_id, slot_id) = inventory_at_c(loc, grids, parts, inventories)?;
    let inv = inventories.try_get_mut(part_id)?;
    inv.get_slot_mut(slot_id)
        .ok_or(BaryError::NoInvSlot(slot_id))
}

/// Fills inventories which have debug sources attached.
fn sys_fill_inventories_attached_to_debug_sources(world: &mut World) {
    for (part_id, portal) in world.debug_portals.iter() {
        let part = ok_or_continue!(world.parts.try_get(*part_id));
        let loc = GridLocation::new(part.grid_id, part.region.origin());
        let slot = ok_or_continue!(get_slot_mut_c(
            loc,
            &world.grids,
            &world.parts,
            &mut world.inventories
        ));

        match portal.state {
            PortalState::Source(item) => {
                if let Some(item) = item {
                    slot.fill_with(item);
                }
            }
            PortalState::Sink => {
                slot.empty();
            }
        }
    }
}

/// Performs inventory transfers according to the pipes that exist
/// in the world.
fn sys_update_pipes(inventories: &mut Components<Inventory>, pipes: &mut Components<Pipe>) {
    for pipe in pipes.values_mut() {
        let inv_a = ok_or_continue!(inventories.try_get(pipe.src.part_id));
        let inv_b = ok_or_continue!(inventories.try_get(pipe.dst.part_id));

        let mut src = some_or_continue!(inv_a.get_slot(pipe.src.slot)).clone();
        let mut dst = some_or_continue!(inv_b.get_slot(pipe.dst.slot)).clone();

        if src.is_empty() {
            pipe.status = MachineStatus::Starved;
            continue;
        }

        let mass = {
            let mul = 150;
            let m = src.mass() / mul as u64;
            if m.is_zero() { Mass::grams(1) } else { m }
        };

        pipe.status = atomic_transfer(&mut src, &mut dst, mass);

        _ = set_inventory_slot(inventories, src, pipe.src.part_id, pipe.src.slot);
        _ = set_inventory_slot(inventories, dst, pipe.dst.part_id, pipe.dst.slot);
    }
}

pub fn step_process(machine: &mut Machine, id: Ent, inv: &mut Components<Inventory>) {
    if machine.recipe().is_none() {
        machine.status = MachineStatus::NoRecipe;
        return;
    }

    if !machine.enabled {
        machine.status = MachineStatus::Off;
        return;
    }

    if machine.steps == 0 {
        if let Ok(inv) = inv.try_get_mut(id) {
            if machine.take_inputs_if_possible(inv) {
                machine.steps += 1;
                machine.status = MachineStatus::Running;
                return;
            } else {
                machine.status = MachineStatus::Starved;
                return;
            }
        } else {
            machine.status = MachineStatus::Starved;
            return;
        }
    }

    if machine.steps > 0 && machine.steps < machine.required_steps {
        machine.status = MachineStatus::Running;
        machine.steps += 1;
    } else if machine.steps >= machine.required_steps {
        if let Ok(inv) = inv.try_get_mut(id) {
            if machine.store_outputs_if_possible(inv) {
                machine.steps = 0;
                machine.products_finished += 1;
                machine.status = MachineStatus::Running;
            } else {
                machine.status = MachineStatus::NoRoom;
            }
        } else {
            machine.status = MachineStatus::NoRoom;
        }
    } else {
        machine.status = MachineStatus::Off;
    }
}

pub fn set_inventory_slot(
    inventories: &mut Components<Inventory>,
    slot: InvSlot,
    inv_id: Ent,
    slot_id: usize,
) -> BaryResult<()> {
    let inv = inventories.try_get_mut(inv_id)?;
    let old_slot = inv
        .get_slot_mut(slot_id)
        .ok_or(BaryError::NoInvSlot(slot_id))?;
    *old_slot = slot;
    Ok(())
}

/// Steps running machines forward by one tick, and modifies their
/// corresponding inventory if necessary.
fn sys_update_machines(world: &mut World) {
    for (part_id, machine) in world.machines.iter_mut() {
        step_process(machine, *part_id, &mut world.inventories);
    }
}

/// Iterates over all active excavators and removes tiles accordingly
fn sys_mine_tiles(world: &mut World) {
    let mut to_remove: BTreeMap<(Ent, Ent), BTreeSet<GlobalTileIndex>> = BTreeMap::new();

    for ex in world.excavators.values_mut() {
        ex.tick();
    }

    for (ex_id, ex) in world.excavators.iter() {
        if !ex.mined_this_tick() {
            continue;
        }

        let Ok(Some((ast_id, tiles))) = get_excavator_tiles(*ex_id, ex, world) else {
            continue;
        };

        to_remove
            .entry((ast_id, *ex_id))
            .and_modify(|e| {
                for t in &tiles {
                    e.insert(*t);
                }
            })
            .or_insert(BTreeSet::from_iter(tiles));
    }

    for ((ast_id, ex_id), tiles) in to_remove {
        for t in tiles {
            if chance(0.5) {
                continue;
            }

            let mat = mine_terrain_tile(world, ast_id, t).ok();
            let inv = world.inventories.try_get_mut(ex_id).ok();
            if let Some((mat, inv)) = mat.zip(inv) {
                let item = match mat {
                    TerrainMaterial::Rock => continue,
                    TerrainMaterial::Dirt => continue,
                    TerrainMaterial::Ice => Item::Ice,
                    TerrainMaterial::Silicon => Item::Silicon,
                    TerrainMaterial::Iron => Item::Iron,
                };
                inv.store(item, 100);
            }
        }
    }
}

pub fn get_slot_c<'a>(
    loc: GridLocation,
    grids: &Components<VehicleGrid>,
    parts: &Components<Part>,
    inventories: &'a Components<Inventory>,
) -> BaryResult<&'a InvSlot> {
    let (part_id, slot_id) = inventory_at_c(loc, grids, parts, inventories)?;
    let inv = inventories.try_get(part_id)?;
    inv.get_slot(slot_id).ok_or(BaryError::NoInvSlot(slot_id))
}

pub fn inventory_at_c(
    loc: GridLocation,
    grids: &Components<VehicleGrid>,
    parts: &Components<Part>,
    inventories: &Components<Inventory>,
) -> BaryResult<(Ent, usize)> {
    let grid = grids.try_get(loc.grid_id)?;
    let occ = grid
        .get_parts_at(loc.coord)
        .ok_or(BaryError::NoPartsAt(loc.coord))?;
    let part_id = occ
        .at_layer(PartLayer::Internal)
        .ok_or(BaryError::NoPartsInLayer(PartLayer::Internal))?;
    let inv = inventories.try_get(part_id)?;
    let part = parts.try_get(part_id)?;
    let local = part.region.to_local(loc.coord);
    let idx = inv
        .get_slot_at(local)
        .ok_or(BaryError::NoInvAt(loc.coord))?;
    Ok((part_id, idx))
}

/// Returns the ID of the first grid in the components list with
/// the given name.
///
/// Buyer beware: grid names are not unique! This
/// only promises to return any grid with the given name, if one exists.
pub fn get_grid_by_name(grids: &Components<VehicleGrid>, name: &str) -> Option<Ent> {
    grids
        .iter()
        .find_map(|(id, grid)| (grid.name == name).then(|| *id))
}

/// Spawns a new grid according to a named blueprint.
pub fn spawn_grid_with_bp_id(
    world: &mut World,
    bp_id: &BlueprintId,
    grid_name: &str,
) -> BaryResult<Ent> {
    let bp = get_blueprint_by_id(&world.blueprints, bp_id)
        .ok_or(BaryError::BadBlueprint)?
        .clone();
    spawn_grid_from_blueprint(world, grid_name, Some(bp_id), &bp)
}

/// Spawns a grid according to the given blueprint.
pub fn spawn_grid_from_blueprint(
    world: &mut World,
    grid_name: impl Into<String>,
    bp_id: Option<&BlueprintId>,
    bp: &Blueprint,
) -> BaryResult<Ent> {
    let grid_name = grid_name.into();
    info!(
        "Spawning grid with name \"{}\" from blueprint {:?}",
        grid_name, bp_id
    );
    let grid = VehicleGrid::with_name(grid_name, bp_id.cloned());
    let grid_id = world.spawner.spawn();
    world.grids.spawn(grid_id, grid.clone());
    for (_id, instance) in bp.parts() {
        insert_part(grid_id, world, instance, false)?;
    }

    for (_id, pipe) in bp.pipes() {
        if let Err(e) = insert_pipe_at_c(
            grid_id,
            pipe.start,
            pipe.end,
            &mut world.spawner,
            &mut world.grids,
            &mut world.parts,
            &mut world.inventories,
            &mut world.pipes,
        ) {
            error!("Failed to insert pipe: {:?}", e);
        }
    }

    let grid = world.grids.try_get_mut(grid_id)?;

    update_grid_physical_props(grid, &mut world.parts)?;

    Ok(grid_id)
}

pub fn spawn_grid_with_random_name(
    world: &mut World,
    bp_id: impl Into<BlueprintId>,
) -> BaryResult<Ent> {
    let name = get_random_ship_name(&world.ship_names);
    let bp_id = bp_id.into();
    spawn_grid_with_bp_id(world, &bp_id, &name)
}

pub fn can_insert_part_c(
    grid_id: Ent,
    pl: GridRegion,
    layer: PartLayer,
    grids: &Components<VehicleGrid>,
) -> BaryResult<bool> {
    let grid = grids.try_get(grid_id)?;
    Ok(grid.can_insert_part(pl, layer))
}

pub fn get_closest_grid(
    grids: &Components<VehicleGrid>,
    test_pos: DVec2,
    dist_limit: impl Into<Option<f64>>,
) -> Option<(Ent, DVec2)> {
    let mut best: Option<(Ent, DVec2, f64)> = None;
    let dist_limit = dist_limit.into().unwrap_or(f64::INFINITY);
    for (e, grid) in grids.iter() {
        let in_frame = express_in_frame(grid.centroid_isometry(), test_pos);
        let dist = in_frame.length_squared();
        if dist > dist_limit {
            continue;
        }
        if let Some(best) = &mut best {
            if dist < best.2 {
                best.0 = *e;
                best.1 = in_frame;
                best.2 = dist;
            }
        } else {
            best = Some((*e, in_frame, dist));
        }
    }
    best.map(|x| (x.0, x.1))
}

pub fn get_blueprint_c(
    grids: &Components<VehicleGrid>,
    parts: &Components<Part>,
    pipes: &Components<Pipe>,
    prototypes: &Components<PartPrototype>,
    grid_id: Ent,
) -> BaryResult<Blueprint> {
    let grid = grids.try_get(grid_id)?;
    let mut bp = Blueprint::new();
    for part_id in &grid.parts {
        let part = parts.try_get(*part_id)?;
        let proto = prototypes.try_get(part.prototype)?;
        bp.add_part(proto.name.to_string(), part.region, part.layer);
    }
    for pipe_id in &grid.pipes {
        let pipe = pipes.try_get(*pipe_id)?;
        let src_part = parts.try_get(pipe.src.part_id)?;
        let dst_part = parts.try_get(pipe.dst.part_id)?;
        let start = src_part.region.to_global(pipe.src.offset);
        let end = dst_part.region.to_global(pipe.dst.offset);

        assert_eq!(src_part.region.to_local(start), pipe.src.offset);
        assert_eq!(dst_part.region.to_local(end), pipe.dst.offset);

        let geo = PipeGeometry {
            start,
            end,
            x_first: false,
        };
        bp.add_pipe(geo);
    }
    Ok(bp)
}

pub fn get_sum_linear_forces(
    grid_id: Ent,
    grids: &Components<VehicleGrid>,
    parts: &Components<Part>,
    thrusters: &Components<Thruster>,
) -> BaryResult<Vec2> {
    let grid = grids.try_get(grid_id)?;
    let mut sum = Vec2::ZERO;
    for part_id in &grid.thrusters {
        let thruster = thrusters.try_get(*part_id)?;
        let part = parts.try_get(*part_id)?;
        let thrust = rotate(Vec2::X, part.region.rot().to_angle() as f32) * thruster.thrust();
        sum += thrust;
    }
    Ok(sum)
}

pub fn merge_grids(
    world: &mut World,
    parent_id: Ent,
    child_id: Ent,
    offset: PartCoord,
    rotation: Rotation,
) -> BaryResult<()> {
    // TODO determine if the merge would violate part intersection constraints

    let child = world.grids.despawn(child_id)?;

    let parent = world.grids.try_get_mut(parent_id)?;

    for part_id in &child.parts {
        parent.parts.insert(*part_id);

        let part = world.parts.try_get_mut(*part_id)?;
        part.grid_id = parent_id;
        part.region.rotate(rotation);
        part.region.shift(offset);
    }

    parent.computers.extend(child.computers.into_iter());
    parent.thrusters.extend(child.thrusters.into_iter());
    parent.lights.extend(child.lights.into_iter());
    parent.pipes.extend(child.pipes.into_iter());

    update_grid_physical_props(parent, &mut world.parts)?;

    for cpu_id in &parent.computers {
        let computer = world.computers.try_get_mut(*cpu_id)?;
        computer.clear_instruction_queue();
        computer.enqueue_instruction(Instruction::Drift.into());
    }

    Ok(())
}

pub fn rename_grid(world: &mut World, grid_id: Ent, name: impl Into<String>) -> BaryResult<String> {
    let mut name = name.into();
    let grid = world.grids.try_get_mut(grid_id)?;
    std::mem::swap(&mut grid.name, &mut name);
    Ok(name)
}

pub fn set_grid_blueprint_id(
    world: &mut World,
    grid_id: Ent,
    bp_id: Option<BlueprintId>,
) -> BaryResult<()> {
    let grid = world.grids.try_get_mut(grid_id)?;
    grid.blueprint = bp_id;
    Ok(())
}

pub fn set_machine_state(world: &mut World, machine_id: Ent, state: bool) -> BaryResult<()> {
    let machine = world.machines.try_get_mut(machine_id)?;
    machine.enabled = state;
    Ok(())
}

fn sys_update_transmissions(world: &mut World) {
    let mut new_transmissions = Vec::new();
    world.transmissions.retain(|tr_id, tr| {
        let Some((inner, outer)) = tr.update(NOMINAL_DT) else {
            info!("[{}] Dissipated.", tr_id);
            return false;
        };

        for (grid_id, grid) in world.grids.iter_mut() {
            if tr.source() == Some(*grid_id) {
                continue;
            }
            let p = grid.particle_location.tr();
            let d = tr.origin().distance(p);
            if inner <= d && d < outer {
                info!(
                    "[{}] {} received with power {}",
                    tr_id,
                    grid.name,
                    tr.received_power()
                );
                if !tr.is_response() {
                    let new_tr = RadioTransmission::new(p)
                        .with_range(1500.0)
                        .with_source(*grid_id)
                        .with_contents(TransmissionContents::Response);
                    new_transmissions.push(new_tr);
                }
            }
        }

        true
    });

    for (grid_id, grid) in world.grids.iter() {
        if chance(0.001) {
            let pos = grid.particle_location.tr();
            let tr = RadioTransmission::new(pos)
                .with_range(10000.0)
                .with_source(*grid_id)
                .with_contents(TransmissionContents::Message);
            new_transmissions.push(tr);
        }
    }

    for tr in new_transmissions {
        let id = world.spawner.spawn();
        world.transmissions.spawn(id, tr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WorldBuilder;

    #[test]
    fn renaming_grid() {
        let mut world = WorldBuilder::new()
            .test_assets()
            .blueprint(("pollux", 2))
            .spawn(("pollux", 2), "Bob", Isometry2d::ZERO)
            .build();

        let parent_id = get_grid_by_name(&world.grids, "Bob").expect("Expected Bob's ID");

        assert_eq!(parent_id, Ent(35));

        let name = rename_grid(&mut world, parent_id, "Kathmandu");

        assert_eq!(name, Ok("Bob".to_string()));
    }

    #[test]
    fn merging_pollux_and_remora() {
        let mut world = WorldBuilder::new()
            .test_assets()
            .blueprint(("pollux", 2))
            .blueprint(("remora", 0))
            .spawn(("pollux", 2), "Bob", Isometry2d::ZERO)
            .spawn(("remora", 0), "Holly", (200.0, 200.0, 0.0))
            .build();

        let parent_id = get_grid_by_name(&world.grids, "Bob").expect("Expected Bob's ID");
        let child_id = get_grid_by_name(&world.grids, "Holly").expect("Expected Holly's ID");

        let offset = PartCoord::new((0, 0));
        let rotation = Rotation::East;

        let res = merge_grids(&mut world, parent_id, child_id, offset, rotation);

        assert_world_is_consistent(&world);

        assert_eq!(res, Ok(()));
    }

    #[test]
    fn splitting_ships_breaks_pipe_connections() {
        let mut world = WorldBuilder::new()
            .test_assets()
            .blueprint(("pollux", 2))
            .spawn(("pollux", 2), "Bob", Isometry2d::ZERO)
            .build();

        world.pipes.clear();

        let grid_id = get_grid_by_name(&world.grids, "Bob").expect("Expected Bob's ID");

        let next_id = world.spawner.next();

        let pipe = insert_pipe(
            grid_id,
            PartCoord::new((14, 8)),
            PartCoord::new((29, 8)),
            &mut world,
        );

        assert_eq!(pipe, Ok(next_id));
        assert_eq!(world.pipes.len(), 1);

        assert_world_is_consistent(&world);

        let grid = world.grids.try_get(grid_id).expect("Expected a grid");

        let mut parts_to_destroy = BTreeSet::new();
        for y in 0..100 {
            let c = PartCoord::new((20, y));
            let occ = grid.get_parts_at(c).unwrap_or(&PartOccupancy::EMPTY);
            for (_layer, id) in occ.iter() {
                parts_to_destroy.insert(id);
            }
        }

        assert_eq!(parts_to_destroy.len(), 11);

        for part_id in parts_to_destroy {
            let part = world.parts.try_get(part_id).unwrap();
            let proto = world.prototypes.try_get(part.prototype).unwrap();
            println!("Destroying {}: {:?}", part_id, proto.name);
            let result = destroy_part(&mut world, part_id);
            assert!(result.is_ok());
        }

        assert_eq!(world.grids.len(), 2);

        assert_world_is_consistent(&world);
    }

    #[test]
    fn vehicle_pathing_is_deterministic() {
        let mut w1 = WorldBuilder::new()
            .test_assets()
            .blueprint("remora")
            .spawn("remora", "", Isometry2d::ZERO)
            .waypoint("remora", (100.0, 400.0, 0.1))
            .build();

        let mut w2 = WorldBuilder::new()
            .test_assets()
            .blueprint("remora")
            .spawn("remora", "", Isometry2d::ZERO)
            .waypoint("remora", (100.0, 400.0, 0.1))
            .build();

        for _ in 0..10000 {
            update_world(&mut w1);
            update_world(&mut w2);

            assert_eq!(w1.ticks, w2.ticks);

            for (g1, g2) in w1.grids.values().zip(w2.grids.values()) {
                assert_eq!(
                    g1.particle_location, g2.particle_location,
                    "Failed on tick {}",
                    w1.ticks
                );
            }
        }
    }

    #[test]
    fn world_persistence() {
        let save_path = "../../saves/test_world";

        if std::fs::exists(save_path).unwrap() {
            std::fs::remove_dir_all(save_path).unwrap();
        }

        let mut world = WorldBuilder::new()
            .test_assets()
            .blueprint("pollux")
            .blueprint("remora")
            .blueprint("bellerophon")
            .blueprint("foundation")
            .spawn("pollux", "billy", (120.0, 43.0, 0.4))
            .spawn("remora", "sally", (-30.0, 21.0, -0.1))
            .spawn("bellerophon", "eisenhower", (50.0, 109.0, 1.4))
            .waypoint("pollux", (50.0, 300.0, 0.2))
            .build();

        for _ in 0..1000 {
            update_world(&mut world);
        }

        let r = save_world(save_path, &world, false);
        assert_eq!(r, Ok(()));

        let r = save_world(save_path, &world, false);
        assert_eq!(r, Err(BaryError::SaveAlreadyExists));

        let r = save_world(save_path, &world, true);
        assert_eq!(r, Ok(()));

        let world = load_world(save_path).expect("Expected successful load");

        assert_eq!(world.grids.len(), 3);
        assert_eq!(world.parts.len(), 374);
    }

    #[test]
    fn build_ship_on_another_ship_then_navigate() {
        let mut world = WorldBuilder::new()
            .test_assets()
            .blueprint("remora")
            .spawn("remora", "ursula", Isometry2d::ZERO)
            .build();

        let grid_id = get_grid_by_name(&world.grids, "ursula").unwrap();
        let bp = get_blueprint_by_id(&world.blueprints, &"remora".into())
            .unwrap()
            .clone();

        for (_id, instance) in bp.parts() {
            let mut instance = instance.clone();
            instance.region.shift((20, 20).into());
            assert!(insert_part(grid_id, &mut world, &instance, true).is_ok());
        }

        let next_id = world.spawner.next();

        assert_eq!(world.grids.len(), 1);

        let grids = split_grid_if_necessary(&mut world, grid_id).unwrap();

        assert_eq!(world.grids.len(), 2);

        assert_eq!(grids, vec![grid_id, next_id]);

        let props_a = get_grid_physical_props_by_id(grid_id, &world.grids, &world.parts).unwrap();
        let props_b = get_grid_physical_props_by_id(next_id, &world.grids, &world.parts).unwrap();

        assert_eq!(props_a.0, Mass::grams(12339000));
        assert_eq!(props_a.1, DVec2::new(2.4357123, 1.2572942));
        assert_eq!(props_a, props_b);

        let grid_a = world.grids.try_get(grid_id).unwrap();
        let grid_b = world.grids.try_get(next_id).unwrap();

        assert_eq!(grid_a.vehicle_bounds, grid_b.vehicle_bounds);

        _ = set_grid_pose(&mut world, next_id, (-10.0, -20.0, 0.3).into());

        assert_world_is_consistent(&world);

        for id in [grid_id, next_id] {
            assert!(set_primary_computer_waypoint(id, (500.0, 800.0, 0.3), &mut world).is_ok());
            assert!(set_primary_computer_state(id, true, &mut world).is_ok());
        }

        for _ in 0..20 {
            for _ in 0..1000 {
                update_world(&mut world);
            }

            let pa = grid_pose(&world.grids, grid_id).unwrap();
            let pb = grid_pose(&world.grids, next_id).unwrap();

            let a1 = Angle::radians(pa.rotation);
            let a2 = Angle::radians(pb.rotation);

            println!(
                "{} {:?} {:?} {a1} {a2}",
                world.ticks,
                pa.to_tuple(),
                pb.to_tuple()
            );
        }

        let pa = grid_pose(&world.grids, grid_id).unwrap();
        let pb = grid_pose(&world.grids, next_id).unwrap();

        assert_eq!(pa.to_tuple(), (499.92654, 799.72156, -5.9987755));
        assert_eq!(pb.to_tuple(), (499.41153, 799.95984, -5.9996347));
    }

    #[test]
    fn splitting_vehicle_should_preserve_part_coordinates() {
        let mut world = WorldBuilder::new()
            .test_assets()
            .blueprint("bellerophon")
            .spawn("bellerophon", "kyle", (0.0, 0.0, 0.3))
            .build();

        let grid_id = get_grid_by_name(&world.grids, "kyle").unwrap();

        assert_eq!(world.grids.len(), 1);
        assert_eq!(world.parts.len(), 245);

        let grid = world.grids.try_get(grid_id).unwrap();

        assert_eq!(grid.origin(), (-10.546879, -7.4537697, 0.3).into());
        assert_eq!(grid.particle_location, (0.0, 0.0, 0.3).into());

        let mut gt_part_coords = BTreeMap::new();
        for (part_id, part) in world.parts.iter() {
            let center = grid.origin() * part.region.center_isometry();
            gt_part_coords.insert(*part_id, center);
        }

        assert_world_is_consistent(&world);

        let test_part_coords = |w: &World| {
            let mut new_part_coords = BTreeMap::new();
            for (part_id, part) in w.parts.iter() {
                let grid = w.grids.try_get(part.grid_id).unwrap();
                let center = grid.origin() * part.region.center_isometry();
                new_part_coords.insert(part_id, (part.grid_id, center));
            }
            for (id, (_grid_id, actual)) in new_part_coords {
                let expected = gt_part_coords.get(id).unwrap();
                let delta = expected.translation - actual.translation;

                assert!(
                    delta.length() < 0.01,
                    "Expected to find part {} at {}, was actually at {}",
                    id,
                    expected.translation,
                    actual.translation,
                );
            }
        };

        // parts' location in the world should never change beyond this point

        let mut parts_to_destroy = BTreeSet::new();
        for y in 0..100 {
            let c = PartCoord::new((32, y));
            let occ = grid.get_parts_at(c).unwrap_or(&PartOccupancy::EMPTY);
            for (_layer, id) in occ.iter() {
                parts_to_destroy.insert(id);
            }
        }

        assert_eq!(parts_to_destroy.len(), 5);

        let new_grid_id = world.spawner.next();

        for part_id in parts_to_destroy {
            let part = world.parts.try_get(part_id).unwrap();
            let proto = world.prototypes.try_get(part.prototype).unwrap();
            println!("Destroying {}: {:?}", part_id, proto.name);
            let result = destroy_part(&mut world, part_id);
            assert!(result.is_ok());

            // test this condition only when the grid is still whole
            if world.grids.len() == 1 {
                test_part_coords(&world);
            }
        }

        assert_world_is_consistent(&world);

        let grid_a = world.grids.try_get(grid_id).unwrap();
        let grid_b = world.grids.try_get(new_grid_id).unwrap();

        assert_eq!(grid_a.parts_mass, Mass::grams(112097000));
        assert_eq!(grid_b.parts_mass, Mass::grams(17231000));

        assert_eq!(grid_a.parts.len(), 141);
        assert_eq!(grid_b.parts.len(), 99);

        assert_eq!(world.grids.len(), 2);

        test_part_coords(&world);
    }

    #[test]
    fn get_inventory_at_grid_location() {
        let mut world = World::empty();

        let grid_id = spawn_empty_grid(&mut world, "whatever");

        let slots = vec![
            SlotPrototype {
                min: IVec2::ZERO,
                max: IVec2::ONE,
                ..SlotPrototype::default()
            },
            SlotPrototype {
                min: IVec2::ONE,
                max: IVec2::splat(2),
                ..SlotPrototype::default()
            },
        ];

        let inv_data = InventoryPrototype { slots };

        let proto = PartPrototype {
            name: "test-cargo".to_string(),
            mass: Mass::grams(1000),
            dims: UVec2::new(6, 4),
            layer: PartLayer::Internal,
            excavator_data: None,
            computer_data: None,
            inventory_data: Some(inv_data),
            thruster_data: None,
            machine_data: None,
            docking_port_data: None,
            debug_portal_data: None,
        };

        let proto_id = world.spawner.spawn();
        world.prototypes.spawn(proto_id, proto.clone());

        let pl = GridRegion::new((-3, 2), Rotation::North, proto.dims);
        let instance = PartInstance::new("test-cargo", PartLayer::Internal, pl);
        let part_id = insert_part(grid_id, &mut world, &instance, true).unwrap();

        for part in world.parts.values() {
            println!("- {:?}", part);
        }

        for inv in world.inventories.values() {
            for slot in inv.slots() {
                println!("- {:?}", slot);
            }
        }

        let grid = world.grids.try_get(grid_id).unwrap();

        for x in 0..3 {
            for y in 0..6 {
                let occ = grid.get_parts_at((x, y).into());
                assert!(occ.is_some(), "{}, {}", x, y);
            }
        }

        assert_eq!(grid.vehicle_bounds, ((0, 0).into(), (4, 6).into()));

        // the part looks like this in the grid frame, with inventories
        // indicated by @, #, %
        //
        //    --%%%%--
        //    --%%%%--
        //    ----@@--
        //    ----@@--
        //    ------##
        //    ------##

        let mut loc = GridLocation::new(grid_id, (3, 0).into());

        let inv_slot = inventory_at_c(loc, &world.grids, &world.parts, &world.inventories);

        assert_eq!(inv_slot, Ok((part_id, 0)));

        loc.coord = (2, 1).into();

        let inv_slot = inventory_at_c(loc, &world.grids, &world.parts, &world.inventories);

        assert_eq!(inv_slot, Ok((part_id, 1)));

        loc.coord = (0, 1).into();

        let failure = inventory_at_c(loc, &world.grids, &world.parts, &world.inventories);

        assert_eq!(failure, Err(BaryError::NoInvAt((0, 1).into())));

        loc.coord = (4, 7).into();

        let failure = inventory_at_c(loc, &world.grids, &world.parts, &world.inventories);

        assert_eq!(failure, Err(BaryError::NoPartsAt((4, 7).into())));
    }

    #[test]
    fn blueprint_versions() {
        let world = WorldBuilder::new()
            .test_assets()
            .blueprint(("pollux", 0))
            .blueprint(("pollux", 2))
            .build();

        assert_eq!(world.blueprints.len(), 2);

        let mut iter = world.blueprints.iter();

        let v0 = iter.next().unwrap();
        let v1 = iter.next().unwrap();

        assert_eq!(*v0.0, Ent(33));
        assert_eq!(*v1.0, Ent(34));
    }
}
