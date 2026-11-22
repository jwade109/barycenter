use bary_ipc::{GridTelemetry, GridsTelemetry, WorldTelemetry};
use bary_sim::World;
use std::collections::BTreeMap;

pub fn make_world_telemetry(world: &World) -> WorldTelemetry {
    WorldTelemetry {
        tick: world.ticks,
        grids: world.grids.len(),
        planets: world.planets.len(),
        thrusters: world.thrusters.len(),
        parts: world.parts.len(),
        blueprints: world.blueprints.len(),
        next_id: world.spawner.next(),
    }
}

pub fn make_grid_telemetry(world: &World) -> GridsTelemetry {
    let mut grids = BTreeMap::new();

    for (id, grid) in world.grids.iter() {
        let mut info = BTreeMap::new();
        for thruster_id in &grid.thrusters {
            if let Some(thruster) = world.thrusters.get(*thruster_id) {
                info.insert(*thruster_id, thruster.unclamped_throttle());
            }
        }

        let mut active_thrusters = 0;

        for thruster_id in &grid.thrusters {
            let Some(thruster) = world.thrusters.get(*thruster_id) else {
                continue;
            };
            if thruster.unclamped_throttle() > 0.001 {
                active_thrusters += 1;
            }
        }

        let model = grid
            .blueprint
            .as_ref()
            .map(|bp| bp.0.clone())
            .unwrap_or_default();

        let tlm = GridTelemetry {
            name: grid.name.clone(),
            model,
            pos: grid.particle_location,
            vel: grid.velocity,
            active_thrusters,
        };

        grids.insert(*id, tlm);
    }

    GridsTelemetry {
        tick: world.ticks,
        grids,
    }
}
