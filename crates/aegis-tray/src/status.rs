//! Tray status types.

/// Protection status displayed in the system tray.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TrayStatus {
    /// Protection is active and filtering prompts.
    #[default]
    Protected,

    /// Protection is temporarily paused.
    Paused,

    /// Service error or unavailable.
    Error,
}

impl TrayStatus {
    /// Returns the status as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Protected => "Protected",
            Self::Paused => "Paused",
            Self::Error => "Error",
        }
    }

    /// Returns the tooltip text for this status.
    pub fn tooltip(&self) -> &'static str {
        match self {
            Self::Protected => "Aegis - Protection Active",
            Self::Paused => "Aegis - Protection Paused",
            Self::Error => "Aegis - Service Error",
        }
    }

    /// Returns whether protection is currently active.
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Protected)
    }

    /// Returns the icon variant name for this status.
    pub fn icon_variant(&self) -> &'static str {
        match self {
            Self::Protected => "protected",
            Self::Paused => "paused",
            Self::Error => "error",
        }
    }
}

impl std::fmt::Display for TrayStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_status_is_protected() {
        assert_eq!(TrayStatus::default(), TrayStatus::Protected);
    }

    #[test]
    fn is_active_returns_correct_value() {
        assert!(TrayStatus::Protected.is_active());
        assert!(!TrayStatus::Paused.is_active());
        assert!(!TrayStatus::Error.is_active());
    }

    #[test]
    fn icon_variant_names() {
        assert_eq!(TrayStatus::Protected.icon_variant(), "protected");
        assert_eq!(TrayStatus::Paused.icon_variant(), "paused");
        assert_eq!(TrayStatus::Error.icon_variant(), "error");
    }

    #[test]
    fn display_impl() {
        assert_eq!(format!("{}", TrayStatus::Protected), "Protected");
        assert_eq!(format!("{}", TrayStatus::Paused), "Paused");
        assert_eq!(format!("{}", TrayStatus::Error), "Error");
    }
}
