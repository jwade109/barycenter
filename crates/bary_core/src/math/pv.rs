use crate::math::{DVec2, Vec2};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize, Serialize)]
pub struct PV {
    pub pos: DVec2,
    pub vel: DVec2,
}

impl PV {
    pub const NAN: PV = PV {
        pos: DVec2::NAN,
        vel: DVec2::NAN,
    };

    pub const INFINITY: PV = PV {
        pos: DVec2::INFINITY,
        vel: DVec2::INFINITY,
    };

    pub const ZERO: PV = PV {
        pos: DVec2::ZERO,
        vel: DVec2::ZERO,
    };

    #[deprecated]
    pub fn pos_f32(&self) -> Vec2 {
        self.pos.as_vec2()
    }

    #[deprecated]
    pub fn vel_f32(&self) -> Vec2 {
        self.vel.as_vec2()
    }

    #[deprecated]
    pub fn from_f32(pos: impl Into<Vec2>, vel: impl Into<Vec2>) -> Self {
        PV {
            pos: pos.into().as_dvec2(),
            vel: vel.into().as_dvec2(),
        }
    }

    pub fn from_f64(pos: impl Into<DVec2>, vel: impl Into<DVec2>) -> Self {
        PV {
            pos: pos.into(),
            vel: vel.into(),
        }
    }

    pub fn pos(pos: impl Into<DVec2>) -> Self {
        PV::from_f64(pos.into(), DVec2::ZERO)
    }

    pub fn vel(vel: impl Into<DVec2>) -> Self {
        PV::from_f64(DVec2::ZERO, vel.into())
    }

    pub fn filter_numerr(&self) -> Option<Self> {
        if !self.pos.is_finite() || !self.vel.is_finite() {
            None
        } else {
            Some(*self)
        }
    }

    pub fn is_zero(&self) -> bool {
        self.pos == DVec2::ZERO && self.vel == DVec2::ZERO
    }
}

pub fn distance_str(x: f64) -> String {
    if x.abs() > 1000000.0 {
        format!("{:0.2} Mm", x / 1000000.0)
    } else if x.abs() > 1000.0 {
        format!("{:0.2} km", x / 1000.0)
    } else if x.abs() > 0.9 {
        format!("{:0.1} m", x)
    } else {
        format!("{:0.1} mm", x * 1000.0)
    }
}

pub fn velocity_str(x: f64) -> String {
    if x.abs() > 1000.0 {
        format!("{:0.2} km/s", x / 1000.0)
    } else {
        format!("{:0.1} m/s", x)
    }
}

pub fn distance_str_v(v: DVec2) -> String {
    format!("({}, {})", distance_str(v.x), distance_str(v.y))
}

pub fn velocity_str_v(v: DVec2) -> String {
    format!("({}, {})", velocity_str(v.x), velocity_str(v.y))
}

impl std::fmt::Display for PV {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({}, {}), ({}, {})",
            distance_str(self.pos.x),
            distance_str(self.pos.y),
            velocity_str(self.vel.x),
            velocity_str(self.vel.y)
        )
    }
}

impl std::ops::Add for PV {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        PV::from_f64(self.pos + other.pos, self.vel + other.vel)
    }
}

impl std::ops::Sub for PV {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        PV::from_f64(self.pos - other.pos, self.vel - other.vel)
    }
}

impl std::ops::Div<f32> for PV {
    type Output = Self;
    fn div(self, rhs: f32) -> Self::Output {
        PV::from_f64(self.pos / rhs as f64, self.vel / rhs as f64)
    }
}

impl std::ops::Div<f64> for PV {
    type Output = Self;
    fn div(self, rhs: f64) -> Self::Output {
        PV::from_f64(self.pos / rhs, self.vel / rhs)
    }
}

impl std::ops::Mul<f32> for PV {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self::Output {
        PV::from_f64(self.pos * rhs as f64, self.vel * rhs as f64)
    }
}

impl std::ops::Mul<f64> for PV {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self::Output {
        PV::from_f64(self.pos * rhs, self.vel * rhs)
    }
}

impl std::ops::AddAssign for PV {
    fn add_assign(&mut self, rhs: Self) {
        self.pos += rhs.pos;
        self.vel += rhs.vel;
    }
}

impl From<((f64, f64), (f64, f64))> for PV {
    fn from(val: ((f64, f64), (f64, f64))) -> Self {
        let r: DVec2 = val.0.into();
        let v: DVec2 = val.1.into();
        PV::from_f64(r, v)
    }
}

impl From<(DVec2, DVec2)> for PV {
    fn from(val: (DVec2, DVec2)) -> Self {
        PV::from_f64(val.0, val.1)
    }
}
