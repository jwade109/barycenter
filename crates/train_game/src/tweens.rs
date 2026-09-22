use std::{cell::RefCell, collections::BTreeMap};

pub fn linear_tween(t: f64) -> f64 {
    t
}

pub fn power_tween(t: f64, p: f64) -> f64 {
    t.powf(p)
}

pub fn ease_in_out_exp(t: f64) -> f64 {
    if t < 0.5 {
        2.0f64.powf(20.0 * t - 10.0) / 2.0
    } else {
        (2.0 - 2.0f64.powf(-20.0 * t + 10.0)) / 2.0
    }
}

pub fn lpf_tween(t: f64) -> f64 {
    11.0 * t / (10.0 * t + 1.0)
}

#[derive(Clone, Copy, Debug)]
pub enum Tween {
    Linear,
    Polynomial(f64),
    Exponential,
    LowPassFilter,
}

struct Animation {
    tween: Tween,
    max_duration: f64,
    actual_duration: f64,
    state: bool,
}

impl Animation {
    fn evaluate(&self) -> f64 {
        let t = self.actual_duration / self.max_duration;
        match self.tween {
            Tween::Linear => linear_tween(t),
            Tween::Polynomial(power) => power_tween(t, power),
            Tween::Exponential => ease_in_out_exp(t),
            Tween::LowPassFilter => lpf_tween(t),
        }
    }
}

pub struct AnimationStates {
    animations: RefCell<BTreeMap<AnimId, Animation>>,
}

impl AnimationStates {
    pub fn new() -> Self {
        Self {
            animations: Default::default(),
        }
    }

    pub fn update(&mut self, dt: f64) {
        for (_, anim) in self.animations.borrow_mut().iter_mut() {
            if anim.state {
                anim.actual_duration += dt;
            } else {
                anim.actual_duration -= dt;
            }
            anim.actual_duration = anim.actual_duration.clamp(0.0, anim.max_duration);
        }

        self.animations
            .borrow_mut()
            .retain(|_, a| a.actual_duration > 0.0 || a.state)
    }

    pub fn animations(&self) -> Vec<(&'static str, u64, Tween, f64)> {
        self.animations
            .borrow()
            .iter()
            .map(|(id, a)| (id.id, id.num, a.tween, a.evaluate()))
            .collect()
    }

    pub fn anim(&self, id: impl Into<AnimId>, tween: Tween, max_duration: f64, state: bool) -> f64 {
        let key = id.into();
        let mut anim = self.animations.borrow_mut();
        let a = anim.entry(key).or_insert(Animation {
            tween,
            max_duration,
            actual_duration: 0.0,
            state,
        });

        a.tween = tween;
        a.max_duration = max_duration;
        a.state = state;

        a.evaluate()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AnimId {
    id: &'static str,
    num: u64,
}

impl Into<AnimId> for u64 {
    fn into(self) -> AnimId {
        AnimId { id: "", num: self }
    }
}
impl Into<AnimId> for usize {
    fn into(self) -> AnimId {
        AnimId { id: "", num: self as u64 }
    }
}

impl Into<AnimId> for (&'static str, u64) {
    fn into(self) -> AnimId {
        AnimId {
            id: self.0,
            num: self.1,
        }
    }
}

impl Into<AnimId> for (&'static str, usize) {
    fn into(self) -> AnimId {
        AnimId {
            id: self.0,
            num: self.1 as u64,
        }
    }
}

impl Into<AnimId> for &'static str {
    fn into(self) -> AnimId {
        AnimId { id: self, num: 0 }
    }
}
