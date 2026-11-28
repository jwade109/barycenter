use crate::orbits::{GlobalOrbit, SparseOrbit};
use bary_core::prelude::*;
use crate::scenario::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum EventType {
    Collide(Ent),
    Escape(Ent),
    Encounter(Ent),
    Impulse(Vec2),
    NumericalError,
}

#[derive(Debug, Clone, Copy)]
pub enum ConvergeError<T> {
    Initial((T, bool), (T, bool)),
    Final((T, bool), (T, bool)),
    MaxIter,
}

pub trait BinarySearchKey
where
    Self: Copy + std::ops::Sub<Output = Self> + std::ops::Add<Output = Self>,
    Self: std::cmp::PartialOrd,
    Self: std::ops::Mul<f32, Output = Self>,
{
}

impl<T> BinarySearchKey for T
where
    T: Copy + std::ops::Sub<Output = T> + std::ops::Add<Output = T>,
    T: std::cmp::PartialOrd,
    T: std::ops::Mul<f32, Output = T>,
{
}

// determines timestamp where condition goes from true to false
pub(crate) fn binary_search_recurse<T: BinarySearchKey>(
    start: (T, bool),
    end: (T, bool),
    tol: T,
    cond: impl Fn(T) -> bool,
    remaining: usize,
) -> Result<T, ConvergeError<T>> {
    if remaining == 0 {
        return Err(ConvergeError::MaxIter);
    }

    if end.0 - start.0 < tol {
        return Ok(end.0);
    }

    let midpoint = start.0 + (end.0 - start.0) * 0.5;
    let a = start.1;
    let b = cond(midpoint);
    let c = end.1;

    if !a {
        Err(ConvergeError::Initial(start, end))
    } else if a && !b {
        binary_search_recurse(start, (midpoint, b), tol, cond, remaining - 1)
    } else if b && !c {
        binary_search_recurse((midpoint, b), end, tol, cond, remaining - 1)
    } else {
        Err(ConvergeError::Final(start, end))
    }
}

pub(crate) fn binary_search<T: BinarySearchKey>(
    start: T,
    end: T,
    tol: T,
    max_iters: usize,
    cond: &impl Fn(T) -> bool,
) -> Result<T, ConvergeError<T>> {
    let a = cond(start);
    let c = cond(end);
    binary_search_recurse((start, a), (end, c), tol, cond, max_iters)
}

#[derive(Debug, Clone, Copy)]
pub enum PredictError<T: BinarySearchKey> {
    Lookup,
    TooManyIterations,
    BadPosition,
    Collision(ConvergeError<T>),
    Escape(ConvergeError<T>),
    Encounter(ConvergeError<T>),
}

fn mutual_separation(o1: &SparseOrbit, o2: &SparseOrbit, t: Nanotime) -> f64 {
    let p1 = o1.pv(t).expect("mutual_separation is poorly written").pos;
    let p2 = o2.pv(t).expect("mutual_separation is poorly written").pos;
    p1.distance(p2)
}

/// determines where the condition goes from true to false, if ever
pub(crate) fn search_condition<T: BinarySearchKey>(
    t1: T,
    t2: T,
    tol: T,
    cond: &impl Fn(T) -> bool,
) -> Result<Option<T>, ConvergeError<T>> {
    let a = cond(t1);
    if !a {
        return Ok(None);
    }
    let b = cond(t2);
    if b {
        return Ok(None);
    }
    binary_search::<T>(t1, t2, tol, 100, cond).map(|t| Some(t))
}

#[derive(Debug, Clone)]
pub(crate) enum BadObjectNextState {
    Lookup,
    BadOrbit,
    BadPosition,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum HorizonState {
    Continuing(Nanotime),
    Indefinite,
    Transition(Nanotime, EventType),
    Terminating(Nanotime, EventType),
}

impl HorizonState {
    pub fn is_change(&self) -> bool {
        match self {
            HorizonState::Continuing(_) | HorizonState::Indefinite => false,
            HorizonState::Terminating(_, _) | HorizonState::Transition(_, _) => true,
        }
    }

    pub fn end(&self) -> Option<Nanotime> {
        match self {
            HorizonState::Indefinite => None,
            HorizonState::Continuing(t)
            | HorizonState::Terminating(t, _)
            | HorizonState::Transition(t, _) => Some(*t),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Propagator {
    pub orbit: GlobalOrbit,
    pub start: Nanotime,
    pub dt: Nanotime,
    pub horizon: HorizonState,
}

impl std::fmt::Display for Propagator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?}, {:?}, {:?}, {}",
            self.start, self.dt, self.horizon, self.orbit,
        )
    }
}

impl Propagator {
    pub fn new(orbit: GlobalOrbit, stamp: Nanotime) -> Self {
        Propagator {
            orbit,
            start: stamp,
            dt: Nanotime::zero(),
            horizon: HorizonState::Continuing(stamp),
        }
    }

    pub fn pv(&self, stamp: Nanotime) -> Option<PV> {
        self.is_active(stamp)
            .then(|| self.orbit.1.pv(stamp).ok())
            .flatten()
    }

    pub fn pv_universal(&self, stamp: Nanotime) -> Option<PV> {
        self.is_active(stamp)
            .then(|| self.orbit.1.pv_universal(stamp).ok())
            .flatten()
    }

    pub fn is_indefinite(&self) -> bool {
        match self.horizon {
            HorizonState::Indefinite => true,
            _ => false,
        }
    }

    pub(crate) fn is_active(&self, stamp: Nanotime) -> bool {
        self.start <= stamp && self.end().unwrap_or(stamp) >= stamp
    }

    pub(crate) fn calculated_to(&self, stamp: Nanotime) -> bool {
        match self.horizon {
            HorizonState::Terminating(_, _) => true,
            HorizonState::Indefinite => true,
            HorizonState::Transition(_, _) => true,
            HorizonState::Continuing(end) => end >= stamp,
        }
    }

    pub(crate) fn is_err(&self) -> bool {
        match self.horizon {
            HorizonState::Terminating(_, EventType::NumericalError) => true,
            _ => false,
        }
    }

    pub fn parent(&self) -> Ent {
        self.orbit.0
    }

    pub fn event(&self) -> Option<EventType> {
        match self.horizon {
            HorizonState::Continuing(_) | HorizonState::Indefinite => None,
            HorizonState::Terminating(_, e) | HorizonState::Transition(_, e) => Some(e),
        }
    }

    pub fn stamped_event(&self) -> Option<(Nanotime, EventType)> {
        match self.horizon {
            HorizonState::Continuing(_) | HorizonState::Indefinite => None,
            HorizonState::Terminating(t, e) | HorizonState::Transition(t, e) => Some((t, e)),
        }
    }

    pub fn end(&self) -> Option<Nanotime> {
        self.horizon.end()
    }

    pub fn finish_or_compute_until(
        &mut self,
        stamp: Nanotime,
        bodies: &[(Ent, &SparseOrbit, f64)],
    ) -> Result<(), PredictError<Nanotime>> {
        while !self.calculated_to(stamp) {
            let e = self.next(bodies);
            e?;
        }
        Ok(())
    }

    pub(crate) fn next_prop(
        &self,
        planets: &PlanetarySystem,
    ) -> Result<Option<Propagator>, BadObjectNextState> {
        let (stamp, e) = match self.horizon {
            HorizonState::Transition(stamp, e) => (stamp, e),
            _ => return Ok(None),
        };

        match e {
            EventType::Collide(_) => Ok(None),
            EventType::NumericalError => Ok(None),
            EventType::Escape(_) => {
                let cur = planets
                    .lookup_planet(self.orbit.0, stamp)
                    .ok_or(BadObjectNextState::Lookup)?;
                let reparent = match cur.2 {
                    Some(id) => id,
                    None => return Ok(None),
                };
                let new = planets
                    .lookup_planet(reparent, stamp)
                    .ok_or(BadObjectNextState::Lookup)?;
                let pv = self
                    .orbit
                    .1
                    .pv(stamp)
                    .map_err(|_| BadObjectNextState::BadPosition)?;
                let dv = cur.1 - new.1;
                let orbit = SparseOrbit::from_pv(pv + dv, new.0.body, stamp)
                    .ok_or(BadObjectNextState::BadOrbit)?;
                Ok(Some(Propagator::new(GlobalOrbit(reparent, orbit), stamp)))
            }
            EventType::Encounter(id) => {
                let cur = planets
                    .lookup_planet(self.orbit.0, stamp)
                    .ok_or(BadObjectNextState::Lookup)?;
                let new = planets
                    .lookup_planet(id, stamp)
                    .ok_or(BadObjectNextState::Lookup)?;
                let pv = self
                    .orbit
                    .1
                    .pv(stamp)
                    .map_err(|_| BadObjectNextState::BadPosition)?;
                let dv = cur.1 - new.1;
                let orbit = SparseOrbit::from_pv(pv + dv, new.0.body, stamp)
                    .ok_or(BadObjectNextState::BadOrbit)?;
                Ok(Some(Propagator::new(GlobalOrbit(id, orbit), stamp)))
            }
            EventType::Impulse(dv) => {
                let pv = self
                    .orbit
                    .1
                    .pv(stamp)
                    .map_err(|_| BadObjectNextState::BadPosition)?;
                let orbit = SparseOrbit::from_pv(pv + PV::vel(dv), self.orbit.1.body, stamp)
                    .ok_or(BadObjectNextState::BadOrbit)?;
                Ok(Some(Propagator::new(
                    GlobalOrbit(self.orbit.0, orbit),
                    stamp,
                )))
            }
        }
    }

    pub fn next(
        &mut self,
        bodies: &[(Ent, &SparseOrbit, f64)],
    ) -> Result<(), PredictError<Nanotime>> {
        let end = match self.horizon {
            HorizonState::Continuing(end) => end,
            _ => return Ok(()),
        };

        if end - self.start < Nanotime::mins(5) {
            // debounce for bad position precision
            // TODO fix
            self.horizon = HorizonState::Continuing(self.start + Nanotime::mins(5));
            return Ok(());
        }

        let will_never_encounter = bodies.iter().all(|(_, orbit, soi)| {
            let rmin = orbit.periapsis_r() - *soi;
            let rmax = orbit.apoapsis_r() + *soi;
            self.orbit.1.apoapsis_r() < rmin || self.orbit.1.periapsis_r() > rmax
        });

        if !self.orbit.1.is_suborbital()
            && !self.orbit.1.will_escape()
            && (will_never_encounter || bodies.is_empty())
        {
            // nothing will ever happen to this orbit
            self.horizon = HorizonState::Indefinite;
            return Ok(());
        }

        let tol = Nanotime::nanos(5);

        let alt = self
            .orbit
            .1
            .pv(end)
            .map_err(|_| PredictError::BadPosition)?
            .pos
            .length();

        let pv = match self.orbit.1.pv(end).ok() {
            Some(pv) => pv,
            None => {
                self.horizon = HorizonState::Terminating(end, EventType::NumericalError);
                return Ok(());
            }
        };

        let going_down = pv.pos.normalize_or_zero().dot(pv.vel) < 0.0;

        let below_all_bodies = bodies.iter().all(|(_, orbit, soi)| {
            let rmin = orbit.periapsis_r() - *soi;
            pv.pos.length() < rmin
        });

        let above_planet = |t: Nanotime| {
            let pos = self.orbit.1.pv(t).unwrap_or(PV::INFINITY).pos;
            pos.length() > self.orbit.1.body.radius
        };

        let beyond_soi = |t: Nanotime| {
            let pos = self.orbit.1.pv(t).unwrap_or(PV::INFINITY).pos;
            pos.length() > self.orbit.1.body.soi
        };

        if beyond_soi(end) {
            self.horizon = HorizonState::Transition(end, EventType::Escape(self.orbit.0));
            return Ok(());
        }

        if self.orbit.1.is_suborbital() && going_down && below_all_bodies
            && let Some(tp) = self.orbit.1.t_next_p(end)
                && let Some(t) = search_condition(
                    tp - Nanotime::secs(2),
                    tp,
                    Nanotime::millis(5),
                    &above_planet,
                )
                .ok()
                .flatten()
                {
                    self.horizon = HorizonState::Terminating(t, EventType::Collide(self.orbit.0));
                    return Ok(());
                }

        let might_hit_planet =
            self.orbit.1.is_suborbital() && alt < self.orbit.1.body.radius * 20.0;
        let can_escape = self.orbit.1.will_escape();
        let near_body = bodies
            .iter()
            .any(|(_, orb, soi)| mutual_separation(&self.orbit.1, orb, end) < soi * 3.0);

        self.dt = if might_hit_planet {
            Nanotime::hours(1)
        } else if can_escape {
            Nanotime::hours(10)
        } else if near_body {
            Nanotime::hours(1)
        } else {
            Nanotime::hours(12)
        };

        let t1 = end;
        let t2 = end + self.dt;

        if self.orbit.1.pv(t1).ok().zip(self.orbit.1.pv(t2).ok()) == None {
            self.horizon = HorizonState::Terminating(t1, EventType::NumericalError);
            return Ok(());
        };

        let escape_soi = |t: Nanotime| {
            let pos = self.orbit.1.pv(t).unwrap_or(PV::INFINITY).pos;
            pos.length() < self.orbit.1.body.soi
        };

        if might_hit_planet {
            if !above_planet(t1) {
                self.horizon = HorizonState::Terminating(t1, EventType::Collide(self.orbit.0));
                return Ok(());
            }

            if let Some(t) = search_condition::<Nanotime>(t1, t2, tol, &above_planet)
                .map_err(PredictError::Collision)?
            {
                if t - self.start < Nanotime::millis(10) {
                    self.horizon = HorizonState::Continuing(t2);
                    return Ok(());
                }
                self.horizon = HorizonState::Terminating(t, EventType::Collide(self.orbit.0));
                return Ok(());
            }
        }

        if can_escape
            && let Some(t) = search_condition::<Nanotime>(t1, t2, tol, &escape_soi)
                .map_err(PredictError::Escape)?
            {
                if t - self.start < Nanotime::millis(10) {
                    self.horizon = HorizonState::Continuing(t2);
                    return Ok(());
                }
                self.horizon = HorizonState::Transition(t, EventType::Escape(self.orbit.0));
                return Ok(());
            }

        if near_body {
            for i in 0..bodies.len() {
                let (_, orbit, soi) = bodies[i];
                let cond = separation_with(&self.orbit.1, orbit, soi);
                let id = bodies[i].0;

                if t1 != self.start && !cond(t1) {
                    self.horizon = HorizonState::Transition(t1, EventType::Encounter(id));
                    return Ok(());
                }

                if let Some(t) = search_condition::<Nanotime>(t1, t2, tol, &cond)
                    .map_err(PredictError::Encounter)?
                {
                    self.horizon = HorizonState::Transition(t, EventType::Encounter(id));
                    return Ok(());
                }
            }
        }

        self.horizon = HorizonState::Continuing(t2);
        Ok(())
    }
}

pub(crate) fn separation_with<'a>(
    ego: &'a SparseOrbit,
    planet: &'a SparseOrbit,
    soi: f64,
) -> impl Fn(Nanotime) -> bool + use<'a> {
    move |t: Nanotime| mutual_separation(ego, planet, t) > soi
}
