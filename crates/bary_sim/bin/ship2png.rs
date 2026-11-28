use bary_core::prelude::*;
use bary_parts::{
    Blueprint, PartClassification, PartDatabase, PartPrototype, load_blueprint_file,
    load_parts_from_dir,
};
use clap::Parser;
use image::{DynamicImage, Rgba, RgbaImage};
use std::path::*;

/// Converts ship file to PNG
#[derive(Parser, Debug, Default, Clone)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Ship file (.vehicle) location
    #[arg(long, short('s'))]
    pub ship_path: PathBuf,

    /// Folder containing part definitions
    #[arg(long, short)]
    pub parts_dir: PathBuf,

    /// Destination filepath for PNG
    #[arg(long, short)]
    pub out: PathBuf,

    /// Whether to draw as schematic or "realistic"
    #[arg(long, short('c'))]
    pub schematic: bool,

    /// Multiplier to scale down by
    #[arg(long, short('x'), default_value = "1.0")]
    pub scale_factor: f32,
}

pub fn read_image(path: &Path) -> Option<RgbaImage> {
    Some(image::open(path).ok()?.to_rgba8())
}

pub fn diagram_color(part: &PartPrototype) -> BColor {
    let cl = part.classification();
    match cl {
        PartClassification::Cargo => BColor::GREEN,
        PartClassification::Machine => BColor::RED,
        PartClassification::Thruster => BColor::ORANGE,
        PartClassification::Auxiliary => BColor::TEAL,
        PartClassification::DockingPort => BColor::PURPLE,
        PartClassification::Other => BColor::GRAY,
        PartClassification::Computer => BColor::PINK,
        PartClassification::Structure => BColor::gray(20),
        PartClassification::Decoration => BColor::WHITE,
    }
}

fn draw_pixel(
    image: &mut image::ImageBuffer<Rgba<u8>, Vec<u8>>,
    coord: PartCoord,
    lower: PartCoord,
    color: BColor,
) {
    let delta = coord - lower;
    let mut p = delta.inner();

    p.y = (image.height() - p.y as u32) as i32;

    if let Some(pixel) = image.get_pixel_mut_checked(p.x as u32, p.y as u32) {
        pixel.0[0] = color.r;
        pixel.0[1] = color.g;
        pixel.0[2] = color.b;
        pixel.0[3] = 255;
    }
}

pub fn generate_image(vehicle: &Blueprint, parts: &PartDatabase) -> Option<DynamicImage> {
    let (pixel_min, pixel_max) = vehicle.bounds();
    let dims = (pixel_max - pixel_min).inner().as_uvec2();
    let mut output = DynamicImage::new_rgba8(dims.x, dims.y);
    let to_export = output.as_mut_rgba8().unwrap();

    for layer in [
        PartLayer::Internal,
        PartLayer::Structural,
        PartLayer::Exterior,
    ] {
        for (_, instance) in vehicle.parts() {
            if instance.layer() != layer {
                continue;
            }

            let color = if let Some(proto) = parts.get(&instance.name) {
                diagram_color(proto)
            } else {
                BColor::GRAY
            };

            for coord in instance.region.cells() {
                draw_pixel(to_export, coord, pixel_min, color);
            }
        }

        // for pipe in vehicle.pipes() {
        //     draw_pixel(to_export, pipe.1.start, pixel_min, BColor::YELLOW);
        //     draw_pixel(to_export, pipe.1.end, pixel_min, BColor::YELLOW);
        // }
    }

    Some(output)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    dbg!(&args);

    let parts = load_parts_from_dir(&args.parts_dir)?;

    let vehicle = load_blueprint_file(&args.ship_path, &parts)?;

    println!(
        "Loaded blueprint with {} parts and {} pipes",
        vehicle.part_count(),
        vehicle.pipe_count()
    );

    let mut img = generate_image(&vehicle, &parts).ok_or("Empty vehicle")?;

    if args.scale_factor < 1.0 {
        let filter = image::imageops::FilterType::Nearest;
        img = img.resize(
            (img.width() as f32 * args.scale_factor).round() as u32,
            (img.height() as f32 * args.scale_factor).round() as u32,
            filter,
        );
    } else if args.scale_factor > 1.0 {
        img = img.resize(
            (img.width() as f32 * args.scale_factor).round() as u32,
            (img.height() as f32 * args.scale_factor).round() as u32,
            image::imageops::FilterType::Nearest,
        );
    }

    img.save(&args.out)?;

    Ok(())
}
