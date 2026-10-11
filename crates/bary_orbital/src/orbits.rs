use crate::{lookup_ta_from_ma, propagator::search_condition};
use bary_core::prelude::*;
use serde::{Deserialize, Serialize};

/// Per wikipedia: 6.67430(15)×10−11 m3⋅kg−1⋅s−2[1]
pub const GRAVITATIONAL_CONSTANT: f64 = 6.67430E-11;

pub fn hyperbolic_range_ta(ecc: f32) -> f32 {
    (-1.0 / ecc).acos()
}

#[derive(Clone, Copy, Debug)]
pub enum ApproachInfo<T: Clone + Copy> {
    NonIntersecting {
        nearest: T,
        farthest: T,
    },
    Intersecting {
        nearest: T,
        farthest: T,
        rising: T,
        falling: T,
    },
}

impl<T: Clone + Copy> ApproachInfo<T> {
    pub fn points(&self) -> Vec<T> {
        match *self {
            ApproachInfo::Intersecting {
                nearest,
                farthest,
                rising,
                falling,
            } => {
                vec![nearest, farthest, rising, falling]
            }
            ApproachInfo::NonIntersecting { nearest, farthest } => {
                vec![nearest, farthest]
            }
        }
    }

    pub fn intersections(&self) -> Option<(T, T)> {
        match self {
            ApproachInfo::Intersecting {
                rising, falling, ..
            } => Some((*rising, *falling)),
            _ => None,
        }
    }

    pub fn approaches(&self) -> (T, T) {
        match self {
            ApproachInfo::Intersecting {
                nearest, farthest, ..
            }
            | ApproachInfo::NonIntersecting { nearest, farthest } => (*nearest, *farthest),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Anomaly {
    Elliptical(f64),
    Parabolic(f64),
    Hyperbolic(f64),
}

impl Anomaly {
    fn with_ecc(ecc: f64, anomaly: f64) -> Self {
        if ecc > 1.0 {
            Anomaly::Hyperbolic(anomaly)
        } else if ecc == 1.0 {
            Anomaly::Parabolic(anomaly)
        } else {
            Anomaly::Elliptical(wrap_pi_npi_f64(anomaly))
        }
    }

    fn as_f64(&self) -> f64 {
        match self {
            Anomaly::Elliptical(v) => *v,
            Anomaly::Parabolic(v) => *v,
            Anomaly::Hyperbolic(v) => *v,
        }
    }
}

fn true_to_eccentric(true_anomaly: Anomaly, ecc: f64) -> Anomaly {
    match true_anomaly {
        Anomaly::Elliptical(v) => Anomaly::Elliptical({
            let term = f64::sqrt((1.0 - ecc) / (1.0 + ecc)) * f64::tan(0.5 * v);
            2.0 * term.atan()
        }),
        Anomaly::Hyperbolic(v) => {
            let x = ((ecc + v.cos()) / (1. + ecc * v.cos())).acosh();
            Anomaly::Hyperbolic(x.abs() * v.signum())
        }
        Anomaly::Parabolic(v) => Anomaly::Parabolic((v / 2.0).tan()),
    }
}

fn eccentric_to_mean(eccentric_anomaly: Anomaly, ecc: f64) -> Anomaly {
    match eccentric_anomaly {
        Anomaly::Elliptical(v) => Anomaly::Elliptical(v - ecc * v.sin()),
        Anomaly::Hyperbolic(v) => Anomaly::Hyperbolic(ecc * v.sinh() - v),
        Anomaly::Parabolic(v) => Anomaly::Parabolic(v + v.powi(3) / 3.0),
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
pub struct Body {
    pub radius: f64,
    pub mu: f64,
    pub soi: f64,
}

impl Body {
    pub const LUNA: Self = Self {
        radius: 1_737_400.0,
        mu: 4.902800118E12,
        soi: 12_000_000.0,
    };

    pub const SMOL: Self = Self {
        radius: 200.0,
        mu: 10000.0,
        soi: 4000.0,
    };

    pub const fn with_mass(radius: f64, mass: f64, soi: f64) -> Self {
        Body {
            radius,
            mu: mass * GRAVITATIONAL_CONSTANT,
            soi,
        }
    }

    pub const fn with_mu(radius: f64, mu: f64, soi: f64) -> Self {
        Body { radius, mu, soi }
    }

    pub fn mu(&self) -> f64 {
        self.mu
    }

    pub fn gravity(&self, p: impl Into<DVec2>) -> DVec2 {
        let p = p.into();
        let rsq = p.length_squared();

        if rsq < self.radius.powi(2) {
            return DVec2::ZERO;
        }

        let rhat = p.normalize_or_zero();
        -self.mu * rhat / rsq
    }
}

// https://en.wikipedia.org/wiki/Vis-viva_equation
pub fn vis_viva_equation(mu: f64, r: f64, a: f64) -> f64 {
    (mu * (2.0 / r - 1.0 / a)).sqrt()
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct SparseOrbit {
    eccentricity: f64,
    pub semi_major_axis: f64,
    pub arg_periapsis: f64,
    pub body: Body,
    pub initial: PV,
    pub epoch: Nanotime,
    pub time_at_periapsis: Option<Nanotime>,
}

impl SparseOrbit {
    pub fn from_pv(pv: impl Into<PV>, body: Body, epoch: Nanotime) -> Option<Self> {
        let pv: PV = pv.into();

        pv.filter_numerr()?;

        let r3 = pv.pos.extend(0.0);
        let v3 = pv.vel.extend(0.0);
        let h = r3.cross(v3);
        let e = v3.cross(h) / body.mu() as f64 - r3 / r3.length();
        let arg_periapsis = f64::atan2(e.y, e.x);
        let semi_major_axis = h.length_squared() / (body.mu() as f64 * (1.0 - e.length_squared()));

        let mut true_anomaly = f64::acos(e.dot(r3) / (e.length() * r3.length()));
        if r3.dot(v3) < 0.0 {
            true_anomaly = if e.length() < 1.0 {
                2.0 * PI_64 - true_anomaly
            } else {
                -true_anomaly
            };
        }

        let true_anomaly = Anomaly::with_ecc(e.length(), true_anomaly);

        let time_at_periapsis = {
            if e.length() > 0.97 {
                None
            } else {
                let eccentric_anomaly = true_to_eccentric(true_anomaly, e.length());
                let mean_anomaly = eccentric_to_mean(eccentric_anomaly, e.length());
                let mean_motion = (body.mu() / semi_major_axis.abs().powi(3)).sqrt();
                Some(epoch - Nanotime::secs_f64(mean_anomaly.as_f64() / mean_motion as f64))
            }
        };

        let o = SparseOrbit {
            eccentricity: e.length(),
            semi_major_axis,
            arg_periapsis,
            body,
            initial: pv,
            epoch,
            time_at_periapsis,
        };

        if let Some(p) = o.period() {
            if p == Nanotime::zero() {
                println!("SparseOrbit returned orbit with zero period: {pv:?}\n  {o:?}");
                return None;
            }
        }

        Some(o)
    }

    pub fn new(
        ra: f64,
        rp: f64,
        argp: f64,
        body: Body,
        epoch: Nanotime,
        retrograde: bool,
    ) -> Option<Self> {
        let sma = (ra + rp) / 2.0;
        let sign = if retrograde { -1.0 } else { 1.0 };
        let p = DVec2::X * rp;
        let v = sign * DVec2::Y * (body.mu() as f64 * ((2.0 / rp) - (1.0 / sma))).sqrt();
        let p = rotate_f64(p, argp);
        let v = rotate_f64(v, argp);
        let pv = PV::from_f64(p, v);
        SparseOrbit::from_pv(pv, body, epoch)
    }

    pub fn circular(radius: f64, body: Body, epoch: Nanotime, retrograde: bool) -> Self {
        let p = DVec2::new(radius, 0.0);
        let v = if retrograde { -1.0 } else { 1.0 }
            * DVec2::new(0.0, (body.mu() as f64 / radius).sqrt());
        SparseOrbit {
            eccentricity: 0.0,
            semi_major_axis: radius as f64,
            arg_periapsis: 0.0,
            body,
            initial: PV::from_f64(p, v),
            epoch,
            time_at_periapsis: Some(epoch),
        }
    }

    pub fn ecc(&self) -> f64 {
        self.eccentricity
    }

    pub fn h(&self) -> f64 {
        // TODO(precision)
        cross2d(self.initial.pos, self.initial.vel) as f64
    }

    pub fn is_retrograde(&self) -> bool {
        self.h() < 0.0
    }

    pub fn is_hyperbolic(&self) -> bool {
        match self.class() {
            OrbitClass::Hyperbolic | OrbitClass::Parabolic => true,
            _ => false,
        }
    }

    pub fn is_suborbital(&self) -> bool {
        self.periapsis_r() < self.body.radius as f64
    }

    pub fn will_escape(&self) -> bool {
        match self.class() {
            OrbitClass::Parabolic | OrbitClass::Hyperbolic => true,
            _ => self.apoapsis_r() > self.body.soi as f64,
        }
    }

    pub fn class(&self) -> OrbitClass {
        if self.eccentricity == 0.0 {
            OrbitClass::Circular
        } else if self.eccentricity < 0.2 {
            OrbitClass::NearCircular
        } else if self.eccentricity < 0.9 {
            OrbitClass::Elliptical
        } else if self.eccentricity < 1.0 {
            if self.eccentricity > 0.97 && self.is_suborbital() {
                OrbitClass::VeryThin
            } else {
                OrbitClass::HighlyElliptical
            }
        } else if self.eccentricity == 1.0 {
            OrbitClass::Parabolic
        } else {
            OrbitClass::Hyperbolic
        }
    }

    pub fn prograde_at(&self, true_anomaly: f64) -> Vec2 {
        let fpa = self.flight_path_angle_at(true_anomaly);
        DVec2::from_angle(fpa)
            .rotate(self.tangent_at(true_anomaly))
            .as_vec2()
    }

    pub fn flight_path_angle_at(&self, true_anomaly: f64) -> f64 {
        (self.eccentricity * true_anomaly.sin()).atan2(1.0 + self.eccentricity * true_anomaly.cos())
    }

    pub fn tangent_at(&self, true_anomaly: f64) -> DVec2 {
        let n = self.normal_at(true_anomaly);
        let angle = match self.is_retrograde() {
            true => -PI_64 / 2.0,
            false => PI_64 / 2.0,
        };
        DVec2::from_angle(angle).rotate(n)
    }

    pub fn normal_at(&self, true_anomaly: f64) -> DVec2 {
        self.position_at(true_anomaly).normalize()
    }

    pub fn semi_latus_rectum(&self) -> f64 {
        if self.eccentricity == 1.0 {
            return 2.0 * self.semi_major_axis;
        }
        self.semi_major_axis * (1.0 - self.eccentricity.powi(2))
    }

    pub fn semi_minor_axis(&self) -> f64 {
        (self.semi_major_axis.abs() * self.semi_latus_rectum().abs()).sqrt()
    }

    pub fn radius_at_angle(&self, angle: f64) -> f64 {
        let ta = angle - self.arg_periapsis;
        self.radius_at(ta)
    }

    pub fn position_at_angle(&self, angle: f64) -> DVec2 {
        let r = self.radius_at_angle(angle);
        rotate_f64(DVec2::X * r, angle)
    }

    pub fn pv_at_angle(&self, angle: f64) -> PV {
        let ta = if self.is_retrograde() {
            -angle + self.arg_periapsis
        } else {
            angle - self.arg_periapsis
        };
        let pos = self.position_at(ta);
        let vel = self.velocity_at(ta);
        PV::from_f64(pos, vel)
    }

    // TODO make this less stupid. should be able to compute
    // true anomaly more directly, and maybe without fallibility
    pub fn ta_at_time(&self, stamp: Nanotime) -> Option<f64> {
        let p = self.pv_universal(stamp).ok()?;
        let ta = if self.is_retrograde() {
            -p.pos.to_angle() as f64 + self.arg_periapsis
        } else {
            p.pos.to_angle() as f64 - self.arg_periapsis
        };
        Some(wrap_pi_npi_f64(ta))
    }

    pub fn radius_at(&self, true_anomaly: f64) -> f64 {
        if self.eccentricity == 1.0 {
            let mu = self.body.mu() as f64;
            return (self.h().powi(2) / mu) * 1.0 / (1.0 + true_anomaly.cos());
        }
        self.semi_major_axis * (1.0 - self.eccentricity.powi(2))
            / (1.0 + self.eccentricity * f64::cos(true_anomaly))
    }

    pub fn period(&self) -> Option<Nanotime> {
        if self.eccentricity >= 1.0 {
            return None;
        }
        let t = 2.0 * PI_64 / self.mean_motion();
        let ret = Nanotime::nanos((t * 1E9) as i64);
        if ret == Nanotime::zero() {
            return None;
        }
        Some(ret)
    }

    pub fn period_or(&self, fallback: Nanotime) -> Nanotime {
        self.period().unwrap_or(fallback)
    }

    pub fn pv(&self, stamp: Nanotime) -> Result<PV, ULData> {
        // if let Some(pv) = self.pv_lut(stamp) {
        //     return Ok(pv);
        // }
        self.pv_universal(stamp)
    }

    pub fn pv_universal(&self, stamp: Nanotime) -> Result<PV, ULData> {
        let tof = if let Some(p) = self.period() {
            (stamp - self.epoch) % p
        } else {
            stamp - self.epoch
        };

        let ul = universal_lagrange(self.initial, tof, self.body.mu());
        let sol = ul.1.ok_or(ul.0)?;
        if sol.pv.pos.length() > 3.0 * self.body.soi {
            return Err(ul.0);
        }
        Ok(sol.pv.filter_numerr().ok_or(ul.0)?)
    }

    pub fn pv_lut(&self, stamp: Nanotime) -> Option<PV> {
        let ma = self.mean_anomaly(stamp)?;
        let ta = lookup_ta_from_ma(ma, self.ecc())?;
        let pos = self.position_at(ta);
        let vel = self.velocity_at(ta);
        Some(PV::from_f64(pos, vel))
    }

    pub fn position_at(&self, true_anomaly: f64) -> DVec2 {
        let r = self.radius_at(true_anomaly);
        let angle = match self.is_retrograde() {
            false => true_anomaly,
            true => -true_anomaly,
        };
        DVec2::from_angle(angle + self.arg_periapsis) * r
    }

    pub fn velocity_at(&self, true_anomaly: f64) -> DVec2 {
        let r = self.radius_at(true_anomaly);
        let v = (self.body.mu() as f64 * (2.0 / r - 1.0 / self.semi_major_axis)).sqrt();
        let h = self.h().abs();
        let cosfpa = h / (r * v);
        let sinfpa = cosfpa * self.eccentricity * true_anomaly.sin()
            / (1.0 + self.eccentricity * true_anomaly.cos());
        let n = self.normal_at(true_anomaly);
        let t = self.tangent_at(true_anomaly);
        v * (t * cosfpa + n * sinfpa)
    }

    pub fn periapsis(&self) -> DVec2 {
        self.position_at(0.0)
    }

    pub fn periapsis_r(&self) -> f64 {
        self.radius_at(0.0)
    }

    pub fn apoapsis(&self) -> DVec2 {
        self.position_at(PI_64)
    }

    pub fn apoapsis_r(&self) -> f64 {
        self.radius_at(PI_64)
    }

    pub fn mean_motion(&self) -> f64 {
        (self.body.mu() as f64 / self.semi_major_axis.abs().powi(3)).sqrt()
    }

    pub fn mean_anomaly(&self, stamp: Nanotime) -> Option<f64> {
        let period = self.period()?;
        let tp = self.t_next_p(stamp)? - period;
        let dt = stamp - tp;
        Some(2.0 * PI_64 * dt.to_secs_f64() / period.to_secs_f64())
    }

    pub fn orbit_number(&self, stamp: Nanotime) -> Option<i64> {
        let p = self.period()?;
        let dt = stamp - self.time_at_periapsis?;
        let n = dt.inner().checked_div(p.inner())?;
        Some(if dt.inner() < 0 { n - 1 } else { n })
    }

    pub fn inverse(&self) -> Option<SparseOrbit> {
        SparseOrbit::from_pv(
            PV::from_f64(self.initial.pos, -self.initial.vel),
            self.body,
            self.epoch,
        )
    }

    pub fn t_next_p(&self, current: Nanotime) -> Option<Nanotime> {
        let tp = self.time_at_periapsis?;
        if self.eccentricity >= 1.0 {
            return (tp >= current).then(|| tp);
        }
        let p = self.period()?;
        let n = self.orbit_number(current)?;
        Some(p * (n + 1) + tp)
    }

    pub fn asymptotes(&self) -> Option<(DVec2, DVec2)> {
        if self.eccentricity < 1.0 {
            return None;
        }
        let u = self.periapsis().normalize();
        let b = self.semi_minor_axis();

        let ua = DVec2::new(self.semi_major_axis, b);
        let ub = DVec2::new(self.semi_major_axis, -b);

        Some((u.rotate(ua), u.rotate(ub)))
    }

    pub fn center(&self) -> DVec2 {
        (self.apoapsis() + self.periapsis()) / 2.0
    }

    pub fn sdf(&self, pos: DVec2) -> f64 {
        let center = self.center();
        let mut d = rotate_f64(pos - center, -self.arg_periapsis);

        d.y *= self.semi_major_axis / self.semi_minor_axis();

        d.length() - self.semi_major_axis
    }

    pub fn obb(&self) -> Option<OBB> {
        (self.eccentricity < 1.0).then(|| {
            OBB::new(
                AABB::new(
                    self.center().as_vec2(),
                    DVec2::new(self.semi_major_axis * 2.0, self.semi_minor_axis() * 2.0).as_vec2(),
                ),
                self.arg_periapsis as f32,
            )
        })
    }

    pub fn nearest_along_track(&self, pos: DVec2) -> (PV, f64) {
        let angle = -pos.angle_to(DVec2::X);
        let p = self.pv_at_angle(angle as f64);
        let d = p.pos.distance(pos);
        if p.pos.length() > pos.length() {
            (p, -d)
        } else {
            (p, d)
        }
    }

    pub fn nearest(&self, pos: DVec2) -> (PV, f64) {
        let (mut ret, mut dist) = self.nearest_along_track(pos);
        let sign = dist.signum();
        let mut test_pos = pos;
        for _ in 0..4 {
            let (pv, d) = self.nearest_along_track(test_pos);
            let u = match pv.vel.try_normalize() {
                Some(u) => u,
                None => return (pv, d),
            };
            let diff = pos - pv.pos;
            let mag = diff.dot(u);
            test_pos = pv.pos + mag * u;
            dist = d;
            ret = pv;
        }

        (ret, dist.abs() * sign)
    }

    pub fn timed_approach_info(
        &self,
        other: SparseOrbit,
        now: Nanotime,
        dur: Nanotime,
    ) -> (Option<Nanotime>, Option<Nanotime>) {
        // distance between orbits along a ray cast from planet object
        let separation = |t: Nanotime| {
            let d = self.pv(t).unwrap_or(PV::NAN) - other.pv(t).unwrap_or(PV::NAN);
            d.pos.length_squared()
        };

        let teval = tspace(now, now + dur, 100);

        let c3 = |t: Nanotime| separation(t) > 1000.0;
        let c4 = |t: Nanotime| separation(t) < 1000.0;

        let mut rising = None;
        let mut falling = None;

        let tol = Nanotime::secs(30);

        for t in teval.windows(2) {
            match search_condition::<Nanotime>(t[0], t[1], tol, &c3) {
                Ok(Some(found)) => rising = Some(found),
                Ok(None) => (),
                Err(e) => {
                    dbg!(e);
                    return (None, None);
                }
            }
            match search_condition::<Nanotime>(t[0], t[1], tol, &c4) {
                Ok(Some(found)) => falling = Some(found),
                Ok(None) => (),
                Err(e) => {
                    dbg!(e);
                    return (None, None);
                }
            }
        }

        (rising, falling)
    }

    pub fn is_similar(&self, other: &Self) -> bool {
        // TODO want this to be a sliding scale in [0, 1]
        let avg = 0.5 * (self.semi_major_axis + other.semi_major_axis);
        let dmax = avg * 0.06;
        let d1 = self.apoapsis().distance(other.apoapsis());
        let d2 = self.periapsis().distance(other.periapsis());
        d1 < dmax && d2 < dmax
    }

    pub fn to_perifocal(&self) -> Self {
        let p = rotate_f64(self.initial.pos, -self.arg_periapsis);
        let v = rotate_f64(self.initial.vel, -self.arg_periapsis);
        SparseOrbit {
            eccentricity: self.eccentricity,
            semi_major_axis: self.semi_major_axis,
            arg_periapsis: 0.0,
            body: self.body,
            initial: PV::from_f64(p, v),
            epoch: self.epoch,
            time_at_periapsis: self.time_at_periapsis,
        }
    }

    pub fn sample_pos(
        &self,
        start: Nanotime,
        end: Nanotime,
        dist: f64,
        origin: impl Into<DVec2>,
    ) -> Option<Vec<DVec2>> {
        let origin = origin.into();
        let mut ret = Vec::new();
        let mut t = start;
        while t < end {
            let pv = self.pv(t).ok()?;
            let dt = Nanotime::secs_f64(dist / pv.vel.length()).max(Nanotime::secs(60));
            ret.push(pv.pos + origin);
            t += dt;
        }
        ret.push(self.pv(end).ok()?.pos + origin);
        Some(ret)
    }
}

impl std::fmt::Display for SparseOrbit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "A{:0.0}-E{:0.2}-P{:0.2}{}",
            self.semi_major_axis,
            self.ecc(),
            self.arg_periapsis,
            if self.is_retrograde() { "r" } else { "p" },
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrbitClass {
    Circular,
    NearCircular,
    Elliptical,
    HighlyElliptical,
    Parabolic,
    Hyperbolic,
    VeryThin,
}

// https://www.coursesidekick.com/mathematics/441994

// https://orbital-mechanics.space/time-since-periapsis-and-keplers-equation/universal-variables.html

// 2nd stumpff function
// aka C(z)
pub fn stumpff_2(z: f64) -> f64 {
    let midwidth = 0.01;
    if z > midwidth {
        (1.0 - z.sqrt().cos()) / z
    } else if z < -midwidth {
        ((-z).sqrt().cosh() - 1.0) / -z
    } else {
        0.5 - 0.04 * z
    }
}

// 3rd stumpff function
// aka S(z)
pub fn stumpff_3(z: f64) -> f64 {
    let midwidth = 0.01;
    if z > midwidth {
        (z.sqrt() - z.sqrt().sin()) / z.powf(1.5)
    } else if z < -midwidth {
        ((-z).sqrt().sinh() - (-z).sqrt()) / (-z).powf(1.5)
    } else {
        -0.00833 * z + 1.0 / 6.0
    }
}

fn universal_kepler(chi: f64, r_0: f64, v_r0: f64, alpha: f64, delta_t: f64, mu: f64) -> f64 {
    let z = alpha * chi.powi(2);
    let first_term = r_0 * v_r0 / mu.sqrt() * chi.powi(2) * stumpff_2(z);
    let second_term = (1.0 - alpha * r_0) * chi.powi(3) * stumpff_3(z);
    let third_term = r_0 * chi;
    let fourth_term = mu.sqrt() * delta_t;
    first_term + second_term + third_term - fourth_term
}

#[derive(Debug, Copy, Clone)]
pub(crate) struct LangrangeCoefficients {
    #[allow(unused)]
    pub(crate) s2: f64,
    #[allow(unused)]
    pub(crate) s3: f64,
    pub(crate) f: f64,
    pub(crate) g: f64,
    pub(crate) fdot: f64,
    pub(crate) gdot: f64,
}

#[derive(Debug, Copy, Clone)]
pub struct ULData {
    pub(crate) initial: PV,
    pub(crate) tof: Nanotime,
    pub(crate) mu: f64,
    pub(crate) r_0: f64,
    pub(crate) v_r0: f64,
    pub(crate) chi_0: f64,
    pub(crate) alpha: f64,
}

impl ULData {
    fn new(initial: impl Into<PV>, tof: Nanotime, mu: f64) -> Self {
        let initial = initial.into();
        let r_0 = initial.pos.length();
        let alpha = 2.0 / r_0 - initial.vel.dot(initial.vel) / mu;
        ULData {
            initial,
            tof,
            mu,
            r_0,
            v_r0: initial.vel.dot(initial.pos) / r_0,
            alpha,
            chi_0: mu.sqrt() * alpha.abs() * tof.to_secs_f64(),
        }
    }

    fn universal_kepler(&self, chi: f64) -> f64 {
        universal_kepler(
            chi,
            self.r_0,
            self.v_r0,
            self.alpha,
            self.tof.to_secs_f64(),
            self.mu,
        )
    }

    fn solve_fast(&self) -> Option<ULResults> {
        ULResults::new(self.chi_0, &self)
    }

    fn solve(&self) -> Option<ULResults> {
        let radius = 800000.0;
        let chi_min = self.chi_0 - radius;
        let chi_max = self.chi_0 + radius;
        let chi = if self.tof == Nanotime::zero() {
            0.0
        } else {
            match rootfinder::root_bisection(
                &|x: f64| self.universal_kepler(x),
                rootfinder::Interval::new(chi_min, chi_max),
                None,
                None,
            ) {
                Ok(x) => x,
                Err(_) => {
                    return None;
                }
            }
        };

        if chi == chi_min || chi == chi_max {
            return None;
        }

        ULResults::new(chi, &self)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ULResults {
    pub(crate) pv: PV,
    pub chi: f64,
    pub(crate) z: f64,
    pub(crate) lc: LangrangeCoefficients,
}

impl ULResults {
    fn new(chi: f64, data: &ULData) -> Option<Self> {
        let z = data.alpha * chi.powi(2);
        let lcoeffs = lagrange_coefficients(data.initial, chi, data.mu, data.tof);
        let pv = lagrange_pv(data.initial, &lcoeffs).filter_numerr()?;
        Some(ULResults {
            pv,
            chi,
            z,
            lc: lcoeffs,
        })
    }
}

// https://en.wikipedia.org/wiki/Universal_variable_formulation
// https://orbital-mechanics.space/time-since-periapsis-and-keplers-equation/universal-lagrange-coefficients-example.html
pub fn universal_lagrange(
    initial: impl Into<PV>,
    tof: Nanotime,
    mu: f64,
) -> (ULData, Option<ULResults>) {
    let data = ULData::new(initial, tof, mu);
    (data, data.solve())
}
pub fn universal_lagrange_fast(
    initial: impl Into<PV>,
    tof: Nanotime,
    mu: f64,
) -> (ULData, Option<ULResults>) {
    let data = ULData::new(initial, tof, mu);
    (data, data.solve_fast())
}

pub(crate) fn lagrange_coefficients(
    initial: impl Into<PV>,
    chi: f64,
    mu: f64,
    dt: Nanotime,
) -> LangrangeCoefficients {
    let initial = initial.into();
    let vec_r_0 = initial.pos;
    let vec_v_0 = initial.vel;

    let r_0 = vec_r_0.length();

    let alpha = 2.0 / r_0 - vec_v_0.dot(vec_v_0) / mu;

    let delta_t = dt.to_secs_f64();

    let z = alpha * chi.powi(2);

    let s2 = stumpff_2(z);
    let s3 = stumpff_3(z);

    let f = 1.0 - chi.powi(2) / r_0 * s2;
    let g = delta_t - chi.powi(3) / mu.sqrt() * s3;

    let vec_r = f * vec_r_0 + g * vec_v_0;
    let r = vec_r.length();

    let fdot = chi * mu.sqrt() / (r * r_0) * (z * s3 - 1.0);
    let gdot = 1.0 - chi.powi(2) / r * s2;

    LangrangeCoefficients {
        s2,
        s3,
        f,
        g,
        fdot,
        gdot,
    }
}

pub(crate) fn lagrange_pv(initial: impl Into<PV>, coeff: &LangrangeCoefficients) -> PV {
    let initial = initial.into();
    let vec_r = coeff.f * initial.pos + coeff.g * initial.vel;
    let vec_v = coeff.fdot * initial.pos + coeff.gdot * initial.vel;
    PV::from_f64(vec_r, vec_v)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct GlobalOrbit(pub Ent, pub SparseOrbit);

impl std::fmt::Display for GlobalOrbit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.0, self.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use more_asserts::*;

    fn ncalc_period(orbit: &SparseOrbit) -> Option<(Nanotime, Nanotime)> {
        let dt = Nanotime::millis(10);
        let mut duration = Nanotime::zero();
        let pv0 = orbit.initial;
        let mut d_prev = 0.0;
        let mut was_decreasing = false;
        while duration < Nanotime::secs(10000) {
            duration += dt;
            let t = orbit.epoch + duration;
            let pv = orbit.pv(t).ok()?;
            let d = pv.pos.distance(pv0.pos);
            let increasing = d > d_prev;
            d_prev = d;

            let aligned = pv0.vel.dot(pv.vel) > 0.0;

            if d < 20.0 && aligned && increasing && was_decreasing {
                return Some((t - dt * 5, t));
            }

            was_decreasing = !increasing;
        }
        None
    }

    fn physics_based_smoketest(orbit: &SparseOrbit) {
        let mut particle = orbit.initial;
        let dt = Nanotime::millis(2);
        let mut t = orbit.epoch;

        let mut last_error = 0.0;
        let max_error_growth = 1.0;
        let mut previous = PV::ZERO;

        while t < Nanotime::secs(100) {
            t += dt;
            let porbit = match orbit.pv(t) {
                Ok(p) => p,
                Err(ul) => {
                    assert!(false, "Bad orbital position at {:?}\n  {:?}", t, ul);
                    return;
                }
            };
            let r2 = particle.pos.length_squared();
            let a = -orbit.body.mu() / r2 * particle.pos.normalize_or_zero();
            let delta = PV::from_f64(a * dt.to_secs_f64(), particle.vel * dt.to_secs_f64());
            particle += delta;

            let error = porbit.pos.distance(particle.pos);
            let max_error = last_error + max_error_growth;
            assert_le!(
                error,
                max_error,
                "Deviation exceeded at {:?}, prev error {:0.3}\
                \n  Particle:       {:?}\
                \n  Previous orbit: {:?}\
                \n  Bad orbit pos:  {:?}",
                t,
                last_error,
                particle,
                previous,
                porbit,
            );
            last_error = error;
            previous = porbit;
        }

        println!("Max error: {:0.3}", last_error);
    }

    fn assert_defined_for_large_time_range(orbit: &SparseOrbit) {
        // TODO apply this to hyperbolic orbits too!
        match orbit.class() {
            OrbitClass::Hyperbolic | OrbitClass::Parabolic => {
                return;
            }
            _ => (),
        }

        let n: usize = 10000;
        let t1 = tspace(Nanotime::zero(), Nanotime::secs(n as i64), n);
        let t2 = tspace(Nanotime::zero(), Nanotime::secs(-(n as i64)), n);
        for t in t1.iter().chain(t2.iter()) {
            let pv = orbit.pv(*t);
            assert!(pv.is_ok(), "Failure at time {:?} - {:?}", t, pv);
        }
    }

    fn calculate_actual_time_at_periapsis(orbit: &SparseOrbit) -> Nanotime {
        if orbit.ecc() == 0.0 {
            return orbit.epoch;
        }

        let velocity_up = |t: Nanotime| {
            let pv = orbit.pv_universal(t).unwrap();
            let u = pv.pos.normalize_or_zero();
            pv.vel.dot(u)
        };

        let condition = |t: Nanotime| velocity_up(t) < 0.0;

        let tol = Nanotime::nanos(5);

        let res = {
            let mut t1 = orbit.epoch;

            while velocity_up(t1) >= 0.0 {
                t1 -= Nanotime::secs(1);
            }

            let mut t2 = orbit.epoch;

            while velocity_up(t2) <= 0.0 {
                t2 += Nanotime::secs(1);
            }

            println!(
                "{} -> {}, {} -> {}",
                t1,
                velocity_up(t1),
                t2,
                velocity_up(t2)
            );

            search_condition(t1, t2, tol, &condition)
        };

        assert!(res.is_ok(), "Search failed: {:?}\n  {}", res, orbit);
        let res = res.unwrap();
        assert!(res.is_some(), "Search failed for orbit {}", orbit);
        let tp = res.unwrap();
        println!("Time at periapsis is {}", tp);
        tp
    }

    fn orbit_consistency_test(
        pv: PV,
        class: OrbitClass,
        body: Body,
        ecc: f64,
        is_retrograde: bool,
    ) {
        println!("{}", pv);

        let orbit = SparseOrbit::from_pv(pv, body, Nanotime::zero());

        assert!(orbit.is_some());

        let orbit = orbit.unwrap();

        println!("{}", orbit);

        if !ecc.is_nan() {
            assert_relative_eq!(ecc, orbit.ecc());
        }

        assert_eq!(
            is_retrograde,
            orbit.is_retrograde(),
            "Orbit {} expected is_retrograde = {}",
            pv,
            is_retrograde
        );

        assert_eq!(orbit.pv_universal(orbit.epoch).ok(), Some(orbit.initial));
        assert!(
            orbit
                .pv(orbit.epoch)
                .unwrap()
                .pos
                .distance(orbit.initial.pos)
                < 10.0
        );

        physics_based_smoketest(&orbit);
        assert_defined_for_large_time_range(&orbit);

        if let Some(((min, max), period)) = ncalc_period(&orbit).zip(orbit.period()) {
            dbg!((min, max, period));
            let tol = Nanotime::secs(1);
            assert_le!(min - tol, period, "Period too small: {:?}", orbit);
            assert_ge!(max + tol, period, "Period too big: {:?}", orbit);
        }

        if let (Some(tp), t2) = (
            orbit.time_at_periapsis,
            calculate_actual_time_at_periapsis(&orbit),
        ) {
            let dt = (tp - t2).abs();
            println!("{} {:?} {:?} {}", orbit, orbit.time_at_periapsis, tp, dt);
            assert!(
                dt.abs() < Nanotime::millis(2),
                "dt exceeds threshold: dt={}",
                dt
            );
        }

        assert_eq!(orbit.class(), class);

        if !orbit.is_hyperbolic() {
            for millis in (0..20000).step_by(100) {
                let t = orbit.epoch + Nanotime::millis(millis);
                let ta = orbit.ta_at_time(t).unwrap();
                let p1 = orbit.position_at(ta);
                let p2 = orbit.pv_universal(t).unwrap().pos;
                assert_le!(
                    p1.distance(p2),
                    0.02,
                    "Disagreement at time {}, ta={}\n  position_at: {}\n  pv_universal: {}",
                    t,
                    ta,
                    p1,
                    p2
                );
                if let Some(p3) = orbit.pv_lut(t) {
                    assert_le!(p1.distance(p3.pos), 1.0);
                }
            }
        }

        if let Some(t) = orbit.t_next_p(orbit.epoch) {
            assert_le!(orbit.ta_at_time(t).unwrap(), 1E-2);
        }
    }

    #[test]
    fn orbit_001_elliptical() {
        orbit_consistency_test(
            PV::from_f64((669.058, -1918.289), (74.723, 60.678)),
            OrbitClass::Elliptical,
            Body::with_mass(63.0, 1000.0, 15000.0),
            0.6335363,
            false,
        );
    }

    #[test]
    fn orbit_002_elliptical() {
        orbit_consistency_test(
            PV::from_f64((430.0, 230.0), (-50.14, 40.13)),
            OrbitClass::Elliptical,
            Body::with_mass(63.0, 1000.0, 15000.0),
            0.860516,
            false,
        );
    }

    #[test]
    fn orbit_003_hyperbolic() {
        orbit_consistency_test(
            PV::from_f64((0.0, -222.776), (333.258, 0.000)),
            OrbitClass::Hyperbolic,
            Body::with_mass(63.0, 1000.0, 15000.0),
            1.0618086,
            false,
        );
    }

    #[test]
    fn orbit_004_elliptical() {
        orbit_consistency_test(
            PV::from_f64((1520.323, 487.734), (-84.935, 70.143)),
            OrbitClass::Elliptical,
            Body::with_mass(63.0, 1000.0, 15000.0),
            0.74756867,
            false,
        );
    }

    #[test]
    fn orbit_005_hyperbolic() {
        orbit_consistency_test(
            PV::from_f64((5535.6294, -125.794685), (-66.63476, 16.682587)),
            OrbitClass::Hyperbolic,
            Body::with_mass(63.0, 1000.0, 15000.0),
            1.0093584,
            false,
        );
    }

    #[test]
    fn orbit_006_hyperbolic() {
        orbit_consistency_test(
            PV::from_f64((65.339584, 1118.9651), (-138.84702, -279.47888)),
            OrbitClass::Hyperbolic,
            Body::with_mass(63.0, 1000.0, 15000.0),
            3.3041847,
            false,
        );
    }

    #[test]
    fn orbit_007_hyperbolic() {
        orbit_consistency_test(
            PV::from_f64((-1856.4648, -1254.9697), (216.31313, -85.84622)),
            OrbitClass::Hyperbolic,
            Body::with_mass(63.0, 1000.0, 15000.0),
            7.5504527,
            false,
        );
    }

    #[test]
    fn orbit_008_hyperbolic() {
        orbit_consistency_test(
            PV::from_f64((-72.39488, 662.50507), (3.4047441, 71.81263)),
            OrbitClass::Hyperbolic,
            Body::with_mass(22.0, 10.0, 800.0),
            4.422243,
            true,
        );
    }

    #[test]
    fn orbit_009_hyperbolic() {
        orbit_consistency_test(
            PV::from_f64((825.33563, 564.6425), (200.0, 230.0)),
            OrbitClass::Hyperbolic,
            Body::with_mass(63.0, 1000.0, 15000.0),
            1.9568859,
            false,
        );
    }

    #[test]
    fn orbit_011_elliptical() {
        orbit_consistency_test(
            PV::from_f64((-70.0, 600.0), (3.0, 16.0)),
            OrbitClass::HighlyElliptical,
            Body::with_mass(22.0, 10.0, 800.0),
            0.96003157,
            true,
        );
    }

    #[test]
    fn orbit_012_elliptical() {
        orbit_consistency_test(
            PV::from_f64((-70.0, 600.0), (3.0, 9.0)),
            OrbitClass::HighlyElliptical,
            Body::with_mass(22.0, 10.0, 800.0),
            0.93487203,
            true,
        );
    }

    #[test]
    fn orbit_010_circular() {
        let body = Body::with_mass(63.0, 1000.0, 15000.0);
        let orbit = SparseOrbit::circular(100.0, body, Nanotime::zero(), false);

        assert_relative_eq!(orbit.h(), 34641.016);
        assert_relative_eq!(orbit.inverse().unwrap().h(), -34641.016);

        orbit_consistency_test(orbit.initial, OrbitClass::Circular, body, 0.0, false);
        orbit_consistency_test(
            orbit.inverse().unwrap().initial,
            OrbitClass::Circular,
            body,
            0.0,
            true,
        );
    }

    #[test]
    fn orbit_013_elliptical() {
        orbit_consistency_test(
            PV::from_f64((1687.193, -2242.213), (59.740, 44.953)),
            OrbitClass::Elliptical,
            Body::with_mass(63.0, 1000.0, 15000.0),
            0.30708584,
            false,
        );
    }

    // #[test]
    // fn orbit_014_elliptical() {
    //     orbit_consistency_test(
    //         PV::from_f64((-3485.286, 1511.773), (-25.496, -58.779)),
    //         OrbitClass::Elliptical,
    //         Body::with_mass(63.0, 1000.0, 15000.0),
    //         0.29959226,
    //         false,
    //     );
    // }

    // #[test]
    // fn orbit_015_elliptical() {
    //     orbit_consistency_test(
    //         PV::from_f64((-3485.286, 1511.773), (-21.694, -50.014)),
    //         OrbitClass::Elliptical,
    //         Body::with_mass(63.0, 1000.0, 15000.0),
    //         0.30708584,
    //         false,
    //     );
    // }

    #[test]
    fn assert_positions_are_as_expected_universal() {
        let body = Body::with_mass(300.0, 1000.0, 100000.0);
        let pv = PV::from_f64((6500.0, 7000.0), (-14.0, 11.0));
        let orbit = SparseOrbit::from_pv(pv, body, Nanotime::zero()).unwrap();
        let inverse = orbit.inverse().unwrap();

        orbit_consistency_test(pv, OrbitClass::Elliptical, body, 0.7496509, false);

        assert_eq!(orbit.period().unwrap(), Nanotime::nanos(732959932416));

        let tests_1 = [
            (0, ((6500.000, 7000.000), (-14.000, 11.000))),
            (1, ((6485.955, 7010.952), (-14.089, 10.904))),
            (2, ((6471.821, 7021.807), (-14.179, 10.807))),
            (3, ((6457.598, 7032.565), (-14.268, 10.710))),
            (4, ((6443.286, 7043.227), (-14.357, 10.613))),
            (5, ((6428.885, 7053.792), (-14.446, 10.516))),
            (6, ((6414.395, 7064.258), (-14.534, 10.418))),
            (50, ((5690.970, 7424.940), (-18.311, 5.894))),
            (100, ((4672.958, 7576.260), (-22.377, -0.008))),
            (200, ((2049.801, 6815.513), (-29.917, -16.783))),
            (500, ((6723.073, 2081.153), (16.944, 30.457))),
        ];

        let tests_2 = [
            (0, ((6500.000, 7000.000), (14.000, -11.000))),
            (1, ((6513.956, 6988.952), (13.910, -11.096))),
            (2, ((6527.820, 6977.807), (13.821, -11.192))),
            (3, ((6541.597, 6966.567), (13.731, -11.288))),
            (4, ((6555.282, 6955.231), (13.641, -11.384))),
            (5, ((6568.878, 6943.799), (13.551, -11.479))),
            (6, ((6582.384, 6932.272), (13.460, -11.575))),
            (50, ((7084.593, 6333.183), (9.304, -15.608))),
            (100, ((7420.361, 5444.321), (4.001, -19.907))),
            (200, ((7165.859, 3044.784), (-10.193, -27.985))),
            (500, ((1028.914, 6126.698), (31.940, 25.448))),
        ];

        for (orbit, tests) in &[(orbit, tests_1), (inverse, tests_2)] {
            for (t, (p, v)) in tests {
                let t = Nanotime::secs(*t);
                let pv = PV::from_f64(*p, *v);
                let actual = orbit.pv_universal(t).unwrap();
                let d = pv - actual;
                assert!(
                    d.pos.length() < 0.001 && d.vel.length() < 0.001,
                    "At time {:?}...\n  expected {}\n  actually {}",
                    t,
                    pv,
                    actual
                );
            }
        }
    }

    #[test]
    fn assert_true_anomaly_pos_as_expected() {
        let body = Body::with_mass(300.0, 1000.0, 100000.0);
        let pv = PV::from_f64((6500.0, 7000.0), (-14.0, 11.0));
        let orbit = SparseOrbit::from_pv(pv, body, Nanotime::zero()).unwrap();

        orbit_consistency_test(pv, OrbitClass::Elliptical, body, 0.7496509, false);

        assert_eq!(orbit.period().unwrap(), Nanotime::nanos(732959932416));

        let tests_1 = [
            (0.0, ((-958.451, -976.645), (88.408, -86.761))),
            (1.0, ((378.518, -1661.429), (106.907, -21.447))),
            (2.0, ((3272.588, -1182.707), (61.942, 29.408))),
        ];

        let tests_2 = [
            (0.0, ((-958.451, -976.645), (-88.408, 86.761))),
            (1.0, ((-1668.252, 347.212), (-23.453, 106.485))),
            (2.0, ((-1244.031, 3249.771), (28.238, 62.484))),
        ];

        for (orbit, tests) in [(orbit, tests_1), (orbit.inverse().unwrap(), tests_2)] {
            for (ta, pv) in tests {
                let pv: PV = pv.into();
                let pos = orbit.position_at(ta);
                let vel = orbit.velocity_at(ta);
                let actual = PV::from_f64(pos, vel);
                let d = pv - actual;
                assert!(
                    d.pos.length() < 0.001 && d.vel.length() < 0.001,
                    "At true anomaly {:0.4}...\n  expected {}\n  actually {}",
                    ta,
                    pv,
                    actual
                );
            }
        }
    }

    #[test]
    fn universal_lagrange_example() {
        let vec_r_0 = DVec2::new(7000.0, -12124.0);
        let vec_v_0 = DVec2::new(2.6679, 4.6210);
        let mu = 3.986004418E5;

        let tof = Nanotime::secs(3600);

        let (_, res) = super::universal_lagrange((vec_r_0, vec_v_0), tof, mu);

        assert_eq!(
            res.unwrap().pv,
            PV::from_f64((-3297.7869, 7413.3867), (-8.297602, -0.9640651))
        );
    }

    #[test]
    fn stumpff() {
        assert_relative_eq!(stumpff_2(-20.0), 2.1388736);
        assert_relative_eq!(stumpff_2(-5.0), 0.74633473);
        assert_relative_eq!(stumpff_2(-1.0), 0.5430807);
        assert_relative_eq!(stumpff_2(-1E-6), 0.50000006);
        assert_relative_eq!(stumpff_2(-1E-12), 0.5);
        assert_relative_eq!(stumpff_2(0.0), 0.5);
        assert_relative_eq!(stumpff_2(1E-12), 0.5);
        assert_relative_eq!(stumpff_2(1E-6), 0.49999997);
        assert_relative_eq!(stumpff_2(1.0), 0.45969772);
        assert_relative_eq!(stumpff_2(5.0), 0.32345456);
        assert_relative_eq!(stumpff_2(20.0), 0.061897416);

        assert_relative_eq!(stumpff_3(-20.0), 0.43931928);
        assert_relative_eq!(stumpff_3(-1E-12), 0.16666667);
        assert_relative_eq!(stumpff_3(0.0), 0.16666667);
        assert_relative_eq!(stumpff_3(1E-12), 0.16666667);
        assert_relative_eq!(stumpff_3(20.0), 0.060859215);
    }

    #[test]
    fn inverse_orbit() {
        const TEST_POSITION: DVec2 = DVec2::new(500.0, 300.0);
        const TEST_VELOCITY: DVec2 = DVec2::new(-200.0, 0.0);

        let body = Body {
            radius: 100.0,
            mu: 1000.0 * 12000.0,
            soi: 10000.0,
        };

        let o1 =
            SparseOrbit::from_pv((TEST_POSITION, TEST_VELOCITY), body, Nanotime::zero()).unwrap();
        let o2 =
            SparseOrbit::from_pv((TEST_POSITION, -TEST_VELOCITY), body, Nanotime::zero()).unwrap();

        let true_h = TEST_POSITION.extend(0.0).cross(TEST_VELOCITY.extend(0.0)).z;

        assert_relative_eq!(o1.h(), true_h);
        assert!(o1.h() > 0.0);
        assert!(!o1.is_retrograde());

        assert_relative_eq!(o2.h(), -true_h);
        assert!(o2.h() < 0.0);
        assert!(o2.is_retrograde());

        assert!(!o1.is_hyperbolic());
        assert!(!o2.is_hyperbolic());

        assert_eq!(o1.period().unwrap(), o2.period().unwrap());

        // TODO make this better
        for i in [0, 1, 2] {
            let t = o1.period().unwrap() * i;
            println!("{t:?} {} {}", o1.pv(t).unwrap(), o2.pv(t).unwrap());
            assert_relative_eq!(
                o1.pv(t).unwrap().pos.x,
                o2.pv(t).unwrap().pos.x,
                epsilon = 0.5
            );
            assert_relative_eq!(
                o1.pv(t).unwrap().pos.y,
                o2.pv(t).unwrap().pos.y,
                epsilon = 0.5
            );
        }
    }

    #[test]
    fn time_at_periapsis() {
        let body = Body::with_mass(50.0, 1000.0, 1E8);
        let pos = DVec2::Y * -700.0;

        for v in linspace_f64(10.0, 70.0, 15) {
            let vel = DVec2::new(v * 3.0, v);
            let pv = PV::from_f64(pos, vel);
            let orbit = SparseOrbit::from_pv(pv, body, Nanotime::zero());
            assert!(orbit.is_some(), "Bad orbit: {}", v);
            let orbit = orbit.unwrap();
            let tp = calculate_actual_time_at_periapsis(&orbit);
            let dt = if let (t1, Some(t2)) = (tp, orbit.time_at_periapsis) {
                t1 - t2
            } else {
                Nanotime::zero()
            };
            println!("{} {:?} {:?} {}", orbit, orbit.time_at_periapsis, tp, dt);
            assert!(
                dt.abs() < Nanotime::millis(1),
                "dt exceeds threshold: dt={}",
                dt
            );
        }
    }
}
