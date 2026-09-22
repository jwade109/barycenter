use crate::into_gpu::IntoGpu;
use bary_core::prelude::Isometry2d;
use glam::{DVec2, DVec4, Mat4, Quat, Vec2, Vec2Swizzles, Vec3, Vec4};

pub struct Transform32 {
    inner: Mat4,
}

impl Transform32 {
    pub fn from_iso(iso: impl Into<Isometry2d>, dims: DVec2, screen_dims: DVec2) -> Self {
        let scale = 2.0 * dims / screen_dims;
        let ar = screen_dims.y as f32 / screen_dims.x as f32;
        let iso = iso.into();
        let pos = iso.tr() / screen_dims * 2.0;
        let rotation = Quat::from_rotation_z(iso.rotation as f32);
        let rot = Mat4::from_rotation_translation(rotation, Vec3::ZERO);
        let tr = Mat4::from_translation(Vec3::new(pos.x as f32 - 1.0, pos.y as f32 - 1.0, 0.0));
        let ar_scale = Mat4::from_scale(Vec3::new(ar, 1.0, 1.0));
        let dim_scale = Mat4::from_scale(scale.as_vec2().extend(1.0));
        Self {
            inner: tr * dim_scale * rot,
        }
    }

    pub fn identity() -> Self {
        Self {
            inner: Mat4::IDENTITY,
        }
    }
}

impl IntoGpu for Transform32 {
    const LAYOUT_SIZE: usize = 64;

    fn into_gpu(&self) -> Vec<u8> {
        let arr = [
            self.inner.x_axis.x,
            self.inner.x_axis.y,
            self.inner.x_axis.z,
            self.inner.x_axis.w,
            self.inner.y_axis.x,
            self.inner.y_axis.y,
            self.inner.y_axis.z,
            self.inner.y_axis.w,
            self.inner.z_axis.x,
            self.inner.z_axis.y,
            self.inner.z_axis.z,
            self.inner.z_axis.w,
            self.inner.w_axis.x,
            self.inner.w_axis.y,
            self.inner.w_axis.z,
            self.inner.w_axis.w,
        ];

        arr.into_iter()
            .map(|f| f.to_le_bytes())
            .collect::<Vec<[u8; 4]>>()
            .concat()
    }
}
