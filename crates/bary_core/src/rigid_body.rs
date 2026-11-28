use crate::math::DVec2;
use crate::prelude::*;

#[derive(Default, Debug, Clone, Copy)]
pub struct BodyFrameAccel {
    pub linear: DVec2,
    pub angular: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct RigidBody {
    pub pv: PV,
    pub angle: f64,
    pub angular_velocity: f64,
}

pub const MAX_ANGULAR_VELOCITY: f64 = 4.0;

impl RigidBody {
    pub const ZERO: RigidBody = RigidBody {
        pv: PV::ZERO,
        angle: 0.0,
        angular_velocity: 0.0,
    };

    pub fn random_spin() -> Self {
        Self {
            pv: PV::ZERO,
            angle: rand(0.0, PI * 2.0) as f64,
            angular_velocity: rand(-0.3, 0.3) as f64,
        }
    }

    pub fn on_sim_tick(&mut self, a: BodyFrameAccel, gravity: DVec2, dt: Nanotime) {
        let linear = if a.linear != DVec2::ZERO {
            rotate_f64(a.linear, self.angle)
        } else {
            DVec2::ZERO
        } + gravity;

        self.angular_velocity += a.angular * dt.to_secs_f64();
        // self.angular_velocity = self
        // .angular_velocity
        // .clamp(-MAX_ANGULAR_VELOCITY, MAX_ANGULAR_VELOCITY);
        self.angle += self.angular_velocity * dt.to_secs_f64();
        self.angle = wrap_0_2pi_f64(self.angle);

        // TODO
        self.pv.vel += linear * dt.to_secs_f64();
        self.pv.pos += self.pv.vel * dt.to_secs_f64();
    }

    pub fn clamp_with_elevation(&mut self, elevation: f64) -> bool {
        let elev = self.pv.pos.length();

        let clamped = elev <= elevation;

        if clamped {
            self.pv.pos = self.pv.pos.normalize_or_zero() * elevation;
            self.pv.vel = DVec2::ZERO;
        }

        clamped
    }
}

pub fn kinematic_apoapis(altitude: f64, vertical_velocity: f64, gravity: f64) -> f64 {
    if vertical_velocity <= 0.0 {
        return altitude;
    }
    altitude + vertical_velocity.powi(2) / (2.0 * gravity.abs())
}
