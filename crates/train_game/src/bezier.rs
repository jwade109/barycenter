use bary_core::prelude::{Isometry2d, linspace_f64};
use glam::DVec2;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
enum BezierOrder {
    Linear([DVec2; 2]),
    Quadratic([DVec2; 3]),
    Cubic([DVec2; 4]),
}

fn linear_bezier(a: DVec2, b: DVec2, t: f64) -> Isometry2d {
    let p = a.lerp(b, t);
    let angle = (b - a).to_angle();
    (p, angle).into()
}

fn quadratic_bezier(a: DVec2, b: DVec2, c: DVec2, t: f64) -> Isometry2d {
    let p = (1.0 - t).powi(2) * a + 2.0 * (1.0 - t) * t * b + t.powi(2) * c;
    let p_prime = 2.0 * (1.0 - t) * (b - a) + 2.0 * t * (c - b);
    (p, p_prime.to_angle()).into()
}

fn cubic_bezier(a: DVec2, b: DVec2, c: DVec2, d: DVec2, t: f64) -> Isometry2d {
    let p = (1.0 - t).powi(3) * a
        + 3.0 * t * (1.0 - t).powi(2) * b
        + 3.0 * t.powi(2) * (1.0 - t) * c
        + t.powi(3) * d;

    let n1 = 3.0 * (1.0 - t).powi(2) * (b - a);
    let n2 = 6.0 * (1.0 - t) * t * (c - b);
    let n3 = 3.0 * t.powi(2) * (d - c);
    let p_prime = n1 + n2 + n3;

    (p, p_prime.to_angle()).into()
}

impl BezierOrder {
    pub fn iter(&self) -> impl Iterator<Item = &DVec2> {
        match self {
            BezierOrder::Linear(v) => v.iter(),
            BezierOrder::Quadratic(v) => v.iter(),
            BezierOrder::Cubic(v) => v.iter(),
        }
    }

    pub fn eval(&self, t: f64) -> Isometry2d {
        match self {
            BezierOrder::Linear(v) => linear_bezier(v[0], v[1], t),
            BezierOrder::Quadratic(v) => quadratic_bezier(v[0], v[1], v[2], t),
            BezierOrder::Cubic(v) => cubic_bezier(v[0], v[1], v[2], v[3], t),
        }
    }

    pub fn start(&self) -> DVec2 {
        match self {
            BezierOrder::Linear(v) => v[0],
            BezierOrder::Quadratic(v) => v[0],
            BezierOrder::Cubic(v) => v[0],
        }
    }

    pub fn end(&self) -> DVec2 {
        match self {
            BezierOrder::Linear(v) => v[1],
            BezierOrder::Quadratic(v) => v[2],
            BezierOrder::Cubic(v) => v[3],
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BezierCurve {
    order: BezierOrder,
}

pub fn nearest_point_segment(a: DVec2, b: DVec2, p: DVec2) -> (f64, DVec2) {
    let v = b - a;
    let u = p - a;
    let t = (v.dot(u)) / (v.dot(v));
    let t = t.clamp(0.0, 1.0);
    (t, a + t * v)
}

impl BezierCurve {
    pub fn new(points: Vec<DVec2>) -> Option<Self> {
        let order = match points.len() {
            2 => BezierOrder::Linear([points[0], points[1]]),
            3 => BezierOrder::Quadratic([points[0], points[1], points[2]]),
            4 => BezierOrder::Cubic([points[0], points[1], points[2], points[3]]),
            _ => return None,
        };

        Some(Self { order })
    }

    pub fn points(&self) -> impl Iterator<Item = &DVec2> {
        self.order.iter()
    }

    pub fn linestring(&self, tmin: f64, tmax: f64, n: usize) -> Vec<Isometry2d> {
        if self.is_linear() {
            vec![self.eval(0.0), self.eval(1.0)]
        } else {
            linspace_f64(tmin, tmax, n)
                .iter()
                .map(|t| self.eval(*t))
                .collect()
        }
    }

    pub fn length(&self) -> f64 {
        let ls = self.linestring(0.0, 1.0, 30);
        ls.windows(2)
            .map(|p| {
                let a = p[0].tr();
                let b = p[1].tr();
                a.distance(b)
            })
            .sum()
    }

    pub fn eval(&self, t: f64) -> Isometry2d {
        self.order.eval(t)
    }

    pub fn is_linear(&self) -> bool {
        matches!(self.order, BezierOrder::Linear(_))
    }

    pub fn start(&self) -> DVec2 {
        self.order.start()
    }

    pub fn end(&self) -> DVec2 {
        self.order.end()
    }

    pub fn nearest_point(&self, p: DVec2) -> (f64, DVec2) {
        match self.order {
            BezierOrder::Linear(v) => nearest_point_segment(v[0], v[1], p),
            BezierOrder::Quadratic(v) => nearest_point_segment(v[0], v[2], p),
            BezierOrder::Cubic(v) => nearest_point_segment(v[0], v[3], p),
        }
    }
}
