use crate::Planet;
use crate::*;
use bary_core::prelude::*;
use bary_orbital::*;
use early_returns::some_or_continue;
use early_returns::some_or_return;
use std::collections::BTreeSet;

/// Updates ring particles.
pub fn sys_update_ring_particles(particles: &mut Vec<Particle>, current_tick: u64) {
    particles.retain(|p| p.is_alive(current_tick));
}

/// Updates vehicle rigid body physics.
pub fn sys_propagate_grid_rigid_bodies(grids: &mut Components<VehicleGrid>) {
    for grid in grids.values_mut() {
        let body_frame_accel = grid.linear_acceleration();
        let omega = grid.angular_acceleration();
        let accel = rotate_f64(body_frame_accel, grid.particle_location.rotation);
        grid.particle_location.translation += grid.velocity.translation * NOMINAL_DT;
        grid.velocity.translation += accel * NOMINAL_DT;
        grid.particle_location.rotation += grid.velocity.rotation * NOMINAL_DT;
        grid.velocity.rotation += omega * NOMINAL_DT;
    }
}

pub fn sys_propagate_n_body_accel(
    grids: &mut Components<VehicleGrid>,
    planets: &Components<Planet>,
) {
    for grid in grids.values_mut() {
        let planet_id = some_or_continue!(grid.orbital_parent);
        let planet = some_or_continue!(planets.get(planet_id));
        let accel = planet.gravity(grid.particle_location.tr());
        grid.velocity.translation += accel * NOMINAL_DT;
    }
}

/// Updates vehicle thruster states according to the
/// primary computer on the parent grid. If the grid
/// has no computer, or that computer has not changed
/// its command, thruster states will not change.
/// Returns a list of grids which have changed their
/// thruster states.
pub fn sys_update_thrusters(
    thrusters: &mut Components<Thruster>,
    grids: &Components<VehicleGrid>,
    parts: &Components<Part>,
    computers: &Components<Computer>,
) -> BTreeSet<Ent> {
    let mut needs_update = BTreeSet::new();

    for (grid_id, grid) in grids.iter() {
        if grid.thrusters.is_empty() {
            continue;
        }

        let Some(cpu_id) = grid.computers.first() else {
            continue;
        };
        let Ok(cpu) = computers.try_get(*cpu_id) else {
            continue;
        };
        if !cpu.fired_this_tick {
            continue;
        }

        let mut thruster_changed = false;

        for thruster_id in &grid.thrusters {
            let Ok(thruster) = thrusters.try_get_mut(*thruster_id) else {
                continue;
            };
            let Ok(part) = parts.try_get(*thruster_id) else {
                continue;
            };

            let ctrl = cpu.vehicle_control;

            let tac = match part.region.rot() {
                Rotation::East => ctrl.plus_x,
                Rotation::North => ctrl.neg_y,
                Rotation::West => ctrl.neg_x,
                Rotation::South => ctrl.plus_y,
            };

            // TODO(optimization) reduce lookups by storing isometry on the thruster?
            let isometry = part.region.center_isometry();
            let center_of_thrust = isometry.translation;
            let rotation = part.region.rot();
            let wrench = body_frame_wrench(
                thruster.thrust(),
                center_of_thrust,
                rotation,
                grid.center_of_mass,
            );

            let changed = if thruster.is_rcs() {
                let can_torque = wrench.rotation.abs() > 0.5 && ctrl.attitude.abs() > 0.5;
                let is_torque =
                    can_torque && wrench.rotation.signum() as f64 == ctrl.attitude.signum();
                let is_linear = tac.throttle > 0.0 && tac.use_rcs;
                thruster.set_state(is_linear || is_torque, Some(*cpu_id))
            } else {
                thruster.set_state(!tac.use_rcs && tac.throttle > 0.0, Some(*cpu_id))
            };

            thruster_changed |= changed;
        }

        if thruster_changed {
            needs_update.insert(*grid_id);
        }
    }

    needs_update
}

/// Appends grid locations to tracker entities.
pub fn sys_update_trackers(
    trackers: &mut Components<Tracker>,
    grids: &Components<VehicleGrid>,
    ticks: u64,
) {
    if ticks % 20 > 0 {
        return;
    }

    let mut to_despawn = BTreeSet::new();

    for (grid_id, grid) in grids.iter() {
        let is_controllable = !grid.computers.is_empty();
        if trackers.try_get(*grid_id).is_err() && is_controllable {
            let tracker = Tracker::default();
            trackers.spawn(*grid_id, tracker);
        }
    }

    for (grid_id, tracker) in trackers.iter_mut() {
        let Ok(grid) = grids.try_get(*grid_id) else {
            to_despawn.insert(*grid_id);
            continue;
        };
        tracker.add(
            grid.origin(),
            grid.particle_location,
            grid.centroid_isometry(),
        );
    }

    for id in to_despawn {
        _ = trackers.despawn(id);
    }
}

pub fn get_excavator_tiles(
    id: Ent,
    ex: &Excavator,
    world: &World,
) -> BaryResult<Option<(Ent, Vec<GlobalTileIndex>)>> {
    let part = world.parts.try_get(id)?;
    let grid = world.grids.try_get(part.grid_id)?;
    let part_iso = grid.origin() * part.region.center_isometry();

    // TODO(gross) spatial lookups here or something.
    for (rock_id, rock) in world.asteroids.iter() {
        let wrt_asteroid = in_frame(rock.iso, part_iso.translation);

        if wrt_asteroid.length() > rock.ast.max_radius() as f64 {
            continue;
        }

        let mut tiles = Vec::new();

        let gc = GlobalTileIndex(vfloor_f64(wrt_asteroid / TERRAIN_TILE_WIDTH_METERS));

        let ri = 2 * (ex.radius as f64 / TERRAIN_TILE_WIDTH_METERS).ceil() as i32;

        for x in -ri..ri {
            for y in -ri..ri {
                let offset = IVec2::new(x, y);
                let g = GlobalTileIndex(gc.0 + offset);
                let c = g.center_isometry();
                let dist = c.translation.distance(wrt_asteroid);
                if dist < ex.radius as f64 {
                    tiles.push(g);
                    //     let o = g.origin_isometry();
                    //     fill_rectangle(d, ast.iso * o, tile_dims, Color::TEAL.alpha(0.5));
                }
            }
        }

        return Ok(Some((*rock_id, tiles)));
    }

    Ok(None)
}

/// Updates computers according to their current
/// directive and the thruster state of the parent vehicle.
pub fn sys_update_computers(
    computers: &mut Components<Computer>,
    parts: &Components<Part>,
    grids: &Components<VehicleGrid>,
) {
    for (cpu_id, computer) in computers.iter_mut() {
        computer.tick_forward();

        if !computer.fired_this_tick {
            continue;
        }

        if let Some(ctrl) = computer.current_control() {
            computer.vehicle_control = ctrl;
        } else if let Some(target_pose) = computer.current_waypoint() {
            let Ok(part) = parts.try_get(*cpu_id) else {
                continue;
            };

            let Ok(grid) = grids.try_get(part.grid_id) else {
                continue;
            };

            let pose = grid.particle_location;

            let target = PV::from_f64(target_pose.translation, Vec2::ZERO);
            let actual = PV::from_f64(pose.translation, grid.velocity.translation);

            let body = RigidBody {
                pv: actual,
                angle: pose.rotation as f64,
                angular_velocity: grid.velocity.rotation as f64,
            };

            let (ctrl, _status) =
                position_hold_control_law(target, target_pose.rotation as f64, &body, DVec2::ZERO);

            computer.vehicle_control = ctrl;
        } else if let Some(target) = computer.current_angle() {
            let Ok(part) = parts.try_get(*cpu_id) else {
                continue;
            };

            let Ok(grid) = grids.try_get(part.grid_id) else {
                continue;
            };

            let actual = Angle::radians(grid.particle_location.rotation);

            let body = RigidBody {
                pv: PV::ZERO,
                angle: actual.as_rad() as f64,
                angular_velocity: grid.velocity.rotation as f64,
            };

            let pid = PDCtrl::new(20.0, 50.0);

            let (ctrl, _status) = attitude_control_law(target.as_rad() as f64, &pid, &body);

            computer.vehicle_control = ctrl;
        } else if let Some(dv) = computer.delta_v_mut() {
            *dv -= Vec2::splat(0.1);
            let Ok(part) = parts.try_get(*cpu_id) else {
                continue;
            };

            let Ok(grid) = grids.try_get(part.grid_id) else {
                continue;
            };
            let target = dv.to_angle();
            let pid = PDCtrl::new(20.0, 50.0);
            let body = RigidBody {
                pv: PV::ZERO,
                angle: grid.particle_location.rotation as f64,
                angular_velocity: grid.velocity.rotation as f64,
            };
            let (ctrl, _status) = attitude_control_law(target as f64, &pid, &body);

            computer.vehicle_control = ctrl;
        }
    }
}

use bary_parts::*;
use chrono::NaiveDate;
use chrono::NaiveDateTime;
use chrono::TimeDelta;
use std::time::Duration;

pub fn timedelta_from_delta_ticks(ticks: i64) -> TimeDelta {
    TimeDelta::milliseconds(1000 / TICKS_PER_SECOND as i64 * ticks)
}

pub fn apparent_elapsed_time(ticks: u64) -> Duration {
    Duration::from_millis(1000 / TICKS_PER_SECOND * ticks)
}

pub fn apparent_datetime(ticks: u64) -> NaiveDateTime {
    let dur = apparent_elapsed_time(ticks);
    let epoch = NaiveDate::from_ymd_opt(2310, 7, 8)
        .unwrap()
        .and_hms_opt(3, 0, 0)
        .unwrap();
    epoch + dur
}

pub fn set_all_thrusters(grid_id: Ent, new_state: bool, world: &mut World) -> BaryResult<()> {
    let grid = world.grids.try_get(grid_id)?;
    for thruster_id in &grid.thrusters {
        let thruster = world.thrusters.try_get_mut(*thruster_id)?;
        thruster.set_state(new_state, None);
    }
    update_single_grid_acceleration(
        grid_id,
        &mut world.grids,
        &mut world.thrusters,
        &mut world.parts,
    )
}

pub fn update_grid_acceleration(dirty_set: BTreeSet<Ent>, world: &mut World) {
    sys_update_grid_acceleration_c(dirty_set, &mut world.grids, &world.thrusters, &world.parts);
}

pub fn get_blueprint(world: &World, grid_id: Ent) -> BaryResult<Blueprint> {
    get_blueprint_c(
        &world.grids,
        &world.parts,
        &world.pipes,
        &world.prototypes,
        grid_id,
    )
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn part_prototypes() {
        let world = WorldBuilder::new()
            .test_assets()
            .blueprint("pollux")
            .blueprint("bellerophon")
            .blueprint("remora")
            .blueprint("spacestation")
            .build();

        let mut iter = world.prototypes.iter();

        let (id, proto) = iter.next().unwrap();
        assert_eq!(*id, Ent(0));
        assert_eq!(proto.part_name(), "angled-frame");

        let (id, proto) = iter.next().unwrap();
        assert_eq!(*id, Ent(1));
        assert_eq!(proto.part_name(), "antenna");

        let (id, proto) = iter.next().unwrap();
        assert_eq!(*id, Ent(2));
        assert_eq!(proto.part_name(), "battery");

        let (id, proto) = iter.next().unwrap();
        assert_eq!(*id, Ent(3));
        assert_eq!(proto.part_name(), "cargo");

        let (id, proto) = iter.next().unwrap();
        assert_eq!(*id, Ent(4));
        assert_eq!(proto.part_name(), "chemical-plant");

        let (id, proto) = iter.next().unwrap();
        assert_eq!(*id, Ent(5));
        assert_eq!(proto.part_name(), "container");

        let (id, proto) = iter.next().unwrap();
        assert_eq!(*id, Ent(6));
        assert_eq!(proto.part_name(), "cpu");

        let (id, proto) = iter.next().unwrap();
        assert_eq!(*id, Ent(7));
        assert_eq!(proto.part_name(), "debug-item-source");

        let (id, proto) = iter.next().unwrap();
        assert_eq!(*id, Ent(8));
        assert_eq!(proto.part_name(), "debug-sink");

        let (id, proto) = iter.next().unwrap();
        assert_eq!(*id, Ent(9));
        assert_eq!(proto.part_name(), "debug-source");

        let (id, proto) = iter.next().unwrap();
        assert_eq!(*id, Ent(10));
        assert_eq!(proto.part_name(), "docking-port");

        assert_world_is_consistent(&world);
    }

    #[test]
    fn vehicle_spawning_and_despawning() {
        let mut world = WorldBuilder::new()
            .test_assets()
            .blueprint("pollux")
            .blueprint("bellerophon")
            .blueprint("remora")
            .blueprint("spacestation")
            .build();

        let name = "pollux";

        // get the blueprint for the pollux
        let bp = get_blueprint_by_id(&world.blueprints, &name.into())
            .expect("Expected a blueprint")
            .clone();

        let expected_grid_id = world.spawner.next();

        // spawn that vehicle using its blueprint
        let grid_id =
            spawn_grid_from_blueprint(&mut world, name.to_string(), Some(&name.into()), &bp)
                .expect("Expected the grid ID");

        // this entity should be the same every time
        assert_eq!(grid_id, expected_grid_id);

        // the mass should already be computed
        let grid = world.grids.get(expected_grid_id).unwrap();
        assert_eq!(grid.parts_mass, Mass::grams(35134000));

        assert_eq!(world.grids.len(), 1);
        assert_eq!(world.parts.len(), 98);
        assert_eq!(world.thrusters.len(), 18);
        assert_eq!(world.computers.len(), 1);
        assert_eq!(world.lights.len(), 12);

        // get the computer entity
        let (id, cpu) = world.computers.iter().next().unwrap();

        // these entities should be the same every time
        assert_eq!(*id, Ent(62));
        assert_eq!(cpu.prototype, Ent(6));

        // get the prototype definition for the computer
        let proto = world.prototypes.get(cpu.prototype).unwrap();

        // it should be the "cpu" part
        assert_eq!(proto.part_name(), "cpu");

        // despawning should work, of course
        let result = despawn_grid(&mut world, grid_id);
        assert_eq!(result, Ok(()));

        // now the world should be empty
        assert_eq!(world.grids.len(), 0);
        assert_eq!(world.parts.len(), 0);
        assert_eq!(world.thrusters.len(), 0);
        assert_eq!(world.computers.len(), 0);
        assert_eq!(world.lights.len(), 0);

        // doing this again should return an error
        let result = despawn_grid(&mut world, grid_id);

        assert_eq!(result, Err(BaryError::EntityNotFound(grid_id)));

        assert_world_is_consistent(&world);
    }

    #[test]
    fn nearest_grid() {
        let mut world = WorldBuilder::new()
            .test_assets()
            .blueprint("pollux")
            .blueprint("bellerophon")
            .blueprint("remora")
            .blueprint("spacestation")
            .build();

        assert!(get_closest_grid(&world.grids, DVec2::new(100.0, 200.0), None).is_none());

        let bp_id: BlueprintId = "remora".into();
        let expected_id = world.spawner.next();
        let id = spawn_grid_with_random_name(&mut world, bp_id).unwrap();
        assert_eq!(id, expected_id);

        let grid = world.grids.try_get_mut(id).unwrap();
        grid.particle_location.translation = DVec2::new(40.0, 156.0);
        grid.particle_location.rotation = 30.0f64.to_radians();

        let centroid = grid.centroid_isometry();

        for _ in 0..100 {
            update_world(&mut world);
            let test_pos = centroid.offset(Vec2::new(100.0, 200.0)).translation;
            let e = get_closest_grid(&world.grids, test_pos, None);
            assert_eq!(e, Some((expected_id, DVec2::new(99.99999, 199.99998))));
        }

        assert_world_is_consistent(&world);
    }

    #[test]
    fn insert_parts() {
        let mut world = WorldBuilder::new()
            .test_assets()
            .blueprint("pollux")
            .build();

        let initial_id = world.spawner.next();

        let part_name = "motor";

        let proto_id = get_proto_by_name(&world.prototypes, part_name).unwrap();

        let proto = world.prototypes.try_get(proto_id).unwrap();
        let dims = proto.dims;

        assert_eq!(proto_id, initial_id - 16);

        let grid_id = spawn_grid_with_random_name(&mut world, "pollux").unwrap();

        assert_eq!(world.parts.len(), 98);
        assert_eq!(world.thrusters.len(), 18);

        assert_eq!(grid_id, initial_id);

        let instance = PartInstance::new(
            part_name,
            PartLayer::Internal,
            GridRegion::new((2, 20), Rotation::East, dims),
        );

        let expected_id = world.spawner.next();

        let id = insert_part(grid_id, &mut world, &instance, true).unwrap();

        assert_world_is_consistent(&world);

        assert_eq!(id, expected_id);

        let part = world.parts.get(id).unwrap();

        assert_eq!(part.grid_id, grid_id);
        assert_eq!(part.prototype, proto_id);
        assert_eq!(
            part.region,
            // TODO allow insertion at a given region
            GridRegion::new((2, 20), Rotation::East, (6, 3))
        );

        assert_eq!(world.parts.len(), 99);
        assert_eq!(world.thrusters.len(), 19);
    }

    #[test]
    fn parts_mass() {
        let mut world = WorldBuilder::new()
            .test_assets()
            .blueprint("pollux")
            .blueprint("bellerophon")
            .blueprint("remora")
            .blueprint("spacestation")
            .build();

        let id = spawn_grid_with_random_name(&mut world, "pollux").unwrap();
        let mass = world.grids.try_get(id).unwrap().parts_mass;
        assert_eq!(mass, Mass::grams(35134000));

        let id = spawn_grid_with_random_name(&mut world, "bellerophon").unwrap();
        let mass = world.grids.try_get(id).unwrap().parts_mass;
        assert_eq!(mass, Mass::grams(178051000));

        let id = spawn_grid_with_random_name(&mut world, "remora").unwrap();
        let mass = world.grids.try_get(id).unwrap().parts_mass;
        assert_eq!(mass, Mass::grams(12339000));

        let id = spawn_grid_with_random_name(&mut world, "spacestation").unwrap();
        let mass = world.grids.try_get(id).unwrap().parts_mass;
        assert_eq!(mass, Mass::grams(145638000));

        assert_world_is_consistent(&world);
    }

    #[test]
    fn calculate_blueprints() {
        let mut world = WorldBuilder::new()
            .test_assets()
            .blueprint("pollux")
            .blueprint("bellerophon")
            .blueprint("remora")
            .blueprint("spacestation")
            .build();

        let mut expected = get_blueprint_by_id(&world.blueprints, &"pollux".into())
            .unwrap()
            .clone();

        expected.normalize_coordinates();

        let id = spawn_grid_with_random_name(&mut world, "pollux").unwrap();

        let mut actual = get_blueprint_c(
            &world.grids,
            &world.parts,
            &world.pipes,
            &world.prototypes,
            id,
        )
        .unwrap();

        actual.normalize_coordinates();

        assert_eq!(actual.part_count(), expected.part_count());

        for (a, b) in actual.parts().zip(expected.parts()) {
            assert_eq!(a.0, b.0);
            assert_eq!(a.1, b.1);
        }

        // TODO this test needs to be revived once pipes are added
        // to the new ECS
        // assert_eq!(actual.pipe_count(), expected.pipe_count());

        // failing because pipes aren't implemented.
        // assert_eq!(actual, expected);

        assert_world_is_consistent(&world);
    }

    #[test]
    fn bad_part_insertion() {
        let mut world = WorldBuilder::new().test_assets().build();
        let id = spawn_empty_grid(&mut world, "whatever");

        let instance = PartInstance::new(
            "dingus",
            PartLayer::Internal,
            GridRegion::new((0, 0), Rotation::East, (3, 3)),
        );

        let result = insert_part(id, &mut world, &instance, true);

        assert_eq!(result, Err(BaryError::BadPartName));

        let instance = PartInstance::new(
            "cargo",
            PartLayer::Internal,
            GridRegion::new((0, 0), Rotation::East, (3, 3)),
        );

        let result = insert_part(Ent(103), &mut world, &instance, true);

        assert_eq!(result, Err(BaryError::EntityNotFound(Ent(103))));

        assert_world_is_consistent(&world);
    }

    #[test]
    fn setting_thruster_state() {
        let mut world = WorldBuilder::new().test_assets().build();

        let expected_id = world.spawner.next();

        let grid_id = spawn_empty_grid(&mut world, "whatever");

        let grid = world.grids.try_get(grid_id).unwrap();

        assert_eq!(grid.parts.len(), 0);
        assert_eq!(grid.parts_mass, Mass::ZERO);

        assert_eq!(grid_id, expected_id);

        let instance_a = PartInstance::new(
            "motor",
            PartLayer::Internal,
            GridRegion::new((0, 0), Rotation::East, (6, 3)),
        );

        let instance_b = PartInstance::new(
            "small-motor",
            PartLayer::Internal,
            GridRegion::new((3, 3), Rotation::North, (4, 2)),
        );

        let a_id = insert_part(grid_id, &mut world, &instance_a, true).unwrap();
        let b_id = insert_part(grid_id, &mut world, &instance_b, true).unwrap();

        let grid = world.grids.try_get(grid_id).unwrap();

        assert_eq!(grid.parts.len(), 2);
        assert_eq!(grid.parts, [a_id, b_id].into());
        assert_eq!(grid.thrusters, [a_id, b_id].into());
        assert_eq!(grid.parts_mass, Mass::grams(870000));

        assert_eq!(a_id, expected_id + 1);
        assert_eq!(b_id, expected_id + 2);

        let r1 = set_thruster_state(a_id, &mut world, true);
        let r2 = set_thruster_state(b_id, &mut world, true);

        update_single_grid_acceleration(
            grid_id,
            &mut world.grids,
            &mut world.thrusters,
            &mut world.parts,
        )
        .unwrap();

        assert_eq!(r1, Ok(grid_id));
        assert_eq!(r2, Ok(grid_id));

        let sum =
            get_sum_linear_forces(grid_id, &world.grids, &world.parts, &world.thrusters).unwrap();

        assert_eq!(sum.x, 400000.0);
        assert_eq!(sum.y, 320000.0);

        let grid = world.grids.try_get(grid_id).unwrap();

        assert_eq!(
            grid.body_frame_forces.translation,
            DVec2::new(400000.0, 320000.0)
        );

        let r1 = set_thruster_state(a_id, &mut world, false);
        let r2 = set_thruster_state(b_id, &mut world, true);

        update_single_grid_acceleration(
            grid_id,
            &mut world.grids,
            &mut world.thrusters,
            &mut world.parts,
        )
        .unwrap();

        assert_eq!(r1, Ok(grid_id));
        assert_eq!(r2, Ok(grid_id));

        let grid = world.grids.try_get(grid_id).unwrap();

        assert_eq!(
            grid.body_frame_forces.translation,
            DVec2::new(0.0, 320000.0)
        );

        let r1 = set_thruster_state(a_id, &mut world, false);
        let r2 = set_thruster_state(b_id, &mut world, false);

        update_single_grid_acceleration(
            grid_id,
            &mut world.grids,
            &mut world.thrusters,
            &mut world.parts,
        )
        .unwrap();

        assert_eq!(r1, Ok(grid_id));
        assert_eq!(r2, Ok(grid_id));

        let grid = world.grids.try_get(grid_id).unwrap();

        assert_eq!(
            grid.body_frame_forces.translation,
            IVec2::new(0, 0).as_dvec2()
        );
    }

    #[test]
    fn parts_center_of_mass() {
        let mut world = WorldBuilder::new()
            .test_assets()
            .blueprint("pollux")
            .build();

        let id = spawn_grid_with_random_name(&mut world, "pollux").unwrap();
        let (_mass, com) = get_grid_physical_props_by_id(id, &world.grids, &world.parts).unwrap();

        assert_eq!(com, DVec2::new(0.001067417, 0.022271877));

        let cargo_id = get_proto_by_name(&world.prototypes, "cargo").unwrap();
        let cargo_proto = world.prototypes.try_get(cargo_id).unwrap();

        assert_eq!(cargo_proto.dims, (6, 6).into());
        assert_eq!(cargo_proto.dims_meters(), (1.5, 1.5).into());

        let instance = PartInstance::from_prototype(cargo_proto, (0, 0).into(), Rotation::East);

        let grid_id = spawn_empty_grid(&mut world, "whatever");
        _ = insert_part(grid_id, &mut world, &instance, true);

        let (_mass, com) =
            get_grid_physical_props_by_id(grid_id, &world.grids, &world.parts).unwrap();

        assert_eq!(com, DVec2::splat(0.75));
    }

    #[test]
    fn adding_part_to_empty_grid_retains_origin_pose() {
        // this caught me by surprise, but this is actually perfectly consistent behavior;
        // adding a part to an empty grid will retain the _origin_ of that grid,
        // NOT the center of mass, in the inertial frame.

        let mut world = WorldBuilder::new().test_assets().build();

        let grid_id = spawn_empty_grid(&mut world, "empty");
        let pose = grid_pose(&world.grids, grid_id).unwrap();
        let origin = get_grid_origin(&world.grids, grid_id).unwrap();
        assert_eq!(pose, Isometry2d::ZERO);
        assert_eq!(origin, Isometry2d::ZERO);

        let insertion_point = PartCoord::new((1, 1));

        // insert a frame
        let instance = PartInstance {
            name: "frame".to_string(),
            layer: PartLayer::Structural,
            region: GridRegion::new(insertion_point, Rotation::East, (2, 2)),
        };

        let result = insert_part(grid_id, &mut world, &instance, true);
        assert!(result.is_ok());

        let part_dims = instance.region.grid_aligned_dims().to_meters();

        assert_eq!(part_dims, Vec2::splat(0.5));

        let pose = grid_pose(&world.grids, grid_id).unwrap();
        let origin = get_grid_origin(&world.grids, grid_id).unwrap();

        let expected_com = insertion_point.to_meters() + part_dims / 2.0;

        assert_eq!(pose, expected_com.into());
        assert_eq!(origin, Isometry2d::ZERO);

        assert_world_is_consistent(&world);
    }

    #[test]
    fn pure_linear_acceleration() {
        let mut world = WorldBuilder::new().test_assets().build();

        // modifying the prototype for motor so it has easy quantities
        let proto_id = get_proto_by_name(&world.prototypes, "small-motor").unwrap();
        let proto = world.prototypes.try_get_mut(proto_id).unwrap();

        proto.mass = Mass::kilograms(1000);
        if let Some(t) = &mut proto.thruster_data {
            // 3500 newtons
            t.thrust = 3500.0;
        }

        let dims = proto.dims;

        let grid_id = spawn_empty_grid(&mut world, "testbed");

        let insert_location = IVec2::new(0, -1);

        let instance = PartInstance {
            name: "small-motor".to_string(),
            layer: PartLayer::Internal,
            region: GridRegion::new(insert_location, Rotation::East, dims),
        };

        let thruster_id = insert_part(grid_id, &mut world, &instance, true).unwrap();

        let grid = world.grids.try_get(grid_id).expect("Expected a grid");

        dbg!(grid);

        let expected_com = PartCoord(dims.as_ivec2()).to_meters().as_dvec2() / 2.0
            + PartCoord(insert_location).to_meters().as_dvec2();

        assert_eq!(grid.center_of_mass, expected_com);
        assert_eq!(grid.origin(), (0.0, 0.0, 0.0).into());

        let (_mass, com) =
            get_grid_physical_props_by_id(grid_id, &world.grids, &world.parts).unwrap();

        assert_eq!(com, expected_com);

        // obviously, turn the main thruster on
        let r = set_thruster_state(thruster_id, &mut world, true);
        assert_eq!(r, Ok(grid_id));

        update_single_grid_acceleration(
            grid_id,
            &mut world.grids,
            &mut world.thrusters,
            &mut world.parts,
        )
        .unwrap();

        let grid = world.grids.try_get_mut(grid_id).unwrap();

        grid.particle_location.translation = DVec2::ZERO;

        assert_eq!(grid.body_frame_forces.translation, DVec2::new(3500.0, 0.0));
        assert_eq!(grid.body_frame_forces.rotation, 0.0);
        assert_eq!(grid.parts_mass, Mass::kilograms(1000));

        // body frame acceleration should be 3.5 m/s^2
        assert_eq!(grid.linear_acceleration(), DVec2::new(3.5, 0.0));
        assert_eq!(grid.angular_acceleration(), 0.0);

        // run the simulation for 2 seconds at 50 Hz
        for _ in 0..100 {
            update_world(&mut world);
        }

        let iso = world.grids.try_get(grid_id).unwrap().particle_location;

        // this is an approximation of the following
        // continuous time kinematic equation:
        // d = 1/2 at^2  --> 0.5 * 3.5 * 2^2 = 7
        assert_eq!(iso.translation, DVec2::new(6.9299994, 0.0));
        assert_eq!(iso.rotation, 0.0);
    }

    #[test]
    fn pure_linear_acceleration_2() {
        let mut world = World::empty();

        let part_name = "test-motor";

        let thruster_data = ThrusterPrototype {
            model: "test-motor-model".to_string(),
            thrust: 8000.0,
            exhaust_velocity: 6000.0,
            is_rcs: false,
            throttle_rate: 0.0,
            primary_color: [0.0, 0.0, 0.0, 0.0],
            secondary_color: [0.0, 0.0, 0.0, 0.0],
            plume_length: 1.0,
            plume_angle: 0.1,
            minimum_throttle: 0.0,
            particle_scale: 1.0,
        };

        let proto = PartPrototype {
            name: part_name.to_string(),
            mass: Mass::kilograms(1000),
            dims: UVec2::new(4, 2),
            layer: PartLayer::Internal,
            excavator_data: None,
            computer_data: None,
            inventory_data: None,
            thruster_data: Some(thruster_data),
            machine_data: None,
            docking_port_data: None,
            debug_portal_data: None,
        };

        let dims = proto.dims;

        let proto_id = world.spawner.spawn();
        world.prototypes.spawn(proto_id, proto);

        let region = GridRegion::new((0, 0), Rotation::East, dims);

        let grid_id = spawn_empty_grid(&mut world, "testbed");

        let instance = PartInstance {
            name: part_name.to_string(),
            layer: PartLayer::Internal,
            region,
        };

        let thruster_id = insert_part(grid_id, &mut world, &instance, true).unwrap();

        _ = set_thruster_state(thruster_id, &mut world, true);

        update_single_grid_acceleration(
            grid_id,
            &mut world.grids,
            &mut world.thrusters,
            &mut world.parts,
        )
        .unwrap();

        let grid = world.grids.try_get_mut(grid_id).unwrap();

        grid.particle_location.translation = DVec2::ZERO;

        assert_eq!(grid.center_of_mass, DVec2::new(0.5, 0.25));

        assert_eq!(grid_pose(&world.grids, grid_id), Some(Isometry2d::ZERO));

        let expected_poses = [
            (0.000000000000, 0.000000000000, 0.000000000000),
            (0.003199999919, 0.000000000000, 0.000000000000),
            (0.009599999525, 0.000000000000, 0.000000000000),
            (0.019199999049, 0.000000000000, 0.000000000000),
            (0.031999997795, 0.000000000000, 0.000000000000),
            (0.047999996692, 0.000000000000, 0.000000000000),
            (0.067199990153, 0.000000000000, 0.000000000000),
            (0.089599989355, 0.000000000000, 0.000000000000),
            (0.115199983120, 0.000000000000, 0.000000000000),
            (0.143999978900, 0.000000000000, 0.000000000000),
            (0.175999969244, 0.000000000000, 0.000000000000),
            (0.211199969053, 0.000000000000, 0.000000000000),
            (0.249599963427, 0.000000000000, 0.000000000000),
            (0.291199952364, 0.000000000000, 0.000000000000),
            (0.335999935865, 0.000000000000, 0.000000000000),
            (0.383999943733, 0.000000000000, 0.000000000000),
            (0.435199946165, 0.000000000000, 0.000000000000),
            (0.489599943161, 0.000000000000, 0.000000000000),
            (0.547199964523, 0.000000000000, 0.000000000000),
            (0.607999980450, 0.000000000000, 0.000000000000),
            (0.671999990940, 0.000000000000, 0.000000000000),
            (0.739199995995, 0.000000000000, 0.000000000000),
            (0.809599995613, 0.000000000000, 0.000000000000),
            (0.883199989796, 0.000000000000, 0.000000000000),
            (0.959999978542, 0.000000000000, 0.000000000000),
            (1.039999961853, 0.000000000000, 0.000000000000),
            (1.123199939728, 0.000000000000, 0.000000000000),
            (1.209599971771, 0.000000000000, 0.000000000000),
            (1.299199938774, 0.000000000000, 0.000000000000),
            (1.391999959946, 0.000000000000, 0.000000000000),
            (1.487999916077, 0.000000000000, 0.000000000000),
            (1.587199926376, 0.000000000000, 0.000000000000),
            (1.689599871635, 0.000000000000, 0.000000000000),
            (1.795199871063, 0.000000000000, 0.000000000000),
            (1.903999805450, 0.000000000000, 0.000000000000),
            (2.015999794006, 0.000000000000, 0.000000000000),
            (2.131199836731, 0.000000000000, 0.000000000000),
            (2.249599695206, 0.000000000000, 0.000000000000),
            (2.371199607849, 0.000000000000, 0.000000000000),
            (2.495999574661, 0.000000000000, 0.000000000000),
            (2.623999595642, 0.000000000000, 0.000000000000),
            (2.755199432373, 0.000000000000, 0.000000000000),
            (2.889599323273, 0.000000000000, 0.000000000000),
            (3.027199268341, 0.000000000000, 0.000000000000),
            (3.167999267578, 0.000000000000, 0.000000000000),
            (3.311999320984, 0.000000000000, 0.000000000000),
            (3.459199190140, 0.000000000000, 0.000000000000),
            (3.609599113464, 0.000000000000, 0.000000000000),
            (3.763199090958, 0.000000000000, 0.000000000000),
            (3.919999122620, 0.000000000000, 0.000000000000),
            (4.079998970032, 0.000000000000, 0.000000000000),
            (4.243198871613, 0.000000000000, 0.000000000000),
            (4.409598827362, 0.000000000000, 0.000000000000),
            (4.579198837280, 0.000000000000, 0.000000000000),
            (4.751998901367, 0.000000000000, 0.000000000000),
            (4.927999019623, 0.000000000000, 0.000000000000),
            (5.107198715210, 0.000000000000, 0.000000000000),
            (5.289598464966, 0.000000000000, 0.000000000000),
            (5.475198268890, 0.000000000000, 0.000000000000),
            (5.663998126984, 0.000000000000, 0.000000000000),
            (5.855998039246, 0.000000000000, 0.000000000000),
            (6.051198005676, 0.000000000000, 0.000000000000),
            (6.249598026276, 0.000000000000, 0.000000000000),
            (6.451198101044, 0.000000000000, 0.000000000000),
            (6.655998229980, 0.000000000000, 0.000000000000),
            (6.863997936249, 0.000000000000, 0.000000000000),
            (7.075197696686, 0.000000000000, 0.000000000000),
            (7.289597511292, 0.000000000000, 0.000000000000),
            (7.507197380066, 0.000000000000, 0.000000000000),
            (7.727997303009, 0.000000000000, 0.000000000000),
            (7.951997280121, 0.000000000000, 0.000000000000),
            (8.179197311401, 0.000000000000, 0.000000000000),
            (8.409597396851, 0.000000000000, 0.000000000000),
            (8.643197059631, 0.000000000000, 0.000000000000),
            (8.879997253418, 0.000000000000, 0.000000000000),
            (9.119997024536, 0.000000000000, 0.000000000000),
            (9.363197326660, 0.000000000000, 0.000000000000),
            (9.609597206116, 0.000000000000, 0.000000000000),
            (9.859196662903, 0.000000000000, 0.000000000000),
            (10.111996650696, 0.000000000000, 0.000000000000),
            (10.367996215820, 0.000000000000, 0.000000000000),
            (10.627196311951, 0.000000000000, 0.000000000000),
            (10.889595985413, 0.000000000000, 0.000000000000),
            (11.155196189880, 0.000000000000, 0.000000000000),
            (11.423995971680, 0.000000000000, 0.000000000000),
            (11.695995330811, 0.000000000000, 0.000000000000),
            (11.971195220947, 0.000000000000, 0.000000000000),
            (12.249594688416, 0.000000000000, 0.000000000000),
            (12.531194686890, 0.000000000000, 0.000000000000),
            (12.815994262695, 0.000000000000, 0.000000000000),
            (13.103994369507, 0.000000000000, 0.000000000000),
            (13.395194053650, 0.000000000000, 0.000000000000),
            (13.689594268799, 0.000000000000, 0.000000000000),
            (13.987194061279, 0.000000000000, 0.000000000000),
            (14.287993431091, 0.000000000000, 0.000000000000),
            (14.591993331909, 0.000000000000, 0.000000000000),
            (14.899192810059, 0.000000000000, 0.000000000000),
            (15.209592819214, 0.000000000000, 0.000000000000),
            (15.523192405701, 0.000000000000, 0.000000000000),
            (15.839992523193, 0.000000000000, 0.000000000000),
        ];

        for i in 0..100 {
            let expected = expected_poses[world.ticks as usize];
            update_world(&mut world);
            let pose = grid_pose(&world.grids, grid_id).unwrap().to_tuple();
            assert_eq!(pose, expected, "Epoch {}", i);
            // println!("({:0.12}, {:0.12}, {:0.12}),", pose.0, pose.1, pose.2);
        }
    }

    #[test]
    fn vehicle_arrives_at_its_destination() {
        // disclaimer: this is a very fragile test, and can be affected
        // by fuel requirements, changing ship design, etc.
        // I wouldn't be surprised if I have to get rid of it.
        // But it's good for now.

        let waypoint: Isometry2d = (600.0, 800.0, 0.5).into();

        let mut world = WorldBuilder::new()
            .test_assets()
            .blueprint("pollux")
            .spawn("pollux", "fran", Isometry2d::ZERO)
            .waypoint("fran", waypoint)
            .build();

        let grid_id = get_grid_by_name(&world.grids, "fran").unwrap();

        for _ in 0..20 {
            for _ in 0..1000 {
                update_world(&mut world);
            }

            let elapsed = apparent_elapsed_time(world.ticks);
            let pose = grid_pose(&world.grids, grid_id).unwrap().to_tuple();
            println!(
                "{} ({:0.1}): {}, {}, {}",
                world.ticks,
                elapsed.as_secs_f64(),
                pose.0,
                pose.1,
                pose.2
            );
        }

        assert_eq!(world.grid_acceleration_updates, 96);

        let pose = grid_pose(&world.grids, grid_id).unwrap();
        let error = pose.translation - waypoint.translation;

        assert!(error.x.abs() < 3.0);
        assert!(error.y.abs() < 3.0);
    }

    // use super::*;
    // use crate::sim::*;
    // use crate::tests::assert_world_is_consistent;
    // use crate::world_builder::WorldBuilder;

    #[test]
    fn rebuilding_grid_from_islands() {
        let mut world = WorldBuilder::new()
            .test_assets()
            .blueprint("pollux")
            .spawn("pollux", "harriet", (0.0, 0.0, 0.0))
            .build();

        let grid_id = get_grid_by_name(&world.grids, "harriet").unwrap();
        let grid = world.grids.try_get(grid_id).unwrap();

        assert_eq!(grid.parts_mass, Mass::grams(35134000));
        assert_eq!(grid.parts.len(), 98);

        let lower = grid.vehicle_bounds.0;

        // slice a thing down the middle of the ship
        let mut parts = BTreeSet::new();
        let x = 22;
        for y in 0..40 {
            let x = x + lower.x;
            let y = y + lower.y;
            if let Some(occ) = grid.occupancy.get(&(x, y)) {
                for (_, id) in occ.iter() {
                    parts.insert(id);
                }
            }
        }

        for part_id in parts {
            let r = destroy_part_without_integrity_check(&mut world, part_id, true);
            assert!(r.is_ok());
        }

        let mut grid = world.grids.try_get(grid_id).unwrap().clone();
        let islands = grid.calculate_islands();

        assert_eq!(islands.len(), 2);

        let rebuilt = rebuild_index_from_islands(&mut grid, &islands, &world.parts).unwrap();

        assert_eq!(rebuilt.len(), 2);

        let ra = &rebuilt[0];
        let rb = &rebuilt[1];

        assert_eq!(ra.parts.len(), 45);
        assert_eq!(rb.parts.len(), 40);

        assert_eq!(ra.thrusters.len(), 9);
        assert_eq!(rb.thrusters.len(), 9);

        assert_eq!(ra.computers.len(), 0);
        assert_eq!(rb.computers.len(), 1);

        assert_eq!(ra.lights.len(), 6);
        assert_eq!(rb.lights.len(), 6);

        assert_eq!(ra.parts_mass, Mass::grams(16817000));
        assert_eq!(rb.parts_mass, Mass::grams(14797000));
    }

    #[test]
    fn removing_parts_from_grid() {
        let mut world = WorldBuilder::new()
            .test_assets()
            .blueprint("pollux")
            .spawn("pollux", "ted", (0.0, 0.0, 0.0))
            .build();

        let grid_id = get_grid_by_name(&world.grids, "ted").unwrap();
        let grid = world.grids.try_get(grid_id).unwrap();
        let parts: Vec<_> = grid.parts.iter().collect();

        assert_eq!(grid.parts_mass, Mass::grams(35134000));
        assert_eq!(grid.parts.len(), 98);

        let part_a = *parts[12];
        let part_b = *parts[20];
        let part_c = *parts[37];

        assert_eq!(part_a, Ent(48));
        assert_eq!(part_b, Ent(56));
        assert_eq!(part_c, Ent(73));

        let op_a = destroy_part_without_integrity_check(&mut world, part_a, false);
        let op_b = destroy_part_without_integrity_check(&mut world, part_b, false);
        let op_c = destroy_part_without_integrity_check(&mut world, part_c, true);

        let region_a = GridRegion::new((-12, -9), Rotation::North, (1, 1));
        let region_b = GridRegion::new((10, -5), Rotation::South, (2, 2));
        let region_c = GridRegion::new((10, 3), Rotation::South, (2, 2));

        let part_a = PartInstance::new("rcs", PartLayer::Internal, region_a);
        let part_b = PartInstance::new("plate", PartLayer::Exterior, region_b);
        let part_c = PartInstance::new("plate", PartLayer::Exterior, region_c);

        assert_eq!(op_a, Ok((part_a, grid_id)));
        assert_eq!(op_b, Ok((part_b, grid_id)));
        assert_eq!(op_c, Ok((part_c, grid_id)));

        let grid = world.grids.try_get(grid_id).unwrap();

        assert_eq!(grid.parts_mass, Mass::grams(35073000));
        assert_eq!(grid.parts.len(), 95);

        assert_world_is_consistent(&world);
    }

    #[test]
    fn split_grid_into_two_grids() {
        let mut world = WorldBuilder::new()
            .test_assets()
            .blueprint("pollux")
            .spawn("pollux", "julia", (0.0, 0.0, 0.0))
            .build();

        let grid_id = get_grid_by_name(&world.grids, "julia").unwrap();

        // this should fail if the grid ID is bad, obviously.
        let result = split_grid_if_necessary(&mut world, Ent(0));

        assert_eq!(result, Err(BaryError::EntityNotFound(Ent(0))));

        let result = split_grid_if_necessary(&mut world, grid_id);

        assert_eq!(result, Ok(vec![]));

        let grid = world.grids.try_get(grid_id).unwrap();
        assert_eq!(grid.parts_mass, Mass::grams(35134000));
        assert_eq!(grid.parts.len(), 98);

        let lower = grid.vehicle_bounds.0;

        // slice a thing down the middle of the ship
        let mut parts = BTreeSet::new();
        let x = 25;
        for y in 0..40 {
            let x = x + lower.x;
            let y = y + lower.x;
            if let Some(occ) = grid.occupancy.get(&(x, y)) {
                for (_, id) in occ.iter() {
                    parts.insert(id);
                }
            }
        }

        assert_eq!(parts.len(), 8);

        for part_id in parts {
            let r = destroy_part_without_integrity_check(&mut world, part_id, true);
            assert!(r.is_ok());
        }

        let sec_grid_id = world.spawner.next();

        let result = split_grid_if_necessary(&mut world, grid_id);

        assert_eq!(result, Ok(vec![grid_id, sec_grid_id]));

        let grid = world.grids.try_get(grid_id).unwrap();
        assert_eq!(grid.parts_mass, Mass::grams(17757000));
        assert_eq!(grid.parts.len(), 52);

        let grid = world.grids.try_get(sec_grid_id).unwrap();
        assert_eq!(grid.parts_mass, Mass::grams(14247000));
        assert_eq!(grid.parts.len(), 38);

        assert_world_is_consistent(&world);
    }
}
