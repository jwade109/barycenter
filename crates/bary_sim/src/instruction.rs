use bary_core::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum Instruction {
    HoldPosition(Isometry2d),
    HoldAttitude(Angle),
    HoldVelocity(DVec2),
    PointAt(Vec2),
    Drift,
    DeltaV(DVec2),
    HoldRelativePosition(Ent, DVec2),
}

impl Instruction {
    pub fn rcs_left() -> Self {
        todo!()
        // Self::Ctrl(VehicleControl::rcs(false, true, false, false))
    }

    pub fn rcs_right() -> Self {
        todo!()
        // Self::Ctrl(VehicleControl::rcs(false, false, true, false))
    }

    pub fn rcs_forward() -> Self {
        todo!()
        // Self::Ctrl(VehicleControl::rcs(true, false, false, false))
    }

    pub fn rcs_backward() -> Self {
        todo!()
        // Self::Ctrl(VehicleControl::rcs(false, false, false, true))
    }
}

impl std::fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
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
            Instruction::HoldVelocity(vel) => {
                write!(f, "VEL {:0.2}", vel)
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
