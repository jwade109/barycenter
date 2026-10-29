use std::collections::VecDeque;

use bary_core::prelude::*;
use bary_factory::*;
use bary_orbital::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum Instruction {
    Ctrl(VehicleControl),
    HoldPosition(Isometry2d),
    HoldAttitude(Angle),
    PointAt(Vec2),
    Drift,
    DeltaV(Vec2),
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
}

impl Computer {
    pub fn new(prototype: Ent) -> Self {
        Self {
            on: true,
            status: MachineStatus::Off,
            ticks_this_cycle: 0,
            ticks_per_cycle: 5,
            fired_this_tick: false,
            ticks: 0,
            vehicle_control: VehicleControl::NULLOPT,
            prototype,
            instructions: VecDeque::new(),
        }
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

    pub fn enqueue_instruction(&mut self, ti: TimedInstruction) {
        self.instructions.push_back(ti);
    }

    pub fn extend_instruction_queue(
        &mut self,
        commands: impl IntoIterator<Item = TimedInstruction>,
    ) {
        self.instructions.extend(commands);
    }

    pub fn clear_instruction_queue(&mut self) {
        self.instructions.clear();
    }

    pub fn instructions(&self) -> impl Iterator<Item = &TimedInstruction> {
        self.instructions.iter()
    }

    pub fn tick_forward(&mut self) {
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
            }
            if let Some(ins) = self.instructions.front_mut() {
                ins.tick();
                if ins.is_complete() {
                    self.instructions.remove(0);
                }
            }
        } else {
            self.fired_this_tick = false;
        }
    }

    pub fn current_instruction(&self) -> Option<Instruction> {
        let cmd = self.instructions.front()?;
        Some(cmd.instruction)
    }

    pub fn current_instruction_mut(&mut self) -> Option<&mut Instruction> {
        let cmd = self.instructions.front_mut()?;
        Some(&mut cmd.instruction)
    }

    pub fn control_from_instruction(&self) -> Option<VehicleControl> {
        let cmd = self.instructions.front()?;
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
        let cmd = self.instructions.front()?;

        match cmd.instruction {
            Instruction::HoldAttitude(hdg) => Some(hdg),
            Instruction::HoldPosition(iso) => Some(Angle::radians(iso.rotation)),
            _ => None,
        }
    }

    pub fn delta_v(&self) -> Option<Vec2> {
        let cmd = self.current_instruction()?;
        match cmd {
            Instruction::DeltaV(dv) => Some(dv),
            _ => None,
        }
    }

    pub fn delta_v_mut(&mut self) -> Option<&mut Vec2> {
        let cmd = self.current_instruction_mut()?;
        match cmd {
            Instruction::DeltaV(dv) => Some(dv),
            _ => None,
        }
    }

    pub fn current_waypoint(&self) -> Option<Isometry2d> {
        let cmd = self.instructions.front()?;
        if let Instruction::HoldPosition(wp) = cmd.instruction {
            Some(wp)
        } else {
            None
        }
    }

    pub fn toggle(&mut self) {
        self.on = !self.on;
    }
}
