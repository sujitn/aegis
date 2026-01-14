//! Tray icon management.

use crate::{status::TrayStatus, TrayError};
use tray_icon::Icon;

/// Icon dimensions for the tray.
const ICON_SIZE: u32 = 32;

/// Manages tray icon loading and generation.
pub struct TrayIcon;

impl TrayIcon {
    /// Loads or generates an icon for the given status.
    pub fn for_status(status: TrayStatus) -> crate::Result<Icon> {
        Self::generate_icon(status)
    }

    /// Generates a simple colored icon for the status.
    /// This creates a basic icon without external files.
    fn generate_icon(status: TrayStatus) -> crate::Result<Icon> {
        let (r, g, b) = match status {
            TrayStatus::Protected => (0x1a, 0x73, 0xe8), // Blue - #1a73e8
            TrayStatus::Paused => (0xfb, 0xbc, 0x04),    // Yellow - #fbbc04
            TrayStatus::Error => (0xea, 0x43, 0x35),     // Red - #ea4335
        };

        let rgba = generate_shield_icon(ICON_SIZE, r, g, b);

        Icon::from_rgba(rgba, ICON_SIZE, ICON_SIZE)
            .map_err(|e| TrayError::IconCreation(e.to_string()))
    }
}

/// Generates a simple shield-shaped icon in RGBA format.
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
    fn icon_size_is_valid() {
        assert!(ICON_SIZE > 0);
        assert!(ICON_SIZE <= 256);
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
