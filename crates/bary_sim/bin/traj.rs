use std::fs::File;
use std::io::Write;

use bary_core::prelude::*;
use bary_sim::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SimSpec {
    ship_name: String,
    instructions: Vec<TimedInstruction>,
    ticks_per_epoch: u64,
}

impl SimSpec {
    fn duration(&self) -> u64 {
        let mut sum = 0;
        for instr in &self.instructions {
            sum += instr.duration.unwrap_or(0);
        }
        sum + 1000
    }
}

fn load_sim_file(filename: &str) -> Result<SimSpec, Box<dyn std::error::Error>> {
    let s = std::fs::read_to_string(filename)?;
    let spec: SimSpec = serde_yaml::from_str(&s)?;
    Ok(spec)
}

struct SimEpoch {
    ticks: u64,
    pose: Isometry2d,
    vel: Isometry2d,
    target_position: Option<DVec2>,
    target_attitude: Option<f64>,
    acc_updates: u64,
    thrusters_firing: u32,
}

fn run_simulation(
    bp_name: &str,
    instructions: Vec<TimedInstruction>,
    ticks: u64,
    ticks_per_epoch: u64,
) -> Vec<SimEpoch> {
    let mut world = WorldBuilder::new()
        .assets()
        .blueprint(bp_name)
        .spawn(bp_name, "simba", (0.0, 0.0, 0.0))
        .build();

    let grid_id = get_grid_by_name(&world.grids, "simba").unwrap();

    _ = set_primary_computer_state(grid_id, true, &mut world);

    let cpu_id = get_primary_cpu_id(grid_id, &world.grids).unwrap();

    let cpu = world.computers.try_get_mut(cpu_id).unwrap();

    cpu.extend_instruction_queue(instructions);

    let mut ret = Vec::new();

    for _ in 0..ticks {
        update_world(&mut world);

        if ticks % ticks_per_epoch > 0 {
            continue;
        }

        let cpu_id = get_primary_cpu_id(grid_id, &world.grids).unwrap();
        let cpu = world.computers.try_get(cpu_id).unwrap();
        let mut pose = grid_pose(&world.grids, grid_id).unwrap();
        let vel = grid_vel(&world.grids, grid_id).unwrap();

        pose.rotation = Angle::radians(pose.rotation).as_rad_f64();

        let thrusters = get_thruster_levels(grid_id, &world.grids, &world.thrusters).unwrap();

        let thrusters_firing = thrusters.into_iter().map(|e| e.1 as u32).sum();

        let epoch = SimEpoch {
            ticks: world.ticks,
            pose,
            vel,
            target_position: cpu.current_waypoint().map(|e| e.translation),
            target_attitude: cpu.current_angle().map(|e| e.as_rad_f64()),
            acc_updates: world.grid_acceleration_updates,
            thrusters_firing,
        };

        ret.push(epoch);
    }

    ret
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();

    let filename = args.get(1).unwrap();

    let spec = load_sim_file(filename).unwrap();

    let epochs = run_simulation(
        &spec.ship_name,
        spec.instructions.clone(),
        spec.duration(),
        spec.ticks_per_epoch,
    );

    let mut file = File::create("sim.csv").unwrap();

    write!(file, "ticks,x,y,a,vx,vy,va,tx,ty,ta,updates,thrusters\n")?;

    for epoch in epochs {
        write!(
            file,
            "{},{:0.3},{:0.3},{:0.3},{:0.3},{:0.3},{:0.3},{:0.3},{:0.3},{:0.3},{},{}\n",
            epoch.ticks,
            epoch.pose.translation.x,
            epoch.pose.translation.y,
            epoch.pose.rotation,
            epoch.vel.translation.x,
            epoch.vel.translation.y,
            epoch.vel.rotation,
            epoch.target_position.map(|t| t.x).unwrap_or(std::f64::NAN),
            epoch.target_position.map(|t| t.y).unwrap_or(std::f64::NAN),
            epoch.target_attitude.unwrap_or(std::f64::NAN),
            epoch.acc_updates,
            epoch.thrusters_firing,
        )?;
    }

    Ok(())
}
