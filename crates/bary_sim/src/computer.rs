use std::collections::VecDeque;

use bary_core::prelude::*;
use bary_factory::*;
use bary_orbital::*;
use log::info;
use serde::{Deserialize, Serialize};

use crate::{DynamicParams, VehicleGrid};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum Instruction {
    Ctrl(VehicleControl),
    HoldPosition(Isometry2d),
    HoldAttitude(Angle),
    PointAt(Vec2),
    Drift,
    DeltaV(DVec2),
    HoldRelativePosition(Ent, DVec2),
}

impl Instruction {
    pub fn rcs_left() -> Self {
        Self::Ctrl(VehicleControl::rcs(false, true, false, false))
    }

    pub fn rcs_right() -> Self {
        Self::Ctrl(VehicleControl::rcs(false, false, true, false))
    }

    pub fn rcs_forward() -> Self {
        Self::Ctrl(VehicleControl::rcs(true, false, false, false))
    }

    pub fn rcs_backward() -> Self {
        Self::Ctrl(VehicleControl::rcs(false, false, false, true))
    }
}

impl std::fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::Ctrl(_vehicle_control) => {
                write!(f, "CTRL")
            }
            Instruction::HoldPosition(iso) => {
                write!(
                    f,
                    "HP pos={:0.2} hdg={:0.2}",
                    iso.translation,
                    iso.rotation.to_degrees()
                )
            }
            Instruction::Drift => {
                write!(f, "DRIFT")
            }
            Instruction::HoldAttitude(angle) => {
                write!(f, "HDG {:0.2}", angle.as_deg())
            }
            Instruction::PointAt(pos) => {
                write!(f, "POINT {:0.2}", pos)
            }
            Instruction::DeltaV(dv) => {
                write!(f, "DV {:0.2}", dv)
            }
            Instruction::HoldRelativePosition(id, rel) => {
                write!(f, "REL {} {:0.2}", id, rel)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct TimedInstruction {
    pub duration: Option<u64>,
    pub instruction: Instruction,
}

impl TimedInstruction {
    pub fn perp(instruction: Instruction) -> Self {
        Self {
            duration: None,
            instruction,
        }
    }

    pub fn timed(ticks: u64, instruction: Instruction) -> Self {
        Self {
            duration: Some(ticks),
            instruction,
        }
    }

    pub fn tick(&mut self) {
        if let Some(dur) = &mut self.duration {
            if *dur > 0 {
                *dur -= 1;
            }
        }
    }

    pub fn is_complete(&self) -> bool {
        self.duration.map(|d| d == 0).unwrap_or(false)
    }
}

impl From<Instruction> for TimedInstruction {
    fn from(value: Instruction) -> Self {
        Self::perp(value)
    }
}

impl std::fmt::Display for TimedInstruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ticks) = self.duration {
            write!(f, "{}, for {} ticks", self.instruction, ticks)
        } else {
            write!(f, "{}", self.instruction)
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
enum ControllerState {
    Idle,
    Executing(TimedInstruction),
    DeltaV {
        initial_odometry: DVec2,
        final_odometry: DVec2,
        odometry_sum: DVec2,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Controller {
    state: ControllerState,
}

impl Controller {
    fn idle() -> Self {
        Self {
            state: ControllerState::Idle,
        }
    }

    fn start(ti: TimedInstruction) -> Self {
        Self {
            state: ControllerState::Executing(ti),
        }
    }

    fn update(&mut self) {
        match &mut self.state {
            ControllerState::Idle => (),
            ControllerState::Executing(ti) => {
                ti.tick();
            }
            ControllerState::DeltaV {
                initial_odometry: _,
                final_odometry: _,
                odometry_sum: _,
            } => todo!(),
        }
    }

    fn is_idle(&self) -> bool {
        matches!(self.state, ControllerState::Idle)
    }
}

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
    instructions: VecDeque<TimedInstruction>,
    thruster_odometry: DVec2,
    controller: Controller,
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
            instructions: VecDeque::new(),
            thruster_odometry: DVec2::ZERO,
            controller: Controller::idle(),
            grid_info: None,
            attitude_pd: PDCtrl::new(30.0, 50.0),
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

    pub fn set_current_control(&mut self, ctrl: VehicleControl) {
        self.vehicle_control = ctrl;
    }

    pub fn prototype(&self) -> Ent {
        self.prototype
    }

    pub fn set_state(&mut self, state: bool) {
        self.on = state;
    }

    fn execute(&mut self, ti: TimedInstruction) {
        info!("Beginning execution: {ti}");
        self.controller = Controller::start(ti);
    }

    pub fn current_instruction(&self) -> Option<&TimedInstruction> {
        if let ControllerState::Executing(ti) = &self.controller.state {
            Some(ti)
        } else {
            None
        }
    }

    pub fn enqueue_instruction(&mut self, ti: TimedInstruction) {
        self.instructions.push_back(ti);
    }

    pub fn clear_instruction_queue(&mut self) {
        self.instructions.clear();
        self.controller = Controller::idle();
    }

    pub fn instructions(&self) -> impl Iterator<Item = &TimedInstruction> {
        self.instructions.iter()
    }

    fn tick(&mut self, params: &mut DynamicParams) {
        let Some(info) = self.grid_info else {
            return;
        };

        if self.controller.is_idle()
            && let Some(ti) = self.instructions.pop_front()
        {
            self.execute(ti);
        } else {
            self.controller.update();
        }

        if let Some(ctrl) = self.control_from_instruction() {
            self.set_current_control(ctrl);
        } else if let Some(target_pose) = self.current_waypoint() {
            self.do_waypoint_navigation(info.pos, info.vel, target_pose);
        } else if let Some(target) = self.current_angle() {
            let actual = Angle::radians(info.pos.rotation);

            let body = RigidBody {
                pv: PV::ZERO,
                angle: actual.as_rad() as f64,
                angular_velocity: info.vel.rotation as f64,
            };

            let pid = PDCtrl::new(20.0, 30.0);

            let (ctrl, _status) = attitude_control_law(target.as_rad() as f64, &pid, &body);

            self.set_current_control(ctrl);
        }
    }

    pub fn update(&mut self, params: &mut DynamicParams) {
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
                self.tick(params);
            }
        } else {
            self.fired_this_tick = false;
        }
    }

    pub fn control_from_instruction(&self) -> Option<VehicleControl> {
        let cmd = self.current_instruction()?;
        if let Instruction::Ctrl(ctrl) = cmd.instruction {
            Some(ctrl)
        } else if let Instruction::Drift = cmd.instruction {
            Some(VehicleControl::NULLOPT)
        } else {
            None
        }
    }

    pub fn current_angle(&self) -> Option<Angle> {
        if let Some(wp) = self.current_waypoint() {
            return Some(Angle::radians(wp.rotation));
        }
        let cmd = self.current_instruction()?;

        match cmd.instruction {
            Instruction::HoldAttitude(hdg) => Some(hdg),
            Instruction::HoldPosition(iso) => Some(Angle::radians(iso.rotation)),
            _ => None,
        }
    }

    pub fn report_dv(&mut self, applied_dv: DVec2) {
        self.thruster_odometry += applied_dv;
    }

    pub fn current_waypoint(&self) -> Option<Isometry2d> {
        let cmd = self.current_instruction()?;
        if let Instruction::HoldPosition(wp) = cmd.instruction {
            Some(wp)
        } else {
            None
        }
    }

    fn do_waypoint_navigation(&mut self, ego: Isometry2d, ego_vel: Isometry2d, target: Isometry2d) {
        let target_pv = PV::from_f64(target.translation, Vec2::ZERO);
        let actual_pv = PV::from_f64(ego.translation, ego_vel.translation);

        let body = RigidBody {
            pv: actual_pv,
            angle: ego.rotation as f64,
            angular_velocity: ego_vel.rotation as f64,
        };

        let (ctrl, _status) = position_hold_control_law(
            target_pv,
            target.rotation,
            &body,
            DVec2::ZERO,
            &self.attitude_pd,
        );

        self.set_current_control(ctrl);
    }

    pub fn toggle(&mut self) {
        self.on = !self.on;
    }
}
