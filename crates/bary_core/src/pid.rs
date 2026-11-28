use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
pub struct PDCtrl {
    pub kp: f64,
    pub kd: f64,
}

impl PDCtrl {
    pub const fn new(kp: f64, kd: f64) -> Self {
        Self { kp, kd }
    }

    pub fn apply(&self, error: f64, error_rate: f64) -> f64 {
        error * self.kp - error_rate * self.kd
    }
}
