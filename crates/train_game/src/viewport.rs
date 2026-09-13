use bary_core::prelude::Isometry2d;
use bary_sim::Camera;
use glam::{DVec2, DVec3, Vec3Swizzles};

pub struct Viewport {
    camera: Camera,
    screen_width: DVec2,
}

impl Viewport {
    pub fn new(camera: Camera, screen_width: DVec2) -> Self {
        Self {
            camera,
            screen_width,
        }
    }

    pub fn is_on_screen(&self, p: impl Into<DVec2>) -> bool {
        let p = p.into() / self.screen_width;
        p.min_element() > 0.0 && p.max_element() < 1.0
    }

    pub fn world_to_screen(&self, p: impl Into<DVec2>) -> DVec2 {
        self.camera.world_to_screen(p.into(), self.screen_width)
    }

    pub fn world_to_screen_parallax(&self, p: impl Into<DVec3>) -> DVec2 {
        let p = p.into();
        let xy = p.xy();
        let off = (xy - self.camera.isometry.tr()) * p.z;

        self.camera.world_to_screen(xy + off, self.screen_width)
    }

    pub fn screen_to_world(&self, p: impl Into<DVec2>) -> DVec2 {
        self.camera.screen_to_world(p.into(), self.screen_width)
    }

    pub fn zoom(&self) -> f64 {
        self.camera.zoom as f64
    }

    pub fn meters(&self, x: f64) -> f64 {
        self.zoom() * x
    }

    pub fn pixels(&self, x: f64) -> f64 {
        x / self.zoom()
    }

    pub fn dims(&self) -> DVec2 {
        self.screen_width
    }

    pub fn w2s_iso(&self, iso: Isometry2d) -> Isometry2d {
        let p = self.world_to_screen(iso.translation.as_dvec2());
        let angle = iso.rotation - self.camera.isometry.rotation;
        Isometry2d::new(p.as_vec2(), angle)
    }
}
