//! Aegis UI Theme - Soft Teal/Mint color palette.
//!
//! A calming, family-friendly color scheme that conveys trust and protection.

use eframe::egui::Color32;

/// Primary brand colors - Soft Teal/Mint palette.
pub mod brand {
    use super::Color32;

    /// Light mint - for subtle highlights and backgrounds.
    pub const LIGHT: Color32 = Color32::from_rgb(0x5e, 0xea, 0xd4); // #5eead4

    /// Primary teal - main accent color.
    pub const PRIMARY: Color32 = Color32::from_rgb(0x14, 0xb8, 0xa6); // #14b8a6

    /// Darker teal - for hover states and emphasis.
    pub const DARK: Color32 = Color32::from_rgb(0x0d, 0x94, 0x88); // #0d9488

    /// Deep teal - for text on light backgrounds.
    pub const DEEP: Color32 = Color32::from_rgb(0x0f, 0x76, 0x6e); // #0f766e

    /// Forest teal - darkest shade for high contrast.
    pub const FOREST: Color32 = Color32::from_rgb(0x11, 0x5e, 0x59); // #115e59
}

/// Semantic status colors.
pub mod status {
    use super::Color32;

    /// Success/Active - friendly green.
    pub const SUCCESS: Color32 = Color32::from_rgb(0x22, 0xc5, 0x5e); // #22c55e

    /// Warning/Paused - warm amber.
    pub const WARNING: Color32 = Color32::from_rgb(0xf5, 0x9e, 0x0b); // #f59e0b

    /// Error/Disabled - soft red.
    pub const ERROR: Color32 = Color32::from_rgb(0xef, 0x44, 0x44); // #ef4444

    /// Info - uses primary teal.
    pub const INFO: Color32 = super::brand::PRIMARY;
}

/// Dashboard card/stat colors.
pub mod cards {
    use super::Color32;

    /// Total count - primary teal.
    pub const TOTAL: Color32 = super::brand::PRIMARY;

    /// Blocked count - soft red.
    pub const BLOCKED: Color32 = Color32::from_rgb(0xef, 0x44, 0x44); // #ef4444

    /// Warning count - warm amber.
    pub const WARNING: Color32 = Color32::from_rgb(0xf5, 0x9e, 0x0b); // #f59e0b

    /// Allowed count - friendly green.
    pub const ALLOWED: Color32 = Color32::from_rgb(0x22, 0xc5, 0x5e); // #22c55e
}

/// UI element colors.
pub mod ui {
    use super::Color32;

    /// Primary button background.
    pub const BUTTON_PRIMARY: Color32 = super::brand::PRIMARY;

    /// Primary button hover.
    pub const BUTTON_PRIMARY_HOVER: Color32 = super::brand::DARK;

    /// Link color.
    pub const LINK: Color32 = super::brand::PRIMARY;

    /// Accent text color.
    pub const ACCENT_TEXT: Color32 = super::brand::DEEP;
}

/// Progress indicator colors.
pub mod progress {
    use super::Color32;

    /// Current step.
    pub const CURRENT: Color32 = super::brand::PRIMARY;

    /// Completed step.
    pub const DONE: Color32 = super::status::SUCCESS;

    /// Pending step.
    pub const PENDING: Color32 = Color32::GRAY;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brand_colors_are_distinct() {
        assert_ne!(brand::LIGHT, brand::PRIMARY);
        assert_ne!(brand::PRIMARY, brand::DARK);
        assert_ne!(brand::DARK, brand::DEEP);
    }

    #[test]
    fn status_colors_are_distinct() {
        assert_ne!(status::SUCCESS, status::WARNING);
        assert_ne!(status::WARNING, status::ERROR);
    }

    #[test]
    fn card_colors_are_set() {
        // Just ensure they compile and are valid colors
        let _ = cards::TOTAL;
        let _ = cards::BLOCKED;
        let _ = cards::WARNING;
        let _ = cards::ALLOWED;
    }
}
