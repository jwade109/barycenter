use bary_core::prelude::*;
use bary_factory::*;
use bary_orbital::*;
use log::error;
use serde::{Deserialize, Serialize};

use crate::*;

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
struct GridInfo {
    pos: Isometry2d,
    vel: Isometry2d,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Computer {
    on: bool,
    status: MachineStatus,
    ticks_this_cycle: u32,
    ticks_per_cycle: u32,
    fired_this_tick: bool,
    ticks: u64,
    vehicle_control: VehicleControl,
    prototype: Ent,
    thruster_odometry: DVec2,
    target_angle: Option<Angle>,
    target_pos: Option<DVec2>,
    target_vel: Option<DVec2>,
    target_odometry: Option<DVec2>,
    grid_info: Option<GridInfo>,
    attitude_pd: PDCtrl,
}

impl Computer {
    pub fn new(prototype: Ent) -> Self {
        Self {
            on: true,
            status: MachineStatus::Off,
            ticks_this_cycle: 0,
            ticks_per_cycle: 1,
            fired_this_tick: false,
            ticks: 0,
            vehicle_control: VehicleControl::NULLOPT,
            prototype,
            thruster_odometry: DVec2::ZERO,
            grid_info: None,
            attitude_pd: PDCtrl::new(30.0, 50.0),
            target_angle: None,
            target_pos: None,
            target_vel: None,
            target_odometry: None,
        }
    }

    pub fn update_grid_info(&mut self, grid: &VehicleGrid) {
        let info = GridInfo {
            pos: grid.particle_location,
            vel: grid.velocity,
        };

        self.grid_info = Some(info);
    }

    pub fn set_attitude_pd_coeffs(&mut self, kp: f64, kd: f64) {
        self.attitude_pd.kp = kp;
        self.attitude_pd.kd = kd;
    }

    pub fn is_on(&self) -> bool {
        self.on
    }

    pub fn fired_this_tick(&self) -> bool {
        self.fired_this_tick
    }

    pub fn status(&self) -> MachineStatus {
        self.status
    }

    pub fn ticks(&self) -> u64 {
        self.ticks
    }

    pub fn odometry(&self) -> DVec2 {
        self.thruster_odometry
    }

    pub fn get_current_control(&self) -> &VehicleControl {
        &self.vehicle_control
    }

    pub fn prototype(&self) -> Ent {
        self.prototype
    }

    pub fn set_state(&mut self, state: bool) {
        self.on = state;
    }

    pub fn set_target_angle(&mut self, angle: Angle) {
        self.target_angle = Some(angle);
    }

    pub fn set_target_pos(&mut self, pos: DVec2) {
        self.target_pos = Some(pos);
    }

    pub fn set_target_waypoint(&mut self, iso: impl Into<Isometry2d>) {
        let iso = iso.into();
        self.target_angle = Some(iso.rotation.into());
        self.target_pos = Some(iso.tr());
    }

    #[deprecated]
    pub fn enqueue_instruction(&mut self, ti: TimedInstruction) {
        match ti.instruction {
            Instruction::HoldAttitude(a) => {
                self.target_angle = Some(a);
            }
            Instruction::HoldPosition(p) => {
                self.target_angle = Some(p.rotation.into());
                self.target_pos = Some(p.tr());
                self.target_vel = None;
            }
            Instruction::HoldVelocity(v) => {
                self.target_vel = Some(v);
                self.target_pos = None;
            }
            Instruction::Drift => {
                self.target_angle = None;
                self.target_vel = None;
                self.target_pos = None;
            }
            Instruction::DeltaV(dv) => {
                self.target_odometry = Some(self.thruster_odometry + dv);
                self.target_pos = None;
                self.target_vel = None;
            }
            e => {
                error!("Bad instruction: {e:?}");
            }
        }
    }

    fn tick(&mut self) {
        let Some(info) = self.grid_info else {
            return;
        };

        if let Some(target_odometry) = self.target_odometry {
            let delta = target_odometry - self.thruster_odometry;
            self.do_velocity_navigation(info, info.vel.tr() + delta);
        } else if let Some(desired_vel) = self.target_vel {
            self.do_velocity_navigation(info, desired_vel);
        } else if let Some(target_pose) = self.current_waypoint() {
            self.do_waypoint_navigation(info, target_pose);
        } else if let Some(target) = self.target_angle() {
            self.do_heading_navigation(info, target);
        } else {
            self.vehicle_control = VehicleControl::NULLOPT;
        }
    }

    pub fn update(&mut self) {
        self.status = match self.on {
            true => MachineStatus::Running,
            false => MachineStatus::Off,
        };

        if self.on {
            self.ticks_this_cycle += 1;
            self.fired_this_tick = self.ticks_this_cycle == self.ticks_per_cycle;
            if self.fired_this_tick {
                self.ticks_this_cycle = 0;
                self.ticks += 1;
                self.tick();
            }
        } else {
            self.fired_this_tick = false;
        }
    }

    pub fn target_angle(&self) -> Option<Angle> {
        self.target_angle
    }

    pub fn target_pos(&self) -> Option<DVec2> {
        self.target_pos
    }

    pub fn current_waypoint(&self) -> Option<Isometry2d> {
        let p = self.target_pos?;
        let a = self.target_angle.unwrap_or(Angle::degrees(0.0));
        Some((p, a.as_rad_f64()).into())
    }

    pub fn report_dv(&mut self, applied_dv: DVec2) {
        self.thruster_odometry += applied_dv;
    }

    fn do_velocity_navigation(&mut self, info: GridInfo, desired_vel: DVec2) {
        let idle_angle = self.target_angle.unwrap_or_default().as_rad_f64();
        let actual_pv = PV::from_f64(info.pos.translation, info.vel.translation);

        let actual = Angle::radians(info.pos.rotation);
        let body = RigidBody {
            pv: actual_pv,
            angle: actual.as_rad() as f64,
            angular_velocity: info.vel.rotation as f64,
        };

        let (ctrl, _status) =
            zero_gravity_velocity_control_law(desired_vel, idle_angle, &body, &self.attitude_pd);

        self.vehicle_control = ctrl;
    }

    fn do_heading_navigation(&mut self, info: GridInfo, target: Angle) {
        let actual = Angle::radians(info.pos.rotation);
        let body = RigidBody {
            pv: PV::ZERO,
            angle: actual.as_rad() as f64,
            angular_velocity: info.vel.rotation as f64,
        };

        let (ctrl, _status) =
            attitude_control_law(target.as_rad() as f64, &self.attitude_pd, &body);

        self.vehicle_control = ctrl;
    }

    fn do_waypoint_navigation(&mut self, info: GridInfo, target: Isometry2d) {
        let target_pv = PV::from_f64(target.translation, Vec2::ZERO);
        let actual_pv = PV::from_f64(info.pos.translation, info.vel.translation);

        let body = RigidBody {
            pv: actual_pv,
            angle: info.pos.rotation as f64,
            angular_velocity: info.vel.rotation as f64,
        };

        let (ctrl, _status) = position_hold_control_law(
            target_pv,
            target.rotation,
            &body,
            DVec2::ZERO,
            &self.attitude_pd,
        );

        self.vehicle_control = ctrl;
    }

    pub fn toggle(&mut self) {
        self.on = !self.on;
    }
}
