use crate::math::wrap_pi_npi;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Angle(f32);

impl Angle {
    pub fn degrees(a: f32) -> Self {
        Self::radians(a.to_radians())
    }

    pub fn radians(a: impl Into<f64>) -> Self {
        Self(wrap_pi_npi(a.into() as f32))
    }

    pub fn as_rad(&self) -> f32 {
        self.0
    }

    pub fn as_rad_f64(&self) -> f64 {
        self.0 as f64
    }

    pub fn as_deg(&self) -> f32 {
        self.0.to_degrees()
    }
}

impl std::fmt::Display for Angle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}°", self.as_deg())
    }
}

impl std::ops::Add for Angle {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self::radians(self.0 + rhs.0)
    }
}

impl std::ops::Sub for Angle {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self::radians(self.0 - rhs.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addition() {
        let a = Angle::degrees(12.382);
        let b = Angle::radians(0.3);
        let c = a + b;

        println!("{} + {} = {}", a, b, c);

        let d = Angle::degrees(450.0);

        let e = c + d;

        println!("{} + {} = {}", c, d, e);
    }

    #[test]
    fn subtraction_1() {
        let a = Angle::degrees(20.0);
        let b = Angle::degrees(14.3);
        let c = a - b;

        println!("{} - {} = {}", a, b, c);
    }

    #[test]
    fn subtraction_2() {
        let a = Angle::degrees(390.0);
        let b = Angle::degrees(410.0);
        let c = a - b;

        println!("{} - {} = {}", a, b, c);
    }
}
