#![deny(missing_docs)]

use crate::prelude::*;
use serde::{Deserialize, Serialize};

/// The number of part grid cells in a meter of length.
/// One square meter contains 16 cells.
/// a 4x4 part is one meter in length on each side.
/// TODO reduce scope of this constant
pub const GRID_CELLS_PER_METER: f32 = 4.0;

/// A part grid coordinate. Can represent either the corner of
/// a grid cell, or the grid cell itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct PartCoord(pub IVec2);

impl PartCoord {
    /// The width of a single grid cell in meters.
    pub const CELL_WIDTH: f32 = 1.0 / GRID_CELLS_PER_METER;

    /// The coordinate (1, 1).
    pub const ONE: Self = Self(IVec2::ONE);

    /// The coordinate (0, 0).
    pub const ZERO: Self = Self(IVec2::ZERO);

    /// Constructs a new PartCoord from an IVec2-like thing.
    pub fn new(p: impl Into<IVec2>) -> Self {
        Self(p.into())
    }

    /// Convert this PartCoord into a tuple.
    pub fn to_tuple(&self) -> (i32, i32) {
        (self.0.x, self.0.y)
    }

    /// Convert this PartCoord into the equivalent floating point
    /// coordinate in meters. This gets the position of the bottom left corner
    /// of this grid cell. For the center of this grid cell,
    /// try [Self::to_meters_center].
    ///
    /// ```
    /// use bary_core::prelude::*;
    ///
    /// let x = PartCoord::new((3, 5));
    /// assert_eq!(x.to_meters(), Vec2::new(0.75, 1.25));
    ///
    /// let x = PartCoord::new((-7, -6));
    /// assert_eq!(x.to_meters(), Vec2::new(-1.75, -1.5));
    /// ```
    pub fn to_meters(&self) -> Vec2 {
        self.0.as_vec2() / GRID_CELLS_PER_METER
    }

    /// Gets the coordinate, in meters, of the center of this
    /// grid cell. For the bottom left corner of this grid cell,
    /// try [Self::to_meters].
    ///
    /// ```
    /// use bary_core::prelude::*;
    ///
    /// let x = PartCoord::new((3, 5));
    /// assert_eq!(x.to_meters_center(), Vec2::new(0.875, 1.375));
    ///
    /// let x = PartCoord::new((-7, -6));
    /// assert_eq!(x.to_meters_center(), Vec2::new(-1.625, -1.375));
    /// ```
    pub fn to_meters_center(&self) -> Vec2 {
        (self.0.as_vec2() + Vec2::splat(0.5)) / GRID_CELLS_PER_METER
    }

    /// Constructs a PartCoord from a floating point coordinate
    /// in meters by flooring that coordinate towards the bottom
    /// left corner. In effect, this function will map a real
    /// world coordinate into the cell which contains that coordinate.
    /// 
    /// ```
    /// use bary_core::prelude::*;
    ///
    /// let x = PartCoord::from_meters_floored((0.1, 0.2));
    /// assert_eq!(x, PartCoord::ZERO);
    ///
    /// let x = PartCoord::from_meters_floored((1.2, 0.6));
    /// assert_eq!(x, PartCoord::new((4, 2)));
    ///
    /// let x = PartCoord::from_meters_floored((-3.2, -8.9));
    /// assert_eq!(x, PartCoord::new((-13, -36)));
    /// ```
    pub fn from_meters_floored(p: impl Into<DVec2>) -> Self {
        Self(vfloor(p.into().as_vec2() * GRID_CELLS_PER_METER))
    }

    /// Gets the inner value of this PartCoord, as IVec2.
    pub fn inner(&self) -> IVec2 {
        self.0
    }

    /// Gets the AABB of this PartCoord, bounded by the lower
    /// and upper coordinate of the grid cell.
    pub fn to_aabb(&self) -> AABB {
        let lower = self.to_meters();
        let upper = (*self + PartCoord::new(IVec2::ONE)).to_meters();
        AABB::from_arbitrary(lower, upper)
    }

    /// Generate an iterator over all PartCoords which intersect
    /// the given AABB.
    pub fn in_aabb(aabb: AABB) -> impl Iterator<Item = Self> {
        let lower = Self::from_meters_floored(aabb.lower()).inner();
        let upper = Self::from_meters_floored(aabb.upper()).inner();

        (lower.x..=upper.x)
            .flat_map(move |x| (lower.y..=upper.y).map(move |y| IVec2::new(x, y)))
            .map(Self::new)
    }

    /// Rotate this PartCoord counterclockwise.
    pub fn rotated_ccw(&self) -> Self {
        let v = self.inner() + IVec2::Y;
        let v = IVec2::new(-v.y, v.x);
        v.into()
    }

    /// Remaps this coordinate in a oopy doopy way
    /// according to some rotation. TODO what is this even doing.
    pub fn origin_with(&self, rot: Rotation) -> Self {
        match rot {
            Rotation::East => *self,
            Rotation::North => *self + IVec2::X.into(),
            Rotation::West => *self + IVec2::ONE.into(),
            Rotation::South => *self + IVec2::Y.into(),
        }
    }
}

impl PartialOrd for PartCoord {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.to_tuple().partial_cmp(&other.to_tuple())
    }
}

impl Ord for PartCoord {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.to_tuple().cmp(&other.to_tuple())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotating_part_coords() {
        let v = PartCoord::ZERO.rotated_ccw();
        assert_eq!(v, (-1, 0).into());

        let v = v.rotated_ccw();
        assert_eq!(v, (-1, -1).into());

        let v = v.rotated_ccw();
        assert_eq!(v, (0, -1).into());

        let v = v.rotated_ccw();
        assert_eq!(v, (0, 0).into());

        let v = PartCoord::ONE.rotated_ccw();
        assert_eq!(v, (-2, 1).into());

        let v = v.rotated_ccw();
        assert_eq!(v, (-2, -2).into());

        let v = v.rotated_ccw();
        assert_eq!(v, (1, -2).into());

        let v = v.rotated_ccw();
        assert_eq!(v, (1, 1).into());
    }
}

impl std::ops::Add for PartCoord {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::AddAssign for PartCoord {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0
    }
}

impl std::ops::Sub for PartCoord {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl From<(i32, i32)> for PartCoord {
    fn from(val: (i32, i32)) -> Self {
        PartCoord(IVec2::new(val.0, val.1))
    }
}

impl From<IVec2> for PartCoord {
    fn from(val: IVec2) -> Self {
        PartCoord(val)
    }
}

impl From<UVec2> for PartCoord {
    fn from(val: UVec2) -> Self {
        PartCoord(val.as_ivec2())
    }
}

impl std::fmt::Display for PartCoord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.0.x, self.0.y)
    }
}
