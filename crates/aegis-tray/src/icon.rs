//! Tray icon management with embedded PNG icons.

use crate::{status::TrayStatus, TrayError};
use image::GenericImageView;
use tray_icon::Icon;

/// Embedded PNG icon data for each status.
/// These icons are generated from the Aegis shield logo with status indicator dots.
mod embedded {
    // Protected status icons (green dot)
    pub const PROTECTED_32: &[u8] =
        include_bytes!("../../aegis-app/assets/icons/tray/tray-protected-32.png");

    // Paused status icons (yellow dot)
    pub const PAUSED_32: &[u8] =
        include_bytes!("../../aegis-app/assets/icons/tray/tray-paused-32.png");

    // Error status icons (red dot)
    pub const ERROR_32: &[u8] =
        include_bytes!("../../aegis-app/assets/icons/tray/tray-error-32.png");
}

/// Manages tray icon loading and generation.
pub struct TrayIcon;

impl TrayIcon {
    /// Loads or generates an icon for the given status.
    pub fn for_status(status: TrayStatus) -> crate::Result<Icon> {
        Self::load_embedded_icon(status)
    }

    /// Loads an embedded PNG icon for the given status.
    fn load_embedded_icon(status: TrayStatus) -> crate::Result<Icon> {
        let png_data = match status {
            TrayStatus::Protected => embedded::PROTECTED_32,
            TrayStatus::Paused => embedded::PAUSED_32,
            TrayStatus::Error => embedded::ERROR_32,
        };

        // Decode the PNG
        let img = image::load_from_memory(png_data)
            .map_err(|e| TrayError::IconCreation(format!("Failed to decode PNG: {}", e)))?;

        let (width, height) = img.dimensions();

        // Convert to RGBA
        let rgba = img.to_rgba8();

        Icon::from_rgba(rgba.into_raw(), width, height)
            .map_err(|e| TrayError::IconCreation(e.to_string()))
    }
}

/// Fallback: generates a simple colored shield icon in RGBA format.
/// Used when embedded icons are not available.
#[allow(dead_code)]
fn generate_shield_icon(size: u32, r: u8, g: u8, b: u8) -> Vec<u8> {
    let mut rgba = vec![0u8; (size * size * 4) as usize];

    let center_x = size as f32 / 2.0;
    let center_y = size as f32 / 2.0;

    for y in 0..size {
        for x in 0..size {
            let idx = ((y * size + x) * 4) as usize;

            // Normalized coordinates (-1 to 1)
            let nx = (x as f32 - center_x) / center_x;
            let ny = (y as f32 - center_y) / center_y;

            // Shield shape: top is rounded, bottom comes to a point
            let in_shield = is_in_shield(nx, ny);

            if in_shield {
                rgba[idx] = r;
                rgba[idx + 1] = g;
                rgba[idx + 2] = b;
                rgba[idx + 3] = 255;
            }
        }
    }

    rgba
}

/// Determines if a point is inside the shield shape.
#[allow(dead_code)]
fn is_in_shield(nx: f32, ny: f32) -> bool {
    // Shield dimensions
    let shield_width = 0.7;

    // Top half: rounded rectangle
    if ny < 0.0 {
        let x_bound = shield_width;
        let y_bound = 0.6;

        // Check if within bounds with rounded corners
        if nx.abs() <= x_bound && ny.abs() <= y_bound {
            // Round the top corners
            let corner_radius = 0.2;
            let corner_x = x_bound - corner_radius;
            let corner_y = y_bound - corner_radius;

            if nx.abs() > corner_x && ny.abs() > corner_y {
                let dx = nx.abs() - corner_x;
                let dy = ny.abs() - corner_y;
                return dx * dx + dy * dy <= corner_radius * corner_radius;
            }
            return true;
        }
    } else {
        // Bottom half: triangle pointing down
        let top_width = shield_width;
        let bottom_y = 0.8;

        // Linear interpolation of width from top to bottom
        let progress = ny / bottom_y;
        let width_at_y = top_width * (1.0 - progress);

        if ny <= bottom_y && nx.abs() <= width_at_y {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_icons_exist() {
        assert!(!embedded::PROTECTED_32.is_empty());
        assert!(!embedded::PAUSED_32.is_empty());
        assert!(!embedded::ERROR_32.is_empty());
    }

    #[test]
    fn can_decode_embedded_icons() {
        for png_data in [
            embedded::PROTECTED_32,
            embedded::PAUSED_32,
            embedded::ERROR_32,
        ] {
            let img = image::load_from_memory(png_data);
            assert!(img.is_ok(), "Failed to decode embedded PNG");
        }
    }

    #[test]
    fn generate_shield_icon_correct_size() {
        let rgba = generate_shield_icon(32, 255, 0, 0);
        assert_eq!(rgba.len(), 32 * 32 * 4);
    }

    #[test]
    fn generate_shield_icon_has_content() {
        let rgba = generate_shield_icon(32, 255, 0, 0);

        // Should have some non-transparent pixels
        let has_content = rgba.chunks(4).any(|pixel| pixel[3] > 0);
        assert!(has_content);
    }

    #[test]
    fn shield_center_is_inside() {
        // Center of the shield should be inside
        assert!(is_in_shield(0.0, 0.0));
        assert!(is_in_shield(0.0, -0.3));
        assert!(is_in_shield(0.0, 0.3));
    }

    #[test]
    fn shield_corners_are_outside() {
        // Far corners should be outside
        assert!(!is_in_shield(1.0, 1.0));
        assert!(!is_in_shield(-1.0, -1.0));
        assert!(!is_in_shield(1.0, -1.0));
        assert!(!is_in_shield(-1.0, 1.0));
    }
}
