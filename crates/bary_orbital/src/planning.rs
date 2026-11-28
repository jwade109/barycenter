use crate::*;
use bary_core::prelude::*;

#[derive(Debug, Clone)]
pub struct ManeuverPlan {
    pub initial: SparseOrbit,
    pub segments: Vec<ManeuverSegment>,
    pub terminal: SparseOrbit,
}

impl ManeuverPlan {
    pub fn new(now: Nanotime, initial: SparseOrbit, dvs: &[(Nanotime, DVec2)]) -> Option<Self> {
        if dvs.is_empty() {
            return None;
        }

        let mut segments: Vec<ManeuverSegment> = vec![];

        for (t, dv) in dvs.iter() {
            let segment = if let Some(c) = segments.last() {
                c.next(*t, *dv)
            } else {
                ManeuverSegment::new(now, *t, initial, *dv)
            }?;

            segments.push(segment);
        }

        let terminal = segments.last()?.next_orbit()?;

        Some(ManeuverPlan {
            initial,
            segments,
            terminal,
        })
    }

    pub fn pv(&self, stamp: Nanotime) -> Option<PV> {
        for segment in &self.segments {
            if let Some(pv) = (segment.start <= stamp && stamp <= segment.end)
                .then(|| segment.orbit.pv(stamp).ok())
                .flatten()
            {
                return Some(pv);
            }
        }
        None
    }

    pub fn start(&self) -> Nanotime {
        self.segments.iter().map(|e| e.start).next().unwrap()
    }

    pub fn end(&self) -> Nanotime {
        self.segments.iter().map(|e| e.end).next_back().unwrap()
    }

    pub fn duration(&self) -> Nanotime {
        self.end() - self.start()
    }

    pub fn dvs(&self) -> impl Iterator<Item = (Nanotime, DVec2)> + use<'_> {
        self.segments.iter().map(|m| m.dv())
    }

    pub fn future_dvs(&self, stamp: Nanotime) -> impl Iterator<Item = (Nanotime, DVec2)> + use<'_> {
        self.dvs().filter(move |(t, _)| *t > stamp)
    }

    pub fn dv(&self) -> f64 {
        self.segments.iter().map(|n| n.impulse.length()).sum()
    }

    pub fn segment_at(&self, stamp: Nanotime) -> Option<&ManeuverSegment> {
        self.segments.iter().find(|s| s.is_valid(stamp))
    }

    pub fn then(&self, other: Self) -> Result<Self, &'static str> {
        if self.end() > other.start() {
            return Err("Self ends after new plan begins");
        }

        let dvs: Vec<_> = self.dvs().chain(other.dvs()).collect();

        ManeuverPlan::new(self.start(), self.initial, &dvs).ok_or("Can't construct")
    }
}

impl std::fmt::Display for ManeuverPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Maneuver Plan ({} segments) ({:0.1})",
            self.segments.len(),
            self.dv()
        )?;
        if self.segments.is_empty() {
            return write!(f, " (empty)");
        }
        for (i, segment) in self.segments.iter().enumerate() {
            writeln!(
                f,
                "{}. {:?} {:?} dV {:0.1} to {}",
                i + 1,
                segment.start,
                segment.end,
                segment.impulse,
                segment.orbit,
            )?;
        }
        writeln!(f, "Ending with {}", self.terminal)
    }
}

#[derive(Debug, Clone)]
pub struct ManeuverSegment {
    pub start: Nanotime,
    pub end: Nanotime,
    pub orbit: SparseOrbit,
    pub impulse: DVec2,
}

impl ManeuverSegment {
    fn new(start: Nanotime, end: Nanotime, orbit: SparseOrbit, dv: DVec2) -> Option<Self> {
        Some(ManeuverSegment {
            start,
            end,
            orbit,
            impulse: dv,
        })
    }

    fn next(&self, t: Nanotime, impulse: DVec2) -> Option<Self> {
        let sparse = self.next_orbit()?;
        assert!(self.end < t);
        ManeuverSegment::new(self.end, t, sparse, impulse)
    }

    fn next_orbit(&self) -> Option<SparseOrbit> {
        let pv = self.orbit.pv(self.end).ok()? + PV::vel(self.impulse);
        SparseOrbit::from_pv(pv, self.orbit.body, self.end)
    }

    fn is_valid(&self, stamp: Nanotime) -> bool {
        self.start <= stamp && self.end > stamp
    }

    fn dv(&self) -> (Nanotime, DVec2) {
        (self.end, self.impulse)
    }
}

impl std::fmt::Display for ManeuverSegment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Segment {:?} to {:?} in {:?}, impulse {}",
            self.start, self.end, self.orbit, self.impulse
        )
    }
}

fn hohmann_transfer(
    current: &SparseOrbit,
    destination: &SparseOrbit,
    now: Nanotime,
) -> Option<ManeuverPlan> {
    match current.class() {
        OrbitClass::Parabolic | OrbitClass::Hyperbolic | OrbitClass::VeryThin => return None,
        _ => (),
    }

    let mu = current.body.mu();
    let r1 = current.periapsis_r();
    let r2 = destination.radius_at_angle(current.arg_periapsis + PI_64);
    let a_transfer = (r1 + r2) / 2.0;
    let v1 = vis_viva_equation(mu, r1, a_transfer);

    let t1 = current.t_next_p(now)?;
    let before = current.pv_universal(t1).ok()?;
    let prograde = before.vel.normalize_or_zero();
    let after = PV::from_f64(before.pos, prograde * v1);

    let dv1 = after.vel - before.vel;

    let transfer_orbit = SparseOrbit::from_pv(after, current.body, t1)?;

    let t2 = t1 + transfer_orbit.period()? / 2;
    let before = transfer_orbit.pv_universal(t2).ok()?;
    let (after, _) = destination.nearest(before.pos);
    let after = PV::from_f64(before.pos, after.vel);

    let dv2 = after.vel - before.vel;

    ManeuverPlan::new(now, *current, &[(t1, dv1), (t2, dv2)])
}

#[allow(unused)]
fn bielliptic_transfer(
    current: &SparseOrbit,
    destination: &SparseOrbit,
    now: Nanotime,
) -> Option<ManeuverPlan> {
    match current.class() {
        OrbitClass::Parabolic | OrbitClass::Hyperbolic | OrbitClass::VeryThin => return None,
        _ => (),
    }

    let r1 = current.semi_major_axis;
    let r2 = destination.semi_major_axis;

    let (r1, r2) = (r1.min(r2), r1.max(r2));

    if r2 / r1 < 11.94 {
        // hohmann transfer is always better
        return None;
    }

    // if ratio is greater than 15.58, any bi-elliptic transfer is better

    let rb = current.apoapsis_r().max(destination.apoapsis_r()) * 2.0;

    if rb > current.body.soi * 0.9 {
        return None;
    }

    let intermediate =
        SparseOrbit::circular(rb, current.body, Nanotime::zero(), current.is_retrograde());

    let p1 = hohmann_transfer(current, &intermediate, now)?;

    let intermediate = p1.segments.iter().nth(1)?;

    let p2 = hohmann_transfer(&intermediate.orbit, destination, p1.end())?;

    p1.then(p2).ok()
}

pub fn get_next_intersection(
    stamp: Nanotime,
    eval: &SparseOrbit,
    target: &SparseOrbit,
) -> Result<Option<(Nanotime, PV)>, ConvergeError<Nanotime>> {
    let n = 100;
    let period = eval.period_or(Nanotime::secs(500));
    let teval = tspace(stamp, stamp + period, n);

    let signed_distance_at = |t: Nanotime| {
        let pcurr = eval.pv(t).unwrap_or(PV::INFINITY);
        target.nearest_along_track(pcurr.pos)
    };

    let initial_sign = signed_distance_at(stamp).1 > 0.0;

    let condition = |t: Nanotime| {
        let (_, d) = signed_distance_at(t);
        let cur = d > 0.0;
        cur == initial_sign
    };

    for ts in teval.windows(2) {
        let t1 = ts[0];
        let t2 = ts[1];
        let t = search_condition(t1, t2, Nanotime::nanos(10), &condition)?;
        if let Some(t) = t {
            let (pv, _) = signed_distance_at(t);
            return Ok(Some((t, pv)));
        }
    }

    Ok(None)
}

fn direct_transfer(
    current: &SparseOrbit,
    destination: &SparseOrbit,
    now: Nanotime,
) -> Option<ManeuverPlan> {
    get_next_intersection(now, current, destination)
        .ok()
        .flatten()
        .and_then(|(t, pvf)| {
            let pvi = current.pv(t).ok()?;
            ManeuverPlan::new(now, *current, &[(t, pvf.vel - pvi.vel)])
        })
}

fn generate_maneuver_plans(
    current: &SparseOrbit,
    destination: &SparseOrbit,
    now: Nanotime,
) -> Vec<ManeuverPlan> {
    // if current.is_retrograde() != destination.is_retrograde() {
    //     return vec![];
    // }

    let direct = direct_transfer(current, destination, now);
    let hohmann = hohmann_transfer(current, destination, now);
    // let bielliptic = bielliptic_transfer(current, &destination, now);

    [direct, hohmann].into_iter().flatten().collect()
}

pub fn rendezvous_plan(
    src: &SparseOrbit,
    dst: &SparseOrbit,
    now: Nanotime,
) -> Option<ManeuverPlan> {
    hohmann_transfer(src, dst, now)
}

pub fn best_maneuver_plan(
    current: &SparseOrbit,
    destination: &SparseOrbit,
    now: Nanotime,
) -> Result<ManeuverPlan, &'static str> {
    if current.is_similar(destination) {
        return Err("Orbits are the same");
    }

    let mut plans = generate_maneuver_plans(current, destination, now);
    plans.sort_by_key(|m| (m.dv() * 1000.0) as i32);
    plans.first().cloned().ok_or("No plan")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn maneuver_plan_segments_join(plan: &ManeuverPlan) {
        for segs in plan.segments.windows(2) {
            let s1 = &segs[0];
            let s2 = &segs[1];
            let p1 = s1.orbit.pv(s1.end).unwrap();
            let p2 = s2.orbit.pv(s2.start).unwrap();

            let d = p1.pos.distance(p2.pos);

            assert!(d < 20.0, "Expected difference to be smaller: {}", d);
        }
    }

    fn maneuver_plan_is_continuous(plan: &ManeuverPlan) {
        let mut t = plan.start();
        let t_end = plan.end();

        assert!(plan.pv(t - Nanotime::secs(1)).is_none());
        assert!(plan.pv(t_end + Nanotime::secs(1)).is_none());

        let mut previous = None;

        while t < t_end {
            let pv = plan.pv(t);
            assert!(pv.is_some(), "Expected PV to be Some: {}", t);
            let pv = pv.unwrap();

            let dt = 5.0 / pv.vel.length();
            let dt = Nanotime::secs_f64(dt);

            if let Some(p) = previous {
                let d = (pv - p).pos.length();
                assert!(
                    d < 10.0,
                    "Expected difference to be smaller at time {}: {}\n for plan:\n{}",
                    t,
                    d,
                    plan
                );
            }

            previous = Some(pv);

            t += dt;
        }
    }

    fn random_orbit() -> SparseOrbit {
        let r1 = rand(1000.0, 8000.0) as f64;
        let r2 = rand(1000.0, 8000.0) as f64;
        let argp = rand(0.0, 2.0) as f64 * PI_64;

        let body = Body::with_mass(63.0, 1000.0, 15000.0);

        SparseOrbit::new(r1.max(r2), r1.min(r2), argp, body, Nanotime::zero(), false).unwrap()
    }

    #[test]
    fn random_maneuver_plan() {
        for _ in 0..100 {
            let c = random_orbit();
            let d = random_orbit();

            println!("c: {}", &c);
            println!("d: {}\n", &d);

            let plan = best_maneuver_plan(&c, &d, Nanotime::zero());

            assert!(plan.is_ok(), "Plan is not Ok: {:?}", plan);

            let plan = plan.unwrap();

            maneuver_plan_segments_join(&plan);
            maneuver_plan_is_continuous(&plan);
        }
    }
}
