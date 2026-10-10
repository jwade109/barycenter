use super::*;

pub use glam::{DVec2, DVec3, I8Vec2, IVec2, IVec3, U8Vec2, UVec2, UVec3, Vec2, Vec3};
use names::Generator;
use rand::Rng;

pub const PI: f32 = std::f32::consts::PI;

pub const PI_64: f64 = std::f64::consts::PI;

pub fn rand(min: f32, max: f32) -> f32 {
    rand::rng().random_range(min..max)
}

pub fn chance(pct: f32) -> bool {
    rand(0.0, 1.0) < pct
}

pub fn randint(min: i32, max: i32) -> i32 {
    rand::rng().random_range(min..max)
}

pub fn randvec(min: f32, max: f32) -> Vec2 {
    let rot = Vec2::from_angle(rand(0.0, std::f32::consts::PI * 2.0));
    let mag = rand(min, max);
    rot.rotate(Vec2::new(mag, 0.0))
}

pub fn randvec3(min: f32, max: f32) -> Vec3 {
    let r = rand(min, max);
    let a = rand(0.0, 2.0 * PI);
    let z = rand(-1.0, 1.0);
    let p = Vec3::new(
        (1.0 - z.powi(2)).sqrt() * a.cos(),
        (1.0 - z.powi(2)).sqrt() * a.sin(),
        z,
    );
    r * p
}

pub fn rotate(v: Vec2, angle: f32) -> Vec2 {
    Vec2::from_angle(angle).rotate(v)
}

pub fn rotate_f64(v: DVec2, angle: f64) -> DVec2 {
    DVec2::from_angle(angle).rotate(v)
}

pub fn cross2d(a: impl Into<DVec2>, b: impl Into<DVec2>) -> f64 {
    let a = a.into();
    let b = b.into();
    a.extend(0.0).cross(b.extend(0.0)).z
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

pub fn lerp_f64(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// maps x from the range [a, b] to [p, q]
pub fn linmap(x: f32, a: f32, b: f32, p: f32, q: f32) -> f32 {
    let s = (x - a) / (b - a);
    lerp(p, q, s)
}

/// vector projection, rejection of a onto b
pub fn vproj(a: Vec2, b: Vec2) -> (Vec2, Vec2) {
    let bu = b.normalize_or_zero();
    let proj = a.dot(bu) * bu;
    (proj, a - proj)
}

pub fn vfloor(v: Vec2) -> IVec2 {
    IVec2::new(v.x.floor() as i32, v.y.floor() as i32)
}

pub fn vfloor_f64(v: DVec2) -> IVec2 {
    IVec2::new(v.x.floor() as i32, v.y.floor() as i32)
}

pub fn vceil(v: Vec2) -> IVec2 {
    IVec2::new(v.x.ceil() as i32, v.y.ceil() as i32)
}

pub fn vround(v: Vec2) -> IVec2 {
    IVec2::new(v.x.round() as i32, v.y.round() as i32)
}

pub fn vround_f64(v: DVec2) -> IVec2 {
    IVec2::new(v.x.round() as i32, v.y.round() as i32)
}

pub fn apply<T: Copy, R>(x: &Vec<T>, func: impl Fn(T) -> R) -> Vec<R> {
    x.iter().map(|x| func(*x)).collect()
}

pub fn apply_filter<T: Copy, K, R>(
    x: &Vec<T>,
    func: impl Fn(T) -> Option<(K, R)>,
) -> (Vec<K>, Vec<R>) {
    x.iter().filter_map(|x| func(*x)).collect()
}

pub fn linspace(a: f32, b: f32, n: usize) -> Vec<f32> {
    if n < 2 {
        return vec![a];
    }
    if n == 2 {
        return vec![a, b];
    }
    (0..n)
        .map(|i| {
            let t = i as f32 / (n - 1) as f32;
            lerp(a, b, t)
        })
        .collect()
}

pub fn linspace_f64(a: f64, b: f64, n: usize) -> Vec<f64> {
    if n < 2 {
        return vec![a];
    }
    if n == 2 {
        return vec![a, b];
    }
    (0..n)
        .map(|i| {
            let t = i as f64 / (n - 1) as f64;
            lerp_f64(a, b, t)
        })
        .collect()
}

pub fn tspace(start: Nanotime, end: Nanotime, nsamples: usize) -> Vec<Nanotime> {
    if start > end {
        return Vec::new();
    }
    (0..nsamples)
        .map(|i| start.lerp(end, i as f32 / (nsamples - 1) as f32))
        .collect()
}

pub fn bhaskara_sin_approx(x: f32) -> f32 {
    let xp = x.abs();
    x.signum() * 16.0 * xp * (PI - xp) / (5.0 * PI.powi(2) - 4.0 * xp * (PI - xp))
}

pub fn is_occluded(light_source: Vec2, test: Vec2, object: Vec2, radius: f32) -> bool {
    let test = test - light_source;
    let object = object - light_source;

    //
    //                      * * *
    //                   *      /  *
    //        @ T       *      / r  *
    //                  *     @     *
    //                  *      O    *
    //                   *         *
    //                      * * *
    //
    //   @ L (0, 0)
    //

    let dobj = object.length();

    if object.distance(test) < radius {
        return true;
    }

    if dobj < radius {
        return true;
    }

    if test.length() < dobj {
        return false;
    }

    let angular_radius = (radius / dobj).asin();
    let angle = test.angle_to(object);
    angle.abs() < angular_radius
}

pub fn get_random_name() -> String {
    let mut generator = Generator::default();
    generator.next().unwrap()
}

pub fn wrap_0_2pi(x: f32) -> f32 {
    let twopi = 2.0 * PI;
    x - twopi * (x / twopi).floor()
}

pub fn wrap_0_2pi_f64(x: f64) -> f64 {
    let twopi = 2.0 * PI_64;
    x - twopi * (x / twopi).floor()
}

pub fn wrap_pi_npi(x: f32) -> f32 {
    f32::atan2(x.sin(), x.cos())
}

pub fn wrap_pi_npi_f64(x: f64) -> f64 {
    f64::atan2(x.sin(), x.cos())
}

pub fn rocket_equation(ve: f64, m0: Mass, m1: Mass) -> f64 {
    ve * (m0.to_kg_f64() / m1.to_kg_f64()).ln()
}

pub fn rotate_ccw(p: PartCoord) -> PartCoord {
    IVec2::Y.rotate(p.inner()).into()
}

pub fn rect_area_moment_of_inertia(dims: Vec2) -> f32 {
    dims.x * dims.y / 12.0 * (dims.x.powi(2) + dims.y.powi(2))
}

pub fn rect_area_moment_of_inertia_with_offset(_distance: f32, _dims: Vec2) {
    todo!()
    // let r = dims
}

pub fn mass_after_maneuver(ve: f64, m0: f64, dv: f64) -> f64 {
    m0 / (dv / ve).exp()
}

pub fn get_yaw(transform: Isometry2d) -> f64 {
    transform.rotation
}

// TODO duplicate
pub fn in_frame(transform: Isometry2d, pos: impl Into<DVec2>) -> DVec2 {
    let pos = pos.into();
    let offset = pos - transform.translation;
    let yaw = get_yaw(transform);
    rotate_f64(offset, -yaw)
}

// TODO duplicate
pub fn express_in_frame(frame: Isometry2d, point: impl Into<DVec2>) -> DVec2 {
    let point = point.into();
    let delta = point - frame.translation;
    let x = frame.local_x().dot(delta);
    let y = frame.local_y().dot(delta);
    (x, y).into()
}

pub fn low_pass_f32(actual: f32, target: f32, rate: f32) -> f32 {
    actual + (target - actual) * rate
}

pub fn low_pass_f64(actual: f64, target: f64, rate: f64) -> f64 {
    actual + (target - actual) * rate
}

pub fn body_frame_wrench(
    thrust: f32,
    center_of_thrust: DVec2,
    rotation: Rotation,
    com: DVec2,
) -> Isometry2d {
    let u = rotation.to_dir();
    let lever_arm = center_of_thrust - com;
    let thrust = thrust * u.as_vec2();
    let torque = cross2d(lever_arm, thrust);
    Isometry2d::new(thrust, torque as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_float_eq::assert_float_absolute_eq;

    #[test]
    fn linspace_is_cool() {
        let t = linspace(-0.3, 0.6, 12);

        assert!(t.len() == 12);

        assert_eq!(t[0], -0.3);

        assert_float_absolute_eq!(t[1], -0.21818182);
        assert_float_absolute_eq!(t[2], -0.13636364);
        assert_float_absolute_eq!(t[3], -0.054545447);
        assert_float_absolute_eq!(t[4], 0.027272731);
        assert_float_absolute_eq!(t[5], 0.109090924);
        assert_float_absolute_eq!(t[6], 0.19090912);
        assert_float_absolute_eq!(t[7], 0.27272725);
        assert_float_absolute_eq!(t[8], 0.35454547);
        assert_float_absolute_eq!(t[9], 0.43636364);
        assert_float_absolute_eq!(t[10], 0.51818186);

        assert_eq!(t[11], 0.6);
    }

    #[test]
    fn wrapping() {
        assert_eq!(wrap_0_2pi(-PI), PI);
        assert_eq!(wrap_0_2pi(0.0), 0.0);
        assert_eq!(wrap_0_2pi(2.0 * PI), 0.0);
        assert_eq!(wrap_0_2pi(PI), PI);
        assert!((wrap_0_2pi(3.0 * PI) - PI).abs() < 0.001);
    }

    #[test]
    fn into_isometry_frame() {
        let iso = Isometry2d::new((3.4, 12.1), 0.3);

        let point = Vec2::new(17.3, 20.9);

        let in_frame = in_frame(iso, point);
        let in_frame_2 = express_in_frame(iso, point);

        assert_eq!(in_frame, in_frame_2);

        assert_eq!(in_frame, DVec2::new(15.879754, 4.2992296));
    }
}
