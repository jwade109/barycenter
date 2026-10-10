use bary_core::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Camera {
    pub isometry: Isometry2d,
    /// camera zoom in raylib units, or whatever
    pub zoom: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            isometry: Isometry2d::ZERO,
            zoom: 1.0,
        }
    }
}

impl Camera {
    pub fn world_to_screen(&self, p: DVec2, screen_width: DVec2) -> DVec2 {
        let delta = p - self.isometry.translation;
        let delta = rotate_f64(delta, -self.isometry.rotation as f64);
        let scaled = delta * self.zoom as f64;
        scaled + screen_width / 2.0
    }

    pub fn screen_to_world(&self, p: DVec2, screen_width: DVec2) -> DVec2 {
        let p = p - screen_width / 2.0;
        let delta = p / self.zoom as f64;
        let delta = rotate_f64(delta, self.isometry.rotation as f64);
        delta + self.isometry.translation
    }
}
