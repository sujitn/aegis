//! Tray menu types and building.

use crate::status::TrayStatus;
use muda::{accelerator::Accelerator, Menu, MenuId, MenuItem, PredefinedMenuItem};

/// Menu action triggered by user interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    /// Open the parent dashboard.
    Dashboard,

    /// Open settings.
    Settings,

    /// Open activity logs.
    Logs,

    /// Pause protection temporarily.
    Pause,

    /// Resume protection.
    Resume,

    /// Quit the application.
    Quit,
}

impl MenuAction {
    /// Returns the menu ID string for this action.
    pub fn id(&self) -> &'static str {
        match self {
            Self::Dashboard => "dashboard",
            Self::Settings => "settings",
            Self::Logs => "logs",
            Self::Pause => "pause",
            Self::Resume => "resume",
            Self::Quit => "quit",
        }
    }

    /// Returns the menu item label for this action.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Dashboard => "Dashboard",
            Self::Settings => "Settings",
            Self::Logs => "Activity Logs",
            Self::Pause => "Pause Protection",
            Self::Resume => "Resume Protection",
            Self::Quit => "Quit Aegis",
        }
    }

    /// Creates a MenuAction from an ID string.
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "dashboard" => Some(Self::Dashboard),
            "settings" => Some(Self::Settings),
            "logs" => Some(Self::Logs),
            "pause" => Some(Self::Pause),
            "resume" => Some(Self::Resume),
            "quit" => Some(Self::Quit),
            _ => None,
        }
    }
}

/// Builder for the tray menu.
pub struct TrayMenu {
    menu: Menu,
    status: TrayStatus,
    pause_item: MenuItem,
    resume_item: MenuItem,
}

impl TrayMenu {
    /// Creates a new tray menu with the given status.
    pub fn new(status: TrayStatus) -> crate::Result<Self> {
        let menu = Menu::new();

        // Status header (disabled, just for display)
        let status_text = format!("Status: {}", status.as_str());
        let status_item = MenuItem::with_id(
            MenuId::new("status"),
            &status_text,
            false, // disabled
            None::<Accelerator>,
        );

        // Main menu items
        let dashboard_item = MenuItem::with_id(
            MenuId::new(MenuAction::Dashboard.id()),
            MenuAction::Dashboard.label(),
            true,
            None::<Accelerator>,
        );

        let settings_item = MenuItem::with_id(
            MenuId::new(MenuAction::Settings.id()),
            MenuAction::Settings.label(),
            true,
            None::<Accelerator>,
        );

        let logs_item = MenuItem::with_id(
            MenuId::new(MenuAction::Logs.id()),
            MenuAction::Logs.label(),
            true,
            None::<Accelerator>,
        );

        // Pause/Resume based on status
        let pause_item = MenuItem::with_id(
            MenuId::new(MenuAction::Pause.id()),
            MenuAction::Pause.label(),
            status == TrayStatus::Protected,
            None::<Accelerator>,
        );

        let resume_item = MenuItem::with_id(
            MenuId::new(MenuAction::Resume.id()),
            MenuAction::Resume.label(),
            status == TrayStatus::Paused,
            None::<Accelerator>,
        );

        let quit_item = MenuItem::with_id(
            MenuId::new(MenuAction::Quit.id()),
            MenuAction::Quit.label(),
            true,
            None::<Accelerator>,
        );

        // Build menu
        menu.append(&status_item)
            .map_err(|e| crate::TrayError::MenuCreation(e.to_string()))?;
        menu.append(&PredefinedMenuItem::separator())
            .map_err(|e| crate::TrayError::MenuCreation(e.to_string()))?;
        menu.append(&dashboard_item)
            .map_err(|e| crate::TrayError::MenuCreation(e.to_string()))?;
        menu.append(&settings_item)
            .map_err(|e| crate::TrayError::MenuCreation(e.to_string()))?;
        menu.append(&logs_item)
            .map_err(|e| crate::TrayError::MenuCreation(e.to_string()))?;
        menu.append(&PredefinedMenuItem::separator())
            .map_err(|e| crate::TrayError::MenuCreation(e.to_string()))?;
        menu.append(&pause_item)
            .map_err(|e| crate::TrayError::MenuCreation(e.to_string()))?;
        menu.append(&resume_item)
            .map_err(|e| crate::TrayError::MenuCreation(e.to_string()))?;
        menu.append(&PredefinedMenuItem::separator())
            .map_err(|e| crate::TrayError::MenuCreation(e.to_string()))?;
        menu.append(&quit_item)
            .map_err(|e| crate::TrayError::MenuCreation(e.to_string()))?;

        Ok(Self {
            menu,
            status,
            pause_item,
            resume_item,
        })
    }

    /// Returns the underlying menu.
    pub fn menu(&self) -> &Menu {
        &self.menu
    }

    /// Updates the menu for the given status.
    pub fn update_status(&mut self, status: TrayStatus) {
        self.status = status;

        // Update pause/resume visibility based on status
        self.pause_item.set_enabled(status == TrayStatus::Protected);
        self.resume_item.set_enabled(status == TrayStatus::Paused);
    }

    /// Returns the current status.
    pub fn status(&self) -> TrayStatus {
        self.status
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_action_roundtrip() {
        let actions = [
            MenuAction::Dashboard,
            MenuAction::Settings,
            MenuAction::Logs,
            MenuAction::Pause,
            MenuAction::Resume,
            MenuAction::Quit,
        ];

        for action in actions {
            let id = action.id();
            let parsed = MenuAction::from_id(id);
            assert_eq!(parsed, Some(action));
        }
    }

    #[test]
    fn menu_action_labels_not_empty() {
        let actions = [
            MenuAction::Dashboard,
            MenuAction::Settings,
            MenuAction::Logs,
            MenuAction::Pause,
            MenuAction::Resume,
            MenuAction::Quit,
        ];

        for action in actions {
            assert!(!action.label().is_empty());
        }
    }
}
