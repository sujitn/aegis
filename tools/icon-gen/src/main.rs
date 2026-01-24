//! Icon generation tool for Aegis.
//!
//! Converts the Aegis logo SVG to various icon formats and sizes.

use anyhow::{Context, Result};
use clap::Parser;
use ico::{IconDir, IconDirEntry, IconImage, ResourceType};
use icns::{IconFamily, IconType, Image as IcnsImage, PixelFormat};
use png::{BitDepth, ColorType, Encoder};
use resvg::usvg::{Options, Tree};
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use tiny_skia::{Pixmap, Transform};

/// Icon sizes to generate for various platforms.
const PNG_SIZES: &[u32] = &[16, 22, 24, 32, 48, 64, 128, 256, 512, 1024];

/// Windows ICO sizes (must be power of 2, max 256).
const ICO_SIZES: &[u32] = &[16, 24, 32, 48, 64, 128, 256];

/// Tray icon sizes (smaller for system tray).
const TRAY_SIZES: &[u32] = &[16, 22, 24, 32, 48];

#[derive(Parser, Debug)]
#[command(name = "icon-gen")]
#[command(about = "Generate Aegis application icons from SVG")]
struct Args {
    /// Input SVG file path.
    #[arg(
        short,
        long,
        default_value = "crates/aegis-app/assets/icons/aegis-logo.svg"
    )]
    input: PathBuf,

    /// Output directory for generated icons.
    #[arg(short, long, default_value = "crates/aegis-app/assets/icons")]
    output: PathBuf,

    /// Also generate tray icons with status colors.
    #[arg(long, default_value = "true")]
    tray_icons: bool,

    /// Verbose output.
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Ensure output directory exists
    fs::create_dir_all(&args.output).context("Failed to create output directory")?;

    // Load and parse the SVG
    println!("Loading SVG: {}", args.input.display());
    let svg_data = fs::read_to_string(&args.input).context("Failed to read SVG file")?;

    let opts = Options::default();
    let tree = Tree::from_str(&svg_data, &opts).context("Failed to parse SVG")?;

    // Generate PNG icons at various sizes
    println!("\nGenerating PNG icons...");
    for &size in PNG_SIZES {
        let output_path = args.output.join(format!("icon-{}.png", size));
        render_svg_to_png(&tree, size, &output_path, args.verbose)?;
    }

    // Generate Windows ICO file
    println!("\nGenerating Windows ICO file...");
    let ico_path = args.output.join("icon.ico");
    generate_ico(&tree, &ico_path, args.verbose)?;

    // Generate macOS ICNS file
    println!("\nGenerating macOS ICNS file...");
    let icns_path = args.output.join("icon.icns");
    generate_icns(&tree, &icns_path, args.verbose)?;

    // Generate tray icons if requested
    if args.tray_icons {
        println!("\nGenerating tray icons...");
        let tray_dir = args.output.join("tray");
        fs::create_dir_all(&tray_dir)?;

        // Generate base tray icons (full color)
        for &size in TRAY_SIZES {
            let output_path = tray_dir.join(format!("tray-{}.png", size));
            render_svg_to_png(&tree, size, &output_path, args.verbose)?;
        }

        // Generate tray icons with status indicator overlays
        generate_tray_status_icons(&tree, &tray_dir, args.verbose)?;
    }

    // Generate a high-res PNG for macOS icns generation
    let hires_path = args.output.join("icon-1024.png");
    if !hires_path.exists() {
        render_svg_to_png(&tree, 1024, &hires_path, args.verbose)?;
    }

    println!("\nIcon generation complete!");
    println!("Output directory: {}", args.output.display());
    println!("\nGenerated files:");
    println!("  - icon.ico   (Windows)");
    println!("  - icon.icns  (macOS)");
    println!("  - icon-*.png (Linux/all platforms)");

    Ok(())
}

/// Renders an SVG tree to a PNG file at the specified size.
fn render_svg_to_png(tree: &Tree, size: u32, output: &Path, verbose: bool) -> Result<()> {
    let pixmap = render_svg(tree, size)?;

    // Save as PNG
    save_png(&pixmap, output)?;

    if verbose {
        println!("  Created: {} ({}x{})", output.display(), size, size);
    }

    Ok(())
}

/// Renders an SVG tree to a pixmap at the specified size.
fn render_svg(tree: &Tree, size: u32) -> Result<Pixmap> {
    let svg_size = tree.size();
    let scale_x = size as f32 / svg_size.width();
    let scale_y = size as f32 / svg_size.height();
    let scale = scale_x.min(scale_y);

    let mut pixmap = Pixmap::new(size, size).context("Failed to create pixmap")?;

    // Center the icon if aspect ratio doesn't match
    let offset_x = (size as f32 - svg_size.width() * scale) / 2.0;
    let offset_y = (size as f32 - svg_size.height() * scale) / 2.0;

    let transform = Transform::from_scale(scale, scale).post_translate(offset_x, offset_y);

    resvg::render(tree, transform, &mut pixmap.as_mut());

    Ok(pixmap)
}

/// Saves a pixmap as a PNG file.
fn save_png(pixmap: &Pixmap, path: &Path) -> Result<()> {
    let file = File::create(path).context("Failed to create PNG file")?;
    let writer = BufWriter::new(file);

    let mut encoder = Encoder::new(writer, pixmap.width(), pixmap.height());
    encoder.set_color(ColorType::Rgba);
    encoder.set_depth(BitDepth::Eight);

    let mut writer = encoder
        .write_header()
        .context("Failed to write PNG header")?;

    // Convert from premultiplied alpha to straight alpha
    let data = pixmap.data();
    let mut rgba = Vec::with_capacity(data.len());

    for chunk in data.chunks(4) {
        let r = chunk[0];
        let g = chunk[1];
        let b = chunk[2];
        let a = chunk[3];

        if a == 0 {
            rgba.extend_from_slice(&[0, 0, 0, 0]);
        } else {
            // Un-premultiply
            let alpha = a as f32 / 255.0;
            rgba.push((r as f32 / alpha).min(255.0) as u8);
            rgba.push((g as f32 / alpha).min(255.0) as u8);
            rgba.push((b as f32 / alpha).min(255.0) as u8);
            rgba.push(a);
        }
    }

    writer
        .write_image_data(&rgba)
        .context("Failed to write PNG data")?;

    Ok(())
}

/// Generates a Windows ICO file containing multiple sizes.
fn generate_ico(tree: &Tree, output: &Path, verbose: bool) -> Result<()> {
    let mut icon_dir = IconDir::new(ResourceType::Icon);

    for &size in ICO_SIZES {
        let pixmap = render_svg(tree, size)?;

        // Convert to RGBA (un-premultiply)
        let data = pixmap.data();
        let mut rgba = Vec::with_capacity(data.len());

        for chunk in data.chunks(4) {
            let r = chunk[0];
            let g = chunk[1];
            let b = chunk[2];
            let a = chunk[3];

            if a == 0 {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            } else {
                let alpha = a as f32 / 255.0;
                rgba.push((r as f32 / alpha).min(255.0) as u8);
                rgba.push((g as f32 / alpha).min(255.0) as u8);
                rgba.push((b as f32 / alpha).min(255.0) as u8);
                rgba.push(a);
            }
        }

        let image = IconImage::from_rgba_data(size, size, rgba);
        let entry = IconDirEntry::encode(&image).context("Failed to encode ICO entry")?;
        icon_dir.add_entry(entry);

        if verbose {
            println!("  Added {}x{} to ICO", size, size);
        }
    }

    let file = File::create(output).context("Failed to create ICO file")?;
    icon_dir.write(file).context("Failed to write ICO file")?;

    println!("  Created: {}", output.display());

    Ok(())
}

/// Generates a macOS ICNS file containing multiple sizes.
fn generate_icns(tree: &Tree, output: &Path, verbose: bool) -> Result<()> {
    let mut icon_family = IconFamily::new();

    // macOS icon sizes and their corresponding IconType
    // (size, icon_type for 1x, icon_type for 2x retina)
    let sizes: &[(u32, IconType, Option<IconType>)] = &[
        (16, IconType::RGBA32_16x16, Some(IconType::RGBA32_16x16_2x)),
        (32, IconType::RGBA32_32x32, Some(IconType::RGBA32_32x32_2x)),
        (128, IconType::RGBA32_128x128, Some(IconType::RGBA32_128x128_2x)),
        (256, IconType::RGBA32_256x256, Some(IconType::RGBA32_256x256_2x)),
        (512, IconType::RGBA32_512x512, Some(IconType::RGBA32_512x512_2x)),
    ];

    for &(size, icon_type_1x, icon_type_2x) in sizes {
        let pixmap = render_svg(tree, size)?;

        // Convert to RGBA (un-premultiply)
        let rgba = pixmap_to_rgba(&pixmap);

        // Add 1x version
        let image = IcnsImage::from_data(PixelFormat::RGBA, size, size, rgba.clone())
            .context("Failed to create ICNS image")?;
        icon_family.add_icon_with_type(&image, icon_type_1x).context("Failed to add icon to ICNS")?;

        if verbose {
            println!("  Added {}x{} to ICNS", size, size);
        }

        // Add 2x retina version if applicable
        if let Some(icon_type_2x) = icon_type_2x {
            let retina_size = size * 2;
            let retina_pixmap = render_svg(tree, retina_size)?;
            let retina_rgba = pixmap_to_rgba(&retina_pixmap);

            let retina_image = IcnsImage::from_data(
                PixelFormat::RGBA,
                retina_size,
                retina_size,
                retina_rgba,
            )
            .context("Failed to create ICNS retina image")?;
            icon_family.add_icon_with_type(&retina_image, icon_type_2x).context("Failed to add retina icon to ICNS")?;

            if verbose {
                println!("  Added {}x{} (@2x) to ICNS", retina_size, retina_size);
            }
        }
    }

    let file = File::create(output).context("Failed to create ICNS file")?;
    icon_family.write(file).context("Failed to write ICNS file")?;

    println!("  Created: {}", output.display());

    Ok(())
}

/// Converts a pixmap to RGBA data (un-premultiplied).
fn pixmap_to_rgba(pixmap: &Pixmap) -> Vec<u8> {
    let data = pixmap.data();
    let mut rgba = Vec::with_capacity(data.len());

    for chunk in data.chunks(4) {
        let r = chunk[0];
        let g = chunk[1];
        let b = chunk[2];
        let a = chunk[3];

        if a == 0 {
            rgba.extend_from_slice(&[0, 0, 0, 0]);
        } else {
            let alpha = a as f32 / 255.0;
            rgba.push((r as f32 / alpha).min(255.0) as u8);
            rgba.push((g as f32 / alpha).min(255.0) as u8);
            rgba.push((b as f32 / alpha).min(255.0) as u8);
            rgba.push(a);
        }
    }

    rgba
}

/// Generates tray icons with status indicator overlays.
fn generate_tray_status_icons(tree: &Tree, output_dir: &Path, verbose: bool) -> Result<()> {
    // Status colors: (name, r, g, b)
    let statuses = [
        ("protected", 0x22, 0xc5, 0x5e), // Green
        ("paused", 0xfb, 0xbc, 0x04),    // Yellow/Amber
        ("error", 0xef, 0x44, 0x44),     // Red
    ];

    for &size in TRAY_SIZES {
        let base_pixmap = render_svg(tree, size)?;

        for (status_name, r, g, b) in &statuses {
            // Clone the base pixmap
            let mut pixmap = base_pixmap.clone();

            // Draw a status indicator dot in the bottom-right corner
            draw_status_dot(&mut pixmap, *r, *g, *b);

            let output_path = output_dir.join(format!("tray-{}-{}.png", status_name, size));
            save_png(&pixmap, &output_path)?;

            if verbose {
                println!("  Created: {} ({}x{})", output_path.display(), size, size);
            }
        }
    }

    Ok(())
}

/// Draws a colored status indicator dot in the bottom-right corner of a pixmap.
fn draw_status_dot(pixmap: &mut Pixmap, r: u8, g: u8, b: u8) {
    let size = pixmap.width();
    let dot_radius = (size as f32 * 0.25).max(3.0);
    let dot_center_x = size as f32 - dot_radius - 1.0;
    let dot_center_y = size as f32 - dot_radius - 1.0;

    let pixels = pixmap.pixels_mut();

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - dot_center_x;
            let dy = y as f32 - dot_center_y;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist <= dot_radius {
                let idx = (y * size + x) as usize;

                // Anti-aliased edge
                let alpha = if dist > dot_radius - 1.0 {
                    ((dot_radius - dist) * 255.0).clamp(0.0, 255.0) as u8
                } else {
                    255
                };

                // Premultiply the color values by alpha
                let alpha_f = alpha as f32 / 255.0;

                // White border (outer ring)
                if dist > dot_radius - 1.5 && dist <= dot_radius {
                    let pr = (255.0 * alpha_f) as u8;
                    let pg = (255.0 * alpha_f) as u8;
                    let pb = (255.0 * alpha_f) as u8;
                    if let Some(color) =
                        tiny_skia::PremultipliedColorU8::from_rgba(pr, pg, pb, alpha)
                    {
                        pixels[idx] = color;
                    }
                } else {
                    // Colored center
                    let pr = (r as f32 * alpha_f) as u8;
                    let pg = (g as f32 * alpha_f) as u8;
                    let pb = (b as f32 * alpha_f) as u8;
                    if let Some(color) =
                        tiny_skia::PremultipliedColorU8::from_rgba(pr, pg, pb, alpha)
                    {
                        pixels[idx] = color;
                    }
                }
            }
        }
    }
}
