use image::{GenericImage, GenericImageView, ImageBuffer, RgbaImage};
use std::path::Path;

use bary_core::prelude::*;
use clap::Parser;
use image::{ColorType, DynamicImage};

pub struct Particle {
    pos: Vec2,
    vel: Vec2,
    remaining_time: f32,
    color: BColor,
}

fn random_color() -> BColor {
    if chance(0.3) {
        BColor::YELLOW
    } else {
        BColor::RED
    }
}

impl Particle {
    fn new(pos: Vec2, _angle: f32, vscale: f32) -> Self {
        let da = rand(-0.04, 0.04);
        let v = rand(130.0, 400.0) * da.cos() * vscale;
        let vel = rotate(Vec2::X * v, da);
        let t = rand(0.3, 0.5);

        Self {
            pos,
            vel,
            remaining_time: t,
            color: random_color(),
        }
    }

    fn step(&mut self) {
        self.pos += self.vel * DT;
        self.remaining_time -= DT;
        self.vel *= 0.99;
    }

    fn color(&self) -> BColor {
        // let a = self.alpha();
        // self.color.mix(&GRAY_800, 1.0 - a)
        self.color
    }
}

const DT: f32 = 0.002;

const WIDTH: u32 = 300;
const HEIGHT: u32 = 50;

pub struct ParticleSim {
    emitter_top: Vec2,
    emitter_bottom: Vec2,
    particles: Vec<Particle>,
    velocity_scale: f32,
    emit: bool,
    switch_dur: f32,
    width: u32,
}

impl ParticleSim {
    fn new(width: u32) -> Self {
        Self {
            emitter_top: Vec2::new(WIDTH as f32 / 2.0, HEIGHT as f32 / 2.0),
            emitter_bottom: Vec2::new(WIDTH as f32 / 2.0, HEIGHT as f32 / 2.0),
            particles: Vec::new(),
            velocity_scale: rand(0.1, 2.0),
            emit: true,
            switch_dur: 3.0,
            width,
        }
    }

    fn step(&mut self, is_emitting: bool) {
        self.velocity_scale = 4.5;

        self.switch_dur -= DT;

        self.particles.iter_mut().for_each(|p| p.step());

        if is_emitting {
            for _ in 0..50 {
                let s = rand(0.0, 1.0);
                let e = self.emitter_bottom.lerp(self.emitter_top, s);
                let e = e + randvec(1.0, 2.0);
                let p = Particle::new(e, 0.0, self.velocity_scale);
                self.particles.push(p);
            }
        }

        self.particles.retain(|p| p.remaining_time > 0.0);
    }
}

fn render_particles(sim: &ParticleSim) -> Result<DynamicImage, Box<dyn std::error::Error>> {
    if sim.particles.is_empty() {
        return Ok(DynamicImage::new(WIDTH, HEIGHT, ColorType::Rgba8));
    }

    let mut img = DynamicImage::new(WIDTH, HEIGHT, ColorType::Rgba8);

    let editable = img.as_mut_rgba8().ok_or("Expected a buffer")?;

    for particle in &sim.particles {
        let p = particle.pos.as_ivec2() - IVec2::X * 100;

        if p.x < 0 || p.y < 0 {
            continue;
        }

        let p = p.as_uvec2();

        let Some(pixel) = editable.get_pixel_mut_checked(p.x, p.y) else {
            continue;
        };

        pixel.0 = particle.color.to_u8();
    }

    Ok(img)
}

/// Run the test client app
#[derive(Parser, Debug, Default, Clone)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long, short, default_value = "10")]
    width: u32,

    #[arg(long, short)]
    outdir: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if !std::fs::exists(&args.outdir)? {
        std::fs::create_dir(&args.outdir)?;
    }

    let mut sim = ParticleSim::new(args.width);

    let mut images = Vec::new();

    for i in 0..200 {
        for _ in 0..3 {
            sim.step(i < 110);
        }

        let img = render_particles(&sim)?;
        images.push(img);
    }

    if images.is_empty() {
        println!("Generated no frames.");
        return Ok(());
    }

    let mut concat: RgbaImage = ImageBuffer::new(WIDTH, HEIGHT * images.len() as u32);

    for (idx, image) in images.into_iter().enumerate() {
        let y = HEIGHT * idx as u32;
        concat.copy_from(&image, 0, y)?;
    }

    let path = Path::new(&args.outdir).join("concat.png");
    println!("Writing to {}", path.display());
    concat.save(path)?;

    Ok(())
}
