use crate::*;
use anyhow::ensure;

pub fn computer_pointers_are_consistent(world: &World) -> Result<(), anyhow::Error> {
    for (_grid_id, grid) in world.grids.iter() {
        for id in &grid.computers {
            ensure!(
                world.computers.try_get(*id).is_ok(),
                "Failed to lookup computer in this grid"
            );
        }
    }

    Ok(())
}

pub fn part_bidirectional_pointers_are_consistent(world: &World) -> Result<(), anyhow::Error> {
    for (grid_id, grid) in world.grids.iter() {
        for id in &grid.parts {
            let part = world.parts.try_get(*id).unwrap();
            ensure!(part.grid_id == *grid_id);
        }
    }

    for (id, part) in world.parts.iter() {
        let grid = world.grids.try_get(part.grid_id).unwrap();
        ensure!(grid.parts.contains(id));
    }

    Ok(())
}

pub fn mass_of_grids_is_accurate(world: &World) -> Result<(), anyhow::Error> {
    for (grid_id, grid) in world.grids.iter() {
        let expected = get_grid_physical_props_by_id(*grid_id, &world.grids, &world.parts).unwrap();
        let actual = (grid.parts_mass, grid.center_of_mass);
        ensure!(actual.0 == expected.0, "Mass is not as expected");
        ensure!(actual.1 == expected.1, "COM is not as expected");
    }

    Ok(())
}

pub fn grid_parts_do_not_intersect(world: &World) -> Result<(), anyhow::Error> {
    for grid in world.grids.values() {
        let mut expected_cells = 0;
        for part_id in &grid.parts {
            let part = world.parts.try_get(*part_id).unwrap();
            let n_cells = part.region.cell_count();
            expected_cells += n_cells;
        }

        let mut actual_cells = 0;
        for occ in grid.occupancy.values() {
            actual_cells += occ.iter().count() as u32;
        }

        ensure!(expected_cells == actual_cells);
    }

    Ok(())
}

fn pipe_invariants(world: &World) -> Result<(), anyhow::Error> {
    for (id, pipe) in world.pipes.iter() {
        let part_a = world.parts.try_get(pipe.src.part_id);
        let part_b = world.parts.try_get(pipe.dst.part_id);
        ensure!(part_a.is_ok());
        ensure!(part_b.is_ok());
        let part_a = part_a?;
        let part_b = part_b?;
        ensure!(part_a.grid_id == part_b.grid_id);
        let grid = world.grids.try_get(part_a.grid_id);
        ensure!(grid.is_ok());
        let grid = grid?;
        ensure!(grid.pipes.contains(id));
    }

    Ok(())
}

pub fn is_world_consistent(world: &World) -> Result<(), anyhow::Error> {
    mass_of_grids_is_accurate(world)?;
    computer_pointers_are_consistent(world)?;
    part_bidirectional_pointers_are_consistent(world)?;
    grid_parts_do_not_intersect(world)?;
    pipe_invariants(world)?;

    Ok(())
}

pub fn assert_world_is_consistent(world: &World) {
    match is_world_consistent(world) {
        Ok(_) => (),
        Err(e) => {
            println!("assert_world_is_consistent failed: {:?}", e);
            assert!(false);
        }
    }
}
