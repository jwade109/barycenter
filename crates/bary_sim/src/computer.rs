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
    pub on: bool,
    pub status: MachineStatus,
    pub ticks_this_cycle: u32,
    pub ticks_per_cycle: u32,
    pub fired_this_tick: bool,
    pub iters: u64,
    pub vehicle_control: VehicleControl,
    pub prototype: Ent,
    pub command_queue: Vec<TimedInstruction>,
}

impl Computer {
    pub fn new(prototype: Ent) -> Self {
        Self {
            on: true,
            status: MachineStatus::Off,
            ticks_this_cycle: 0,
            ticks_per_cycle: 5,
            fired_this_tick: false,
            iters: 0,
            vehicle_control: VehicleControl::NULLOPT,
            prototype,
            command_queue: vec![
                // TimedInstruction::timed(3000, Instruction::rcs_forward()),
                // TimedInstruction::timed(1500, Instruction::rcs_right()),
                // TimedInstruction::timed(1500, Instruction::rcs_left()),
                // TimedInstruction::timed(800, Instruction::rcs_backward()),
                // TimedInstruction::timed(20000, Instruction::Drift),
            ],
        }
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
                self.iters += 1;
            }
            if let Some(ins) = self.command_queue.first_mut() {
                ins.tick();
                if ins.is_complete() {
                    self.command_queue.remove(0);
                }
            }
        } else {
            self.fired_this_tick = false;
        }
    }

    pub fn current_instruction(&self) -> Option<Instruction> {
        let cmd = self.command_queue.first()?;
        Some(cmd.instruction)
    }

    pub fn current_instruction_mut(&mut self) -> Option<&mut Instruction> {
        let cmd = self.command_queue.first_mut()?;
        Some(&mut cmd.instruction)
    }

    pub fn current_control(&self) -> Option<VehicleControl> {
        let cmd = self.command_queue.first()?;
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
        let cmd = self.command_queue.first()?;

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
        let cmd = self.command_queue.first()?;
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
