#![deny(missing_docs)]

use bary_core::prelude::*;
use image::RgbaImage;
use noise::{NoiseFn, Perlin};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use serde::{Deserialize, Serialize};

/// Enumeration of different material types
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum TerrainMaterial {
    /// rock
    Rock,
    /// dirt
    Dirt,
    /// ice
    Ice,
    /// silicon
    Silicon,
    /// iron
    Iron,
}

impl TerrainMaterial {
    fn to_color(&self) -> BColor {
        match self {
            TerrainMaterial::Rock => BColor::gray(120),
            TerrainMaterial::Dirt => BColor::DARKBROWN,
            TerrainMaterial::Ice => BColor::RAYWHITE,
            TerrainMaterial::Silicon => BColor::SILVER,
            TerrainMaterial::Iron => BColor::gray(170),
        }
    }

    /// Produces a randomly selected terrain material (not uniform)
    pub fn random() -> Self {
        let n = randint(0, 9);
        match n {
            0..5 => TerrainMaterial::Rock,
            5 => TerrainMaterial::Dirt,
            6 => TerrainMaterial::Ice,
            7 => TerrainMaterial::Silicon,
            8 => TerrainMaterial::Iron,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct AsteroidShapeParameter {
    amplitude: f32,
    frequency: u32,
    phase: f32,
}

/// Structure representing an Asteroid or large body of solid terrain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asteroid {
    seed: u32,
    base_radius: f32,
    parameters: Vec<AsteroidShapeParameter>,
    deposits: Vec<(Vec2, f32, TerrainMaterial)>,
    craters: Vec<(Vec2, f32)>,
}

impl Asteroid {
    /// Produces a random asteroid given a base radius and seed.
    pub fn random(base_radius: f32, seed: Option<u32>) -> Self {
        let seed = seed.unwrap_or(randint(1000, 1000000) as u32);
        let mut rng = StdRng::seed_from_u64(seed as u64);
        let n_params = rng.random_range(5..=8);
        let mut frequency = 1;
        let parameters = (0..n_params)
            .into_iter()
            .map(|_| {
                let amplitude = rng.random_range(0.0..=0.5) / frequency as f32;
                let phase = rng.random_range(0.0..=2.0 * PI);
                let param: AsteroidShapeParameter = AsteroidShapeParameter {
                    amplitude,
                    frequency,
                    phase,
                };
                frequency += rng.random_range(2..=5);
                param
            })
            .collect();

        let n_deposits = rng.random_range(8..30);

        let mut s = Self {
            seed,
            base_radius,
            parameters,
            deposits: Vec::new(),
            craters: Vec::new(),
        };

        let max_r = s.max_radius();
        let n_craters = rng.random_range(4..28);

        for _ in 0..n_craters {
            let x = rng.random_range(-max_r..=max_r);
            let y = rng.random_range(-max_r..=max_r);
            let r = rng.random_range(10.0..(max_r / 3.0).max(12.0));
            s.craters.push((Vec2::new(x, y), r))
        }

        let deposits = (0..n_deposits)
            .map(|_| {
                let theta = rng.random_range(0.0..=2.0 * PI);
                let r_max = s.radius_at(theta);
                let r = rng.random_range(2.0..=r_max - 1.0);
                let size = rng.random_range(3.0..=base_radius * 0.3);
                let p = rotate(Vec2::X * r, theta);

                let idx = rng.random_range(0..4);
                let mat = match idx {
                    0 => TerrainMaterial::Dirt,
                    1 => TerrainMaterial::Ice,
                    2 => TerrainMaterial::Silicon,
                    _ => TerrainMaterial::Iron,
                };
                (p, size, mat)
            })
            .collect();

        s.deposits = deposits;

        s
    }

    /// The seed that this asteroid was generated with.
    pub fn seed(&self) -> u32 {
        self.seed
    }

    /// The direction of the surface normal vector at a particular
    /// point on this Asteroid.
    pub fn face_dir(&self, p: Vec2) -> Vec3 {
        let theta = p.to_angle();
        let r = self.base_radius;
        let d = p.length();
        let angle = d / r * 0.5 * PI;
        let xy = rotate(Vec2::X, theta) * angle.sin();
        let z = angle.cos();
        xy.extend(z).normalize_or_zero()
    }

    /// The base radius of the asteroid. Terrain height is generated
    /// as a deviation from this radius.
    pub fn base_radius(&self) -> f32 {
        self.base_radius
    }

    /// Gets the radius of this asteroid at a particular CCW angle from +X.
    pub fn radius_at(&self, theta: f32) -> f32 {
        let mut r = 1.0;
        for param in &self.parameters {
            r += param.amplitude * (param.frequency as f32 * theta + param.phase).cos()
        }
        r * self.base_radius
    }

    /// Gets the largest possible radius of this asteroid
    /// given its shape parameters.
    pub fn max_radius(&self) -> f32 {
        let mut r = 1.0;
        for param in &self.parameters {
            r += param.amplitude;
        }
        r * self.base_radius
    }

    /// Gets the smallest possible radius of this asteroid
    /// given its shape parameters.
    pub fn min_radius(&self) -> f32 {
        let mut r = 1.0;
        for param in &self.parameters {
            r -= param.amplitude;
        }
        r * self.base_radius
    }

    /// Determines whether the given point is "inside" the
    /// terrain defined by this asteroid.
    pub fn contains(&self, p: Vec2) -> bool {
        self.signed_distance(p) >= 0.0
    }

    /// Definition for an SDF that returns a positive distance if
    /// inside the asteroid, a negative distance if outside, and
    /// zero if directly on the asteroid's undisturbed surface.
    pub fn signed_distance(&self, p: Vec2) -> f32 {
        let theta = p.to_angle();
        let r = p.length();
        let r_ast = self.radius_at(theta);
        r_ast - r
    }

    /// Get the material present at this location on the asteroid.
    pub fn material_at(&self, p: Vec2) -> Option<TerrainMaterial> {
        if let Some(deposit) = self.get_deposit(p) {
            return Some(deposit);
        }

        self.contains(p).then_some(TerrainMaterial::Rock)
    }

    /// Gets the material value of the deposit in this location,
    /// if any exists.
    pub fn get_deposit(&self, p: Vec2) -> Option<TerrainMaterial> {
        if self.noise_c(p) > 0.0 {
            return None;
        }
        self.deposits.iter().find_map(|(c, r, value)| {
            let p = p - c;
            (p.length() < *r).then_some(*value)
        })
    }

    fn perlin(&self) -> Perlin {
        // TODO(gross) this is probably pretty expensive,
        // so it would be great to cache this somehow.
        // Perlin can't be serialized however, and probably shouldn't be.
        Perlin::new(self.seed)
    }

    /// Noise expression A. Lowest frequency.
    pub fn noise_a(&self, p: Vec2) -> f32 {
        let scale_1 = 1000.0;
        self.perlin()
            .get([p.x as f64 / scale_1, p.y as f64 / scale_1, 0.0]) as f32
    }

    /// Noise expression B. Highest frequency.
    pub fn noise_b(&self, p: Vec2) -> f32 {
        let scale_2 = 30.0;
        self.perlin()
            .get([p.x as f64 / scale_2, p.y as f64 / scale_2, 0.0]) as f32
    }

    /// Noise expression C. Medium frequency.
    pub fn noise_c(&self, p: Vec2) -> f32 {
        let scale = 60.0;
        self.perlin()
            .get([p.x as f64 / scale, p.y as f64 / scale, 0.0]) as f32
    }

    /// A sum of Noise A and Noise B.
    pub fn noise(&self, p: Vec2) -> f32 {
        let n1 = self.noise_a(p);
        let n2 = self.noise_b(p);
        n1 + n2
    }

    /// Gets the color of the surface at the given point.
    pub fn sample_color(&self, p: Vec2, highlight_deposits: bool) -> Option<BColor> {
        if !self.contains(p) {
            return None;
        }
        let n = self.noise(p);

        let mut c = if n < 0.0 {
            if let Some(v) = self.get_deposit(p) {
                if highlight_deposits {
                    [255, 100, 0, 255]
                } else {
                    v.to_color().to_u8()
                }
            } else {
                [105, 105, 105, 255]
            }
        } else if n < 0.8 {
            [110, 110, 110, 255]
        } else {
            [115, 115, 115, 255]
        };

        let crater_delta = (n > 0.2)
            .then(|| {
                self.craters.iter().find_map(|(c, r)| {
                    let d = c.distance(p);
                    ((d - r).abs() < 4.0).then_some([10, 10, 10])
                })
            })
            .flatten();

        if let Some(delta) = crater_delta {
            for i in 0..3 {
                if c[i] as i32 + delta[i] as i32 <= 255 {
                    c[i] += delta[i];
                } else {
                    c[i] = 255;
                }
            }
        }

        Some(BColor::from_u8(c))
    }

    /// Check whether this asteroid contains a particular zone.
    pub fn contains_zone(&self, z: Zone) -> bool {
        self.zones().find(|o| *o == z).is_some()
    }

    /// Get an iterator over the zones in this asteroid.
    pub fn zones(&self) -> impl Iterator<Item = Zone> + use<'_> {
        let size = 100;
        let max_r = self.max_radius();
        let n = (max_r as f64 / size as f64).ceil() as i32;
        (-n..=n).flat_map(move |x| {
            (-n..=n).map(move |y| Zone {
                size,
                index: IVec2 { x, y },
            })
        })
    }
}

/// A portion of an asteroid.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Deserialize, Serialize)]
pub struct Zone {
    size: u32,
    index: IVec2,
}

impl Zone {
    /// Get the axis-aligned bounding box for this zone.
    pub fn aabb(&self) -> AABB {
        let lower = self.size as f64 * self.index.as_dvec2();
        let upper = self.size as f64 * (self.index + IVec2::ONE).as_dvec2();

        // TODO stupid cast

        AABB::from_arbitrary(lower.as_vec2(), upper.as_vec2())
    }
}

/// Get the zone corresponding with a given location and scale factor.
pub fn get_zone(pos: DVec2, size: u32) -> Zone {
    let index = vfloor_f64(pos / size as f64);
    Zone { size, index }
}

fn write_pixel(img: &mut RgbaImage, p: IVec2, color: BColor) {
    if p.x < 0 || p.y < 0 || p.x >= img.width() as i32 || p.y >= img.height() as i32 {
        return;
    }
    let c = color.to_u8();
    if let Some(pixel) =
        img.get_pixel_mut_checked(p.x as u32, (img.height() as i32 - p.y - 1) as u32)
    {
        pixel.0 = c;
    }
}

fn attenuate_light(color: BColor, dot: f32) -> BColor {
    let mut c = color.to_u8();
    let dot = dot.max(0.04);
    for i in 0..3 {
        // if dot < 0.8 {
        let new_color = (c[i] as f32 * dot).clamp(0.0, 255.0);
        c[i] = new_color as u8;
        // }
    }
    BColor::from_u8(c)
}

/// Construct an image representation of the given asteroid.
pub fn make_asteroid_image(
    ast: &Asteroid,
    viewport: AABB,
    width: u32,
    light_dir: Option<f32>,
    debug_info: bool,
    highlight_deposits: bool,
) -> RgbaImage {
    let max_r = ast.max_radius();
    let min_r = ast.min_radius();
    let height = (width as f32 * viewport.span.y / viewport.span.x) as u32;

    let mut img = RgbaImage::from_pixel(width, height, image::Rgba([0, 0, 0, 0]));

    let world_to_img = |p: Vec2| -> IVec2 {
        let u = viewport.to_normalized(p);
        vfloor(u * Vec2::new(width as f32, height as f32))
    };

    for w in 0..width {
        let sx = ((w as f32 + 0.5)) / width as f32;
        for h in 0..height {
            let sy = ((h as f32 + 0.5)) / height as f32;
            let p = viewport.from_normalized(Vec2::new(sx, sy));
            let color = match ast.sample_color(p, highlight_deposits) {
                Some(c) => c,
                None => continue,
            };

            let color = if let Some(a) = light_dir {
                let light_dir = rotate(Vec2::X, a).normalize_or_zero().extend(0.0);
                let face_dir = ast.face_dir(p);
                let dot = light_dir.dot(-face_dir);
                attenuate_light(color, dot)
            } else {
                color
            };

            write_pixel(&mut img, UVec2::new(w, h).as_ivec2(), color);
        }
    }

    if debug_info {
        for x in (0..(max_r.ceil() as u32)).step_by(50) {
            for x in [-(x as f32), x as f32] {
                let p = Vec2::new(x, 0.0);
                if ast.contains(p) {
                    let q = world_to_img(p);
                    write_pixel(&mut img, q, BColor::from_u8([70, 255, 70, 255]));
                }
            }
        }

        for y in (0..(max_r.ceil() as u32)).step_by(50) {
            for y in [-(y as f32), y as f32] {
                let p = Vec2::new(0.0, y);
                if ast.contains(p) {
                    let q = world_to_img(p);
                    write_pixel(&mut img, q, BColor::from_u8([70, 255, 70, 255]));
                }
            }
        }

        for theta in linspace(0.0, 2.0 * PI, 100) {
            let p = rotate(Vec2::X * max_r, theta);
            let q = world_to_img(p);
            write_pixel(&mut img, q, BColor::from_u8([255, 50, 50, 255]));

            let p = rotate(Vec2::X * min_r, theta);
            let q = world_to_img(p);
            write_pixel(&mut img, q, BColor::from_u8([255, 50, 50, 255]));

            let r = ast.radius_at(theta);
            let p = rotate(Vec2::X * r, theta);
            let q = world_to_img(p);
            write_pixel(&mut img, q, BColor::from_u8([50, 255, 255, 255]));
        }
    }

    img
}
