use bary_core::prelude::*;
use clap::{Parser, ValueEnum};
use image::*;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(ValueEnum, Debug, Default, Clone, Copy)]
enum Direction {
    #[default]
    Out,
    X,
    Y,
}

/// Converts a part skin to animation spritesheet
#[derive(Parser, Debug, Default, Clone)]
#[command(version, about, long_about = None)]
struct Args {
    /// Part name
    #[arg(long, short)]
    input: PathBuf,

    /// Output spritesheet
    #[arg(long, short)]
    output: PathBuf,

    /// Starting x coordinate
    #[arg(long, short, default_value = "0")]
    x: u32,

    /// Starting y coordinate
    #[arg(long, short, default_value = "0")]
    y: u32,

    /// Building direction
    #[arg(long, short, default_value = "out")]
    dir: Direction,

    /// Pixels built per frame
    #[arg(long, short, default_value = "10")]
    n: usize,
}

fn sort_by_cost(points: &mut Vec<(IVec2, f32)>) {
    points.sort_by(|(_, k), (_, l)| l.total_cmp(k));
}

fn visit_pixels(
    visited: &mut HashSet<(u32, u32)>,
    open_set: &mut Vec<(IVec2, f32)>,
    src: &ImageBuffer<Rgba<u8>, Vec<u8>>,
    img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    dir: Direction,
) {
    while let Some((p, c)) = open_set.pop() {
        let (x, y) = (p.x, p.y);
        let key = (x as u32, y as u32);

        if visited.contains(&key) {
            continue;
        }

        visited.insert(key);

        let p = match img.get_pixel_mut_checked(x as u32, y as u32) {
            Some(p) => p,
            None => continue,
        };

        let q = match src.get_pixel_checked(x as u32, y as u32) {
            Some(q) => q,
            None => continue,
        };

        if q.0[3] > 0 {
            p.0 = q.0;
            for u in -1i32..=1 {
                for v in -1i32..=1 {
                    let mut new_c = c + rand(0.0, 1.0);

                    match dir {
                        Direction::X => {
                            new_c += u.abs() as f32;
                        }
                        Direction::Y => {
                            new_c += v.abs() as f32;
                        }
                        Direction::Out => {
                            new_c += u.abs() as f32;
                            new_c += v.abs() as f32;
                        }
                    }

                    let u = x + u;
                    let v = y + v;
                    if u < 0 || v < 0 {
                        continue;
                    }
                    if !visited.contains(&(u as u32, v as u32)) {
                        open_set.push((IVec2::new(u, v), new_c));
                    }
                    sort_by_cost(open_set);
                }
            }
            break;
        }
    }
}

pub fn read_image(path: &Path) -> Option<RgbaImage> {
    Some(image::open(path).ok()?.to_rgba8())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    dbg!(&args);

    let img = read_image(&args.input).ok_or("Bad image")?;

    let mut out = img.clone();
    for p in out.pixels_mut() {
        if p.0[3] > 0 {
            p.0 = [0, 0, 0, 0];
        }
    }

    let origin = UVec2::new(args.x, args.y).as_ivec2();
    let mut visited = HashSet::new();
    let mut open_set = vec![(origin, 0.0)];

    let mut all_images = Vec::new();

    while !open_set.is_empty() {
        for _ in 0..args.n {
            visit_pixels(&mut visited, &mut open_set, &img, &mut out, args.dir);
            if open_set.is_empty() {
                break;
            }
        }
        all_images.push(out.clone());
    }

    let n = all_images.len();

    println!("{n} sprites");

    let width = n as u32 * out.width();

    let mut concat = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(width, out.height());

    for (i, img) in all_images.iter().enumerate() {
        let x = out.width() * i as u32;
        concat.copy_from(img, x, 0)?;
    }

    concat.save(args.output)?;

    Ok(())
}
