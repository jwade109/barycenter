use super::math::*;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Copy, Deserialize, Serialize)]
pub struct AABB {
    pub center: Vec2,
    pub span: Vec2,
}

impl AABB {
    pub const UNIT: Self = Self {
        center: Vec2::splat(0.5),
        span: Vec2::splat(1.0),
    };

    pub fn new(center: impl Into<DVec2>, span: impl Into<DVec2>) -> Self {
        let center = center.into();
        let span = span.into();
        Self {
            center: center.as_vec2(),
            span: span.as_vec2(),
        }
    }

    pub fn unit() -> Self {
        Self::from_arbitrary((0.0, 0.0), (1.0, 1.0))
    }

    pub fn with_center(&self, c: Vec2) -> Self {
        Self {
            center: c,
            span: self.span,
        }
    }

    pub fn with_padding(pad: f32) -> Self {
        Self::new(Vec2::ZERO, Vec2::ZERO).padded(pad)
    }

    pub fn from_arbitrary(a: impl Into<Vec2>, b: impl Into<Vec2>) -> Self {
        let a: Vec2 = a.into();
        let b: Vec2 = b.into();
        let low = Vec2::new(a.x.min(b.x), a.y.min(b.y));
        let hi = Vec2::new(a.x.max(b.x), a.y.max(b.y));
        AABB::new((hi + low) / 2.0, hi - low)
    }

    pub fn from_list(plist: &[Vec2]) -> Option<Self> {
        let p0 = plist.get(0)?;
        let mut ret = AABB::new(*p0, Vec2::ZERO);
        for p in plist {
            ret.include(p)
        }
        Some(ret)
    }

    pub fn from_wh(w: f32, h: f32) -> AABB {
        let span = Vec2::new(w, h);
        AABB::new(Vec2::ZERO, span)
    }

    pub fn padded(&self, padding: f32) -> Self {
        let d = Vec2::new(padding, padding) / 2.0;
        AABB::new(self.center, self.span + d)
    }

    pub fn include(&mut self, p: &Vec2) {
        let l = self.lower();
        let u = self.upper();

        let a = Vec2::new(l.x.min(p.x), l.y.min(p.y));
        let b = Vec2::new(u.x.max(p.x), u.y.max(p.y));

        *self = AABB::from_arbitrary(a, b);
    }

    pub fn lower(&self) -> Vec2 {
        self.center - self.span / 2.0
    }

    pub fn top_left(&self) -> Vec2 {
        self.center + Vec2::new(-self.span.x / 2.0, self.span.y / 2.0)
    }

    pub fn top_center(&self) -> Vec2 {
        self.center + Vec2::Y * self.span.y / 2.0
    }

    pub fn bottom_center(&self) -> Vec2 {
        self.center - Vec2::Y * self.span.y / 2.0
    }

    pub fn bottom_right(&self) -> Vec2 {
        self.center + Vec2::new(self.span.x / 2.0, -self.span.y / 2.0)
    }

    pub fn upper(&self) -> Vec2 {
        self.center + self.span / 2.0
    }

    pub fn mid_left(&self) -> Vec2 {
        self.center - Vec2::X * self.span.x / 2.0
    }

    pub fn corners(&self) -> [Vec2; 4] {
        let c = self.center;
        let ur = self.span / 2.0;
        let ul = Vec2::new(-ur.x, ur.y);
        let ll = Vec2::new(-ur.x, -ur.y);
        let lr = Vec2::new(ur.x, -ur.y);
        [c + ur, c + lr, c + ll, c + ul]
    }

    pub fn offset(&self, d: Vec2) -> Self {
        AABB::new(self.center + d, self.span)
    }

    pub fn to_normalized(&self, p: Vec2) -> Vec2 {
        let lower = self.center - self.span / 2.0;
        let u = p - lower;
        u / self.span
    }

    pub fn from_normalized(&self, u: Vec2) -> Vec2 {
        let lower = self.center - self.span / 2.0;
        u * self.span + lower
    }

    pub fn map(&self, to: Self, p: Vec2) -> Vec2 {
        let u = self.to_normalized(p);
        to.from_normalized(u)
    }

    pub fn map_box(&self, to: Self, b: AABB) -> AABB {
        let u = self.map(to, b.lower());
        let v = self.map(to, b.upper());
        AABB::from_arbitrary(u, v)
    }

    pub fn contains(&self, p: Vec2) -> bool {
        let u = self.to_normalized(p);
        0.0 <= u.x && u.x <= 1.0 && 0.0 <= u.y && u.y <= 1.0
    }

    pub fn intersects(&self, other: &AABB) -> bool {
        let amin = self.lower();
        let amax = self.upper();
        let bmin = other.lower();
        let bmax = other.upper();
        if amax.x < bmin.x {
            return false;
        }
        if amin.x >= bmax.x {
            return false;
        }
        if amax.y < bmin.y {
            return false;
        }
        if amin.y >= bmax.y {
            return false;
        }
        true
    }

    pub fn scale(&self, scalar: f32) -> Self {
        AABB::new(self.center * scalar, self.span * scalar)
    }

    pub fn scale_about_center(&self, scalar: f32) -> Self {
        AABB::new(self.center, self.span * scalar)
    }

    pub fn rotate_about(&self, p: Vec2, angle: f32) -> OBB {
        let d = rotate(self.center - p, angle) + p;
        OBB::new(AABB::new(d, self.span), angle)
    }

    pub fn flip_y_about(&self, y: f32) -> Self {
        let dy = self.center.y - y;
        let center = self.center.with_y(self.center.y - 2.0 * dy);
        AABB::new(center, self.span)
    }

    pub fn polygon(&self) -> Polygon {
        Polygon::from_slice(&self.corners())
    }

    pub fn uniform_sample(&self) -> Vec2 {
        let sx = rand(0.0, 1.0);
        let sy = rand(0.0, 1.0);
        self.from_normalized(Vec2::new(sx, sy))
    }
}

impl From<((f32, f32), (f32, f32))> for AABB {
    fn from(value: ((f32, f32), (f32, f32))) -> Self {
        AABB::from_arbitrary(value.0, value.1)
    }
}

pub fn range_intersects(a: (f32, f32), b: (f32, f32)) -> bool {
    (a.0 <= b.0 && b.0 <= a.1) || (b.0 <= a.0 && a.0 <= b.1)
}

#[derive(Debug, Default, Clone, Copy)]
pub struct OBB(pub AABB, pub f32);

impl OBB {
    pub fn new(aabb: impl Into<AABB>, angle: f32) -> Self {
        OBB(aabb.into(), angle)
    }

    pub fn with_aabb(&self, aabb: impl Into<AABB>) -> Self {
        Self(aabb.into(), self.1)
    }

    pub fn offset(&self, d: Vec2) -> Self {
        OBB::new(AABB::new(self.0.center + d, self.0.span), self.1)
    }

    pub fn aabb(&self) -> AABB {
        match AABB::from_list(&self.corners()) {
            Some(aabb) => aabb,
            None => unreachable!(),
        }
    }

    pub fn corners(&self) -> [Vec2; 4] {
        let c = self.0.center;
        let mut corners = self.0.corners();

        for p in &mut corners {
            *p = rotate(*p - c, self.1) + c;
        }

        corners
    }

    pub fn polygon(&self) -> Polygon {
        let c = self.corners();
        Polygon::from_slice(&[c[0], c[1], c[2], c[3], c[0]])
    }

    pub fn axes(&self) -> (Vec2, Vec2) {
        (rotate(Vec2::X, self.1), rotate(Vec2::Y, self.1))
    }

    pub fn project_onto(&self, axis: Vec2) -> (f32, f32) {
        let x = self
            .corners()
            .map(|e: Vec2| e.dot(axis.normalize_or_zero()));

        let cmp = |f: &&f32, y: &&f32| f.total_cmp(y);

        let min = x.iter().min_by(cmp).unwrap();
        let max = x.iter().max_by(cmp).unwrap();

        (*min, *max)
    }

    pub fn _intersects(&self, other: OBB) -> bool {
        let a1 = self.axes();
        let a2 = other.axes();

        [a1.0, a1.1, a2.0, a2.1].into_iter().all(|axis| {
            let range_a = self.project_onto(axis);
            let range_b = other.project_onto(axis);
            return range_intersects(range_a, range_b);
        })
    }

    pub fn intersects(&self, other: OBB) -> Option<Vec2> {
        for c in self.corners() {
            if other.contains(c) {
                return Some(c);
            }
        }
        for c in other.corners() {
            if self.contains(c) {
                return Some(c);
            }
        }
        None
    }

    pub fn contains(&self, p: Vec2) -> bool {
        let d = p - self.0.center;
        let t = rotate(d, -self.1) + self.0.center;
        self.0.contains(t)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Polygon(Vec<Vec2>);

impl Polygon {
    pub fn new(points: Vec<Vec2>) -> Self {
        Polygon(points)
    }

    pub fn circle(center: impl Into<Vec2>, r: f32, n: usize) -> Self {
        let center = center.into();
        let mut points: Vec<_> = linspace(0.0, PI * 2.0, n + 1)
            .into_iter()
            .map(|a| rotate(Vec2::X * r, a) + center)
            .collect();
        points.pop();
        Polygon(points)
    }

    pub fn from_slice(points: &[Vec2]) -> Self {
        Polygon(points.to_owned())
    }

    pub fn iter(&self) -> impl Iterator<Item = &Vec2> + use<'_> {
        self.0.iter()
    }

    pub fn iter_closed(&self) -> impl Iterator<Item = &Vec2> + use<'_> {
        self.0.iter().chain(self.0.get(0))
    }

    pub fn open(&self) -> &Vec<Vec2> {
        &self.0
    }

    pub fn rotate_about(&self, o: Vec2, angle: f32) -> Self {
        let rotated = self.0.iter().map(|p| rotate(p - o, angle) + o).collect();
        Self(rotated)
    }

    pub fn closed(&self) -> Vec<Vec2> {
        let mut points = self.0.clone();
        if !points.is_empty() {
            points.push(points[0]);
        }
        points
    }
}
