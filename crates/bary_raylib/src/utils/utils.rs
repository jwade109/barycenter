use bary_core::prelude::*;
use bary_sim::Camera;
use raylib::prelude::*;

pub fn glam_to_raylib(v: Vec2) -> Vector2 {
    Vector2::new(v.x, v.y)
}

pub fn glam_to_raylib_swap_y(v: impl Into<DVec2>) -> Vector2 {
    let v = v.into().as_vec2();
    Vector2::new(v.x, -v.y)
}

pub fn raylib_to_glam(v: Vector2) -> Vec2 {
    Vec2::new(v.x, v.y)
}

pub fn raylib_to_glam_invert_y(v: Vector2) -> Vec2 {
    Vec2::new(v.x, -v.y)
}

pub fn default_camera_2d() -> Camera2D {
    Camera2D {
        offset: Vector2::zero(),
        target: Vector2::zero(),
        rotation: 0.0,
        zoom: 1.0,
    }
}

pub fn screen_to_world(camera: &Camera, screen_pos: Vec2, screen_dims: Vec2) -> DVec2 {
    // this is gross
    let offset = screen_dims / 2.0;
    let delta = raylib_to_glam_invert_y(glam_to_raylib(screen_pos) - glam_to_raylib(offset))
        .as_dvec2()
        / camera.zoom as f64;
    rotate_f64(delta, camera.isometry.rotation)
        + raylib_to_glam(glam_to_raylib(camera.isometry.translation.as_vec2())).as_dvec2()
}

pub fn get_world_to_screen(
    camera: &Camera,
    world_pos: impl Into<DVec2>,
    screen_dims: Vec2,
) -> Vec2 {
    let world_pos = world_pos.into().as_vec2();
    let offset = screen_dims / 2.0;
    let p = (world_pos - camera.isometry.translation.as_vec2()) * camera.zoom;
    rotate(p.with_y(-p.y), camera.isometry.rotation as f32) + offset
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_to_world_conversion() {
        let screen_dims = Vec2::new(1080.0, 720.0);

        let center = screen_dims / 2.0;

        let target = DVec2::new(100.0, 120.0);

        let camera = Camera {
            isometry: Isometry2d::new(target, 34.0f32.to_radians()),
            zoom: 1.2,
        };

        let screen_pos = center;
        let world_pos = screen_to_world(&camera, screen_pos, screen_dims);
        assert_eq!(world_pos, target);

        let screen_pos = Vec2::new(560.0, 293.0);
        let world_pos = screen_to_world(&camera, screen_pos, screen_dims);
        assert_eq!(world_pos, DVec2::new(82.59569, 175.60782));
    }
}
