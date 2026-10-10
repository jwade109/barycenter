use crate::math::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, PartialEq, Clone, Copy, Deserialize, Serialize)]
pub struct Isometry2d {
    pub translation: DVec2,
    pub rotation: f64,
}

impl Isometry2d {
    pub const ZERO: Self = Self {
        translation: DVec2::ZERO,
        rotation: 0.0,
    };

    pub fn new(translation: impl Into<DVec2>, rotation: impl Into<f64>) -> Self {
        Self {
            translation: translation.into(),
            rotation: rotation.into(),
        }
    }

    pub fn from_xya(x: f64, y: f64, a: f64) -> Self {
        Self {
            translation: DVec2::new(x, y),
            rotation: a,
        }
    }

    pub fn from_pos(pos: impl Into<DVec2>) -> Self {
        Self {
            translation: pos.into(),
            rotation: 0.0,
        }
    }

    pub fn local_x(&self) -> DVec2 {
        rotate_f64(DVec2::X, self.rotation)
    }

    pub fn local_y(&self) -> DVec2 {
        rotate_f64(DVec2::Y, self.rotation)
    }

    pub fn offset(&self, offset: impl Into<DVec2>) -> Self {
        let offset = offset.into();
        let mut ret = *self;
        ret.translation += ret.local_x() * offset.x + ret.local_y() * offset.y;
        ret
    }

    pub fn with_rotation(mut self, rotation: f64) -> Self {
        self.rotation = rotation;
        self
    }

    pub fn to_tuple(&self) -> (f64, f64, f64) {
        (self.translation.x, self.translation.y, self.rotation)
    }

    pub fn in_frame(&self, p: DVec2) -> DVec2 {
        let offset = p - self.translation;
        let yaw = self.rotation as f64;
        rotate_f64(offset, -yaw)
    }

    pub fn tr(&self) -> DVec2 {
        self.translation
    }
}

impl std::ops::Mul for Isometry2d {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        let mut ret = self.offset(rhs.translation);
        ret.rotation += rhs.rotation;
        ret
    }
}

impl std::ops::Mul<f64> for Isometry2d {
    type Output = Self;
    fn mul(mut self, rhs: f64) -> Self::Output {
        self.translation *= rhs;
        self.rotation *= rhs;
        self
    }
}

impl std::ops::Add for Isometry2d {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(
            self.translation + rhs.translation,
            self.rotation + rhs.rotation,
        )
    }
}

impl std::ops::AddAssign for Isometry2d {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl From<(f32, f32, f32)> for Isometry2d {
    fn from((x, y, r): (f32, f32, f32)) -> Self {
        Self {
            translation: DVec2::new(x.into(), y.into()),
            rotation: r.into(),
        }
    }
}

impl From<(f64, f64, f64)> for Isometry2d {
    fn from((x, y, r): (f64, f64, f64)) -> Self {
        Self {
            translation: DVec2::new(x, y),
            rotation: r,
        }
    }
}

impl From<(f64, f64)> for Isometry2d {
    fn from((x, y): (f64, f64)) -> Self {
        Self {
            translation: DVec2::new(x, y),
            rotation: 0.0,
        }
    }
}

impl From<(Vec2, f32)> for Isometry2d {
    fn from((translation, rotation): (Vec2, f32)) -> Self {
        Self {
            translation: translation.into(),
            rotation: rotation.into(),
        }
    }
}

impl From<Vec2> for Isometry2d {
    fn from(translation: Vec2) -> Self {
        Self {
            translation: translation.into(),
            rotation: 0.0,
        }
    }
}

impl From<DVec2> for Isometry2d {
    fn from(translation: DVec2) -> Self {
        Self {
            translation,
            rotation: 0.0,
        }
    }
}

impl From<(DVec2, f64)> for Isometry2d {
    fn from((translation, rotation): (DVec2, f64)) -> Self {
        Self {
            translation,
            rotation,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isometry_mul() {
        let a = Isometry2d::new((2.3, 4.0), 0.1);
        let b = Isometry2d::new((0.5, -3.4), 0.6);
        let c = Isometry2d::new((-0.4, 12.1), -0.9);

        let iso = a * b * c;

        assert_eq!(iso.translation, (-4.9640365, 9.663805).into());
        assert_eq!(iso.rotation, -0.19999993);
    }
}
