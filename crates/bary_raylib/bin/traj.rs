use bary_core::prelude::*;
use bary_raylib::{Raylib, draw_circle};
use bary_sim::*;
use clap::Parser;
use raylib::color::Color;
use raylib::drawing::{RaylibDraw, RaylibDrawHandle};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Parser)]
struct Args {
    config: PathBuf,
}

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

fn load_sim_file(filename: impl AsRef<Path>) -> Result<SimSpec, Box<dyn std::error::Error>> {
    let s = std::fs::read_to_string(filename)?;
    let spec: SimSpec = serde_yaml::from_str(&s)?;
    Ok(spec)
}

#[derive(Debug)]
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

    for ti in instructions {
        cpu.enqueue_instruction(ti);
    }

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
            target_attitude: cpu.target_angle().map(|e| e.as_rad_f64()),
            acc_updates: world.grid_acceleration_updates,
            thrusters_firing,
        };

        ret.push(epoch);
    }

    ret
}

fn draw_simulation(d: &mut RaylibDrawHandle, epochs: &Vec<SimEpoch>) {
    let width = d.get_screen_width();
    let height = d.get_screen_height();
    let c = DVec2::new(width as f64 / 2.0, -height as f64 / 2.0);

    for epoch in epochs {
        if let Some(p) = epoch.target_position {
            draw_circle(d, c + p * 10.0, 10.0, Color::RED);
        }
    }

    for epoch in epochs {
        draw_circle(d, c + epoch.pose.tr() * 10.0, 2.0, Color::WHITE);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let mut raylib = Raylib::init();

    let spec = load_sim_file(args.config).unwrap();

    let epochs = run_simulation(
        &spec.ship_name,
        spec.instructions.clone(),
        spec.duration(),
        spec.ticks_per_epoch,
    );

    for epoch in &epochs {
        println!("{epoch:?}");
    }

    loop {
        raylib.draw(|mut handle| {
            handle.clear_background(Color::LIGHTBLUE.brightness(-0.6));
            draw_simulation(&mut handle, &epochs);
        });
    }
}
