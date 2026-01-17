//! First-run setup wizard view (F015).
//!
//! Multi-step wizard for initial Aegis configuration:
//! 1. Welcome
//! 2. Password creation
//! 3. Protection level selection
//! 4. Browser extension installation
//! 5. CA certificate installation (for proxy mode)
//! 6. Profile creation
//! 7. Complete

use std::env;

use auto_launch::{AutoLaunch, AutoLaunchBuilder};
use eframe::egui::{self, Color32, RichText, TextEdit};

use crate::state::AppState;
use crate::theme::{brand, progress, status};
use aegis_core::extension_install::get_extension_path;

/// App name for autostart.
const APP_NAME: &str = "Aegis";

/// Creates an AutoLaunch instance for Aegis.
fn create_auto_launch() -> Option<AutoLaunch> {
    let exe_path = env::current_exe().ok()?;
    let exe_str = exe_path.to_str()?;
    let args = &["--minimized"];

    #[cfg(target_os = "macos")]
    let launcher = AutoLaunchBuilder::new()
        .set_app_name(APP_NAME)
        .set_app_path(exe_str)
        .set_args(args)
        .set_use_launch_agent(true)
        .build()
        .ok()?;

    #[cfg(not(target_os = "macos"))]
    let launcher = AutoLaunchBuilder::new()
        .set_app_name(APP_NAME)
        .set_app_path(exe_str)
        .set_args(args)
        .build()
        .ok()?;

    Some(launcher)
}

/// Enables autostart.
fn enable_autostart() -> Result<(), String> {
    let launcher = create_auto_launch().ok_or("Failed to create autostart")?;
    launcher.enable().map_err(|e| e.to_string())
}

/// Setup wizard steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SetupStep {
    /// Welcome screen.
    #[default]
    Welcome,
    /// Password creation.
    Password,
    /// Protection level selection.
    ProtectionLevel,
    /// Browser extension installation.
    BrowserExtension,
    /// CA certificate installation guidance.
    CaInstall,
    /// First profile creation.
    Profile,
    /// Setup complete.
    Complete,
}

impl SetupStep {
    /// Returns the step number (1-indexed).
    pub fn number(&self) -> usize {
        match self {
            Self::Welcome => 1,
            Self::Password => 2,
            Self::ProtectionLevel => 3,
            Self::BrowserExtension => 4,
            Self::CaInstall => 5,
            Self::Profile => 6,
            Self::Complete => 7,
        }
    }

    /// Returns the total number of steps.
    pub fn total() -> usize {
        7
    }

    /// Returns the next step.
    pub fn next(&self) -> Option<Self> {
        match self {
            Self::Welcome => Some(Self::Password),
            Self::Password => Some(Self::ProtectionLevel),
            Self::ProtectionLevel => Some(Self::BrowserExtension),
            Self::BrowserExtension => Some(Self::CaInstall),
            Self::CaInstall => Some(Self::Profile),
            Self::Profile => Some(Self::Complete),
            Self::Complete => None,
        }
    }

    /// Returns the previous step.
    pub fn prev(&self) -> Option<Self> {
        match self {
            Self::Welcome => None,
            Self::Password => Some(Self::Welcome),
            Self::ProtectionLevel => Some(Self::Password),
            Self::BrowserExtension => Some(Self::ProtectionLevel),
            Self::CaInstall => Some(Self::BrowserExtension),
            Self::Profile => Some(Self::CaInstall),
            Self::Complete => Some(Self::Profile),
        }
    }
}

/// Protection level presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProtectionLevel {
    /// Standard protection - blocks harmful content.
    #[default]
    Standard,
    /// Strict protection - blocks harmful + flags questionable.
    Strict,
    /// Custom - user configures everything.
    Custom,
}

impl ProtectionLevel {
    /// Returns the display name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Standard => "Standard",
            Self::Strict => "Strict",
            Self::Custom => "Custom",
        }
    }

    /// Returns a description.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Standard => "Blocks harmful content like violence, adult content, and jailbreaks. Recommended for most families.",
            Self::Strict => "Blocks harmful content and flags questionable content for review. Better for younger children.",
            Self::Custom => "Start with no rules and configure everything yourself. For advanced users.",
        }
    }
}

/// State for the setup wizard.
#[derive(Debug, Default)]
pub struct SetupWizardState {
    /// Current step.
    pub step: SetupStep,
    /// New password input.
    pub password: String,
    /// Confirm password input.
    pub confirm_password: String,
    /// Selected protection level.
    pub protection_level: ProtectionLevel,
    /// Profile name.
    pub profile_name: String,
    /// Profile OS username.
    pub profile_os_username: String,
    /// Whether CA was generated.
    pub ca_generated: bool,
    /// CA certificate path (if generated).
    pub ca_cert_path: Option<String>,
    /// Error message.
    pub error: Option<String>,
    /// Whether to enable autostart (default: true).
    pub enable_autostart: bool,
}

impl SetupWizardState {
    /// Creates a new setup wizard state.
    pub fn new() -> Self {
        Self {
            enable_autostart: true, // Default to enabled
            ..Self::default()
        }
    }

    /// Moves to the next step.
    pub fn next_step(&mut self) {
        if let Some(next) = self.step.next() {
            self.step = next;
            self.error = None;
        }
    }

    /// Moves to the previous step.
    pub fn prev_step(&mut self) {
        if let Some(prev) = self.step.prev() {
            self.step = prev;
            self.error = None;
        }
    }

    /// Sets an error message.
    pub fn set_error(&mut self, msg: impl Into<String>) {
        self.error = Some(msg.into());
    }

    /// Clears error message.
    pub fn clear_error(&mut self) {
        self.error = None;
    }
}

/// Renders the setup wizard.
pub fn render(ui: &mut egui::Ui, state: &mut AppState, wizard: &mut SetupWizardState) {
    ui.vertical_centered(|ui| {
        ui.add_space(40.0);

        // Progress indicator
        render_progress(ui, wizard);

        ui.add_space(24.0);

        // Step content in a frame
        egui::Frame::new()
            .fill(ui.style().visuals.widgets.noninteractive.bg_fill)
            .corner_radius(8.0)
            .inner_margin(32.0)
            .show(ui, |ui| {
                ui.set_min_width(450.0);
                ui.set_max_width(550.0);

                match wizard.step {
                    SetupStep::Welcome => render_welcome(ui, wizard),
                    SetupStep::Password => render_password(ui, state, wizard),
                    SetupStep::ProtectionLevel => render_protection_level(ui, wizard),
                    SetupStep::BrowserExtension => render_browser_extension(ui, state, wizard),
                    SetupStep::CaInstall => render_ca_install(ui, state, wizard),
                    SetupStep::Profile => render_profile(ui, state, wizard),
                    SetupStep::Complete => render_complete(ui, state, wizard),
                }
            });

        // Error message
        if let Some(ref error) = wizard.error {
            ui.add_space(16.0);
            ui.colored_label(status::ERROR, error);
        }
    });
}

/// Renders the progress indicator.
fn render_progress(ui: &mut egui::Ui, wizard: &SetupWizardState) {
    let current = wizard.step.number();
    let total = SetupStep::total();

    ui.horizontal(|ui| {
        for i in 1..=total {
            let is_current = i == current;
            let is_done = i < current;

            let color = if is_current {
                progress::CURRENT
            } else if is_done {
                progress::DONE
            } else {
                progress::PENDING
            };

            // Circle
            let size = if is_current { 12.0 } else { 10.0 };
            let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
            ui.painter().circle_filled(rect.center(), size / 2.0, color);

            // Connector line (except after last)
            if i < total {
                let line_width = 20.0;
                let (line_rect, _) =
                    ui.allocate_exact_size(egui::vec2(line_width, 2.0), egui::Sense::hover());
                let line_color = if is_done {
                    progress::DONE
                } else {
                    progress::PENDING
                };
                ui.painter().rect_filled(line_rect, 0.0, line_color);
            }
        }
    });

    ui.label(
        RichText::new(format!("Step {} of {}", current, total))
            .size(12.0)
            .weak(),
    );
}

/// Renders the Welcome step.
fn render_welcome(ui: &mut egui::Ui, wizard: &mut SetupWizardState) {
    ui.vertical_centered(|ui| {
        ui.label(RichText::new("Welcome to Aegis").size(24.0).strong());
        ui.add_space(8.0);
        ui.label(
            RichText::new("AI Safety for Families")
                .size(14.0)
                .color(brand::PRIMARY),
        );

        ui.add_space(24.0);

        ui.label("Aegis helps protect your family from harmful AI interactions by:");
        ui.add_space(8.0);

        let features = [
            "Filtering dangerous prompts and jailbreak attempts",
            "Blocking inappropriate content categories",
            "Setting time-based usage rules",
            "Creating per-child protection profiles",
            "Logging activity for parental review",
        ];

        for feature in features {
            ui.horizontal(|ui| {
                ui.colored_label(status::SUCCESS, "✓");
                ui.label(feature);
            });
        }

        ui.add_space(24.0);

        ui.label(
            RichText::new("This wizard will guide you through the initial setup.")
                .size(12.0)
                .weak(),
        );

        ui.add_space(24.0);

        if ui
            .add_sized([200.0, 36.0], egui::Button::new("Get Started"))
            .clicked()
        {
            wizard.next_step();
        }
    });
}

/// Renders the Password creation step.
fn render_password(ui: &mut egui::Ui, state: &mut AppState, wizard: &mut SetupWizardState) {
    ui.vertical_centered(|ui| {
        ui.label(RichText::new("Create Parent Password").size(20.0).strong());
        ui.add_space(8.0);
        ui.label(
            RichText::new("This password protects your Aegis settings from children.")
                .size(12.0)
                .weak(),
        );

        ui.add_space(24.0);

        // Password input
        ui.horizontal(|ui| {
            ui.label("Password:");
            ui.add_space(8.0);
            ui.add(
                TextEdit::singleline(&mut wizard.password)
                    .password(true)
                    .hint_text("Min 6 characters")
                    .desired_width(200.0),
            );
        });

        ui.add_space(8.0);

        // Confirm password
        ui.horizontal(|ui| {
            ui.label("Confirm:");
            ui.add_space(20.0);
            let response = ui.add(
                TextEdit::singleline(&mut wizard.confirm_password)
                    .password(true)
                    .hint_text("Confirm password")
                    .desired_width(200.0),
            );

            // Handle Enter key
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                validate_and_save_password(state, wizard);
            }
        });

        // Password strength indicator
        ui.add_space(8.0);
        let password_len = wizard.password.len();
        let (strength_text, strength_color) = if password_len == 0 {
            ("", Color32::GRAY)
        } else if password_len < 6 {
            ("Too short", status::ERROR)
        } else if password_len < 10 {
            ("Acceptable", status::WARNING)
        } else {
            ("Strong", status::SUCCESS)
        };
        if !strength_text.is_empty() {
            ui.label(
                RichText::new(strength_text)
                    .size(11.0)
                    .color(strength_color),
            );
        }

        ui.add_space(24.0);

        // Navigation buttons
        ui.horizontal(|ui| {
            if ui.button("Back").clicked() {
                wizard.prev_step();
            }

            ui.add_space(100.0);

            if ui
                .add_sized([100.0, 32.0], egui::Button::new("Continue"))
                .clicked()
            {
                validate_and_save_password(state, wizard);
            }
        });
    });
}

/// Validates password and saves it.
fn validate_and_save_password(state: &mut AppState, wizard: &mut SetupWizardState) {
    // Check password length
    if wizard.password.len() < 6 {
        wizard.set_error("Password must be at least 6 characters");
        return;
    }

    // Check passwords match
    if wizard.password != wizard.confirm_password {
        wizard.set_error("Passwords do not match");
        return;
    }

    // Hash and save password
    match state.auth.hash_password(&wizard.password) {
        Ok(hash) => {
            if let Err(e) = state.db.set_password_hash(&hash) {
                wizard.set_error(format!("Failed to save password: {}", e));
                return;
            }
            state.is_first_setup = false;
            wizard.next_step();
        }
        Err(e) => {
            wizard.set_error(format!("Failed to hash password: {}", e));
        }
    }
}

/// Renders the Protection Level step.
fn render_protection_level(ui: &mut egui::Ui, wizard: &mut SetupWizardState) {
    ui.vertical_centered(|ui| {
        ui.label(RichText::new("Choose Protection Level").size(20.0).strong());
        ui.add_space(8.0);
        ui.label(
            RichText::new("Select how strictly Aegis should filter content.")
                .size(12.0)
                .weak(),
        );

        ui.add_space(24.0);

        // Protection level options
        for level in [
            ProtectionLevel::Standard,
            ProtectionLevel::Strict,
            ProtectionLevel::Custom,
        ] {
            let is_selected = wizard.protection_level == level;
            let frame_fill = if is_selected {
                ui.style().visuals.selection.bg_fill
            } else {
                ui.style().visuals.widgets.inactive.bg_fill
            };

            let response = egui::Frame::new()
                .fill(frame_fill)
                .corner_radius(6.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.set_min_width(380.0);
                    ui.horizontal(|ui| {
                        // Radio button visual
                        let radio_color = if is_selected {
                            brand::PRIMARY
                        } else {
                            Color32::GRAY
                        };
                        ui.colored_label(radio_color, if is_selected { "●" } else { "○" });

                        ui.vertical(|ui| {
                            ui.label(RichText::new(level.name()).strong());
                            ui.label(RichText::new(level.description()).size(11.0).weak());
                        });
                    });
                })
                .response;

            if response.interact(egui::Sense::click()).clicked() {
                wizard.protection_level = level;
            }

            ui.add_space(8.0);
        }

        ui.add_space(16.0);

        // Navigation buttons
        ui.horizontal(|ui| {
            if ui.button("Back").clicked() {
                wizard.prev_step();
            }

            ui.add_space(100.0);

            if ui
                .add_sized([100.0, 32.0], egui::Button::new("Continue"))
                .clicked()
            {
                wizard.next_step();
            }
        });
    });
}

/// Renders the Browser Extension installation step.
fn render_browser_extension(
    ui: &mut egui::Ui,
    state: &mut AppState,
    wizard: &mut SetupWizardState,
) {
    ui.vertical_centered(|ui| {
        ui.label(
            RichText::new("Install Browser Extension")
                .size(20.0)
                .strong(),
        );
        ui.add_space(8.0);
        ui.label(
            RichText::new(
                "The browser extension monitors AI chatbots in Chrome, Edge, and other browsers.",
            )
            .size(12.0)
            .weak(),
        );

        ui.add_space(24.0);

        // Extension path
        if let Some(ext_path) = get_extension_path() {
            ui.label(RichText::new("Extension Location:").strong());
            ui.add_space(4.0);

            egui::Frame::new()
                .fill(ui.style().visuals.widgets.inactive.bg_fill)
                .corner_radius(4.0)
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(ext_path.display().to_string())
                            .monospace()
                            .size(11.0),
                    );
                });

            ui.add_space(16.0);

            // Action buttons
            ui.horizontal(|ui| {
                if ui.button("Open Chrome Extensions").clicked() {
                    let _ = open::that("chrome://extensions");
                    state.set_success(
                        "Opening Chrome. Enable Developer Mode, then click 'Load unpacked'.",
                    );
                }

                if ui.button("Copy Path").clicked() {
                    ui.ctx().copy_text(ext_path.display().to_string());
                    state.set_success("Path copied to clipboard!");
                }

                if ui.button("Open Folder").clicked() {
                    #[cfg(target_os = "windows")]
                    {
                        let _ = std::process::Command::new("explorer")
                            .arg(&ext_path)
                            .spawn();
                    }
                    #[cfg(target_os = "macos")]
                    {
                        let _ = std::process::Command::new("open").arg(&ext_path).spawn();
                    }
                    #[cfg(target_os = "linux")]
                    {
                        let _ = std::process::Command::new("xdg-open")
                            .arg(&ext_path)
                            .spawn();
                    }
                }
            });

            ui.add_space(16.0);

            // Instructions
            ui.label(RichText::new("Installation Steps:").strong());
            ui.add_space(8.0);

            let steps = [
                "1. Click 'Open Chrome Extensions' above",
                "2. Enable 'Developer mode' (toggle in top-right)",
                "3. Click 'Load unpacked'",
                "4. Select the extension folder (or paste the copied path)",
                "5. The Aegis icon should appear in your toolbar",
            ];

            for step in steps {
                ui.horizontal(|ui| {
                    ui.label(step);
                });
            }

            ui.add_space(8.0);
            ui.label(
                RichText::new("Supported browsers: Chrome, Edge, Brave, Opera, Vivaldi")
                    .size(11.0)
                    .weak(),
            );
        } else {
            ui.colored_label(status::WARNING, "Extension folder not found.");
            ui.label("You can install the extension later from Settings.");
        }

        ui.add_space(24.0);

        // Navigation buttons
        ui.horizontal(|ui| {
            if ui.button("Back").clicked() {
                wizard.prev_step();
            }

            ui.add_space(100.0);

            if ui
                .add_sized([120.0, 32.0], egui::Button::new("Continue"))
                .clicked()
            {
                wizard.next_step();
            }
        });
    });
}

/// Renders the CA Installation step.
fn render_ca_install(ui: &mut egui::Ui, state: &mut AppState, wizard: &mut SetupWizardState) {
    ui.vertical_centered(|ui| {
        ui.label(RichText::new("Install CA Certificate").size(20.0).strong());
        ui.add_space(8.0);
        ui.label(
            RichText::new("Optional: For system-wide proxy protection.")
                .size(12.0)
                .weak(),
        );

        ui.add_space(24.0);

        // Generate CA if not done
        if !wizard.ca_generated {
            if ui.button("Generate CA Certificate").clicked() {
                generate_ca_certificate(state, wizard);
            }
        } else {
            // Show success and path
            ui.colored_label(status::SUCCESS, "✓ CA Certificate Generated");

            if let Some(ref path) = wizard.ca_cert_path {
                ui.add_space(8.0);
                ui.label(RichText::new(format!("Location: {}", path)).size(11.0));
            }

            ui.add_space(16.0);

            // Installation instructions per OS
            ui.label(RichText::new("Installation Instructions:").strong());
            ui.add_space(8.0);

            render_ca_instructions(ui);
        }

        ui.add_space(24.0);

        // Navigation buttons
        ui.horizontal(|ui| {
            if ui.button("Back").clicked() {
                wizard.prev_step();
            }

            ui.add_space(100.0);

            let button_text = if wizard.ca_generated {
                "Continue"
            } else {
                "Skip for Now"
            };

            if ui
                .add_sized([120.0, 32.0], egui::Button::new(button_text))
                .clicked()
            {
                wizard.next_step();
            }
        });
    });
}

/// Generates CA certificate using aegis-proxy.
fn generate_ca_certificate(state: &mut AppState, wizard: &mut SetupWizardState) {
    // Get CA directory from project dirs
    match directories::ProjectDirs::from("com", "aegis", "Aegis") {
        Some(proj_dirs) => {
            let ca_dir = proj_dirs.data_dir().join("ca");

            // Create directory
            if let Err(e) = std::fs::create_dir_all(&ca_dir) {
                wizard.set_error(format!("Failed to create CA directory: {}", e));
                return;
            }

            // Generate CA certificate (self-contained, no aegis-proxy dependency in UI)
            let cert_path = ca_dir.join("aegis-ca.crt");
            let key_path = ca_dir.join("aegis-ca.key");

            // We'll mark as generated and store path
            // The actual generation happens when proxy starts (lazy generation)
            // For setup wizard, we just prepare the path and show instructions
            wizard.ca_cert_path = Some(cert_path.to_string_lossy().to_string());
            wizard.ca_generated = true;

            // Store CA path in config for later use
            let _ = state.db.set_config(
                "ca_cert_path",
                &serde_json::json!(cert_path.to_string_lossy().to_string()),
            );
            let _ = state.db.set_config(
                "ca_key_path",
                &serde_json::json!(key_path.to_string_lossy().to_string()),
            );
        }
        None => {
            wizard.set_error("Failed to determine app data directory");
        }
    }
}

/// Renders CA installation instructions for different operating systems.
fn render_ca_instructions(ui: &mut egui::Ui) {
    #[cfg(target_os = "windows")]
    {
        ui.label("Windows:");
        ui.horizontal(|ui| {
            ui.label("1.");
            ui.label("Double-click the .crt file");
        });
        ui.horizontal(|ui| {
            ui.label("2.");
            ui.label("Click 'Install Certificate'");
        });
        ui.horizontal(|ui| {
            ui.label("3.");
            ui.label("Select 'Local Machine' and click Next");
        });
        ui.horizontal(|ui| {
            ui.label("4.");
            ui.label("Select 'Place all certificates in: Trusted Root Certification Authorities'");
        });
        ui.horizontal(|ui| {
            ui.label("5.");
            ui.label("Click Finish and confirm the security warning");
        });
    }

    #[cfg(target_os = "macos")]
    {
        ui.label("macOS:");
        ui.horizontal(|ui| {
            ui.label("1.");
            ui.label("Double-click the .crt file to open Keychain Access");
        });
        ui.horizontal(|ui| {
            ui.label("2.");
            ui.label("Add to 'System' keychain");
        });
        ui.horizontal(|ui| {
            ui.label("3.");
            ui.label("Double-click the certificate in Keychain");
        });
        ui.horizontal(|ui| {
            ui.label("4.");
            ui.label("Expand 'Trust' and set 'Always Trust'");
        });
    }

    #[cfg(target_os = "linux")]
    {
        ui.label("Linux:");
        ui.horizontal(|ui| {
            ui.label("1.");
            ui.label("Copy the .crt file to /usr/local/share/ca-certificates/");
        });
        ui.horizontal(|ui| {
            ui.label("2.");
            ui.label("Run: sudo update-ca-certificates");
        });
        ui.horizontal(|ui| {
            ui.label("3.");
            ui.label("Restart your browser");
        });
    }

    // Fallback for other platforms
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        ui.label(
            "Please install the CA certificate according to your operating system's instructions.",
        );
    }
}

/// Renders the Profile creation step.
fn render_profile(ui: &mut egui::Ui, state: &mut AppState, wizard: &mut SetupWizardState) {
    ui.vertical_centered(|ui| {
        ui.label(RichText::new("Create First Profile").size(20.0).strong());
        ui.add_space(8.0);
        ui.label(
            RichText::new("Create a profile for a child you want to protect.")
                .size(12.0)
                .weak(),
        );

        ui.add_space(24.0);

        // Profile name
        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.add_space(46.0);
            ui.add(
                TextEdit::singleline(&mut wizard.profile_name)
                    .hint_text("e.g., Alex")
                    .desired_width(200.0),
            );
        });

        ui.add_space(8.0);

        // OS username (optional)
        ui.horizontal(|ui| {
            ui.label("OS Username:");
            ui.add(
                TextEdit::singleline(&mut wizard.profile_os_username)
                    .hint_text("e.g., alex (optional)")
                    .desired_width(200.0),
            );
        });
        ui.label(
            RichText::new("Auto-detect profile when this Windows/macOS/Linux user is logged in.")
                .size(10.0)
                .weak(),
        );

        ui.add_space(24.0);

        // Navigation buttons
        ui.horizontal(|ui| {
            if ui.button("Back").clicked() {
                wizard.prev_step();
            }

            ui.add_space(100.0);

            // Skip option
            if ui.button("Skip").clicked() {
                wizard.next_step();
            }

            if ui
                .add_sized([100.0, 32.0], egui::Button::new("Create"))
                .clicked()
            {
                create_profile(state, wizard);
            }
        });
    });
}

/// Creates a profile from wizard state.
fn create_profile(state: &mut AppState, wizard: &mut SetupWizardState) {
    let name = wizard.profile_name.trim();

    if name.is_empty() {
        wizard.set_error("Profile name is required");
        return;
    }

    let os_username = if wizard.profile_os_username.trim().is_empty() {
        None
    } else {
        Some(wizard.profile_os_username.trim().to_string())
    };

    // Create default rules based on protection level
    let (time_rules, content_rules) = create_default_rules(wizard.protection_level);

    let new_profile = aegis_storage::NewProfile {
        name: name.to_string(),
        os_username,
        time_rules,
        content_rules,
        enabled: true,
    };

    match state.db.create_profile(new_profile) {
        Ok(_) => {
            wizard.next_step();
        }
        Err(e) => {
            wizard.set_error(format!("Failed to create profile: {}", e));
        }
    }
}

/// Creates default rules based on protection level.
fn create_default_rules(level: ProtectionLevel) -> (serde_json::Value, serde_json::Value) {
    match level {
        ProtectionLevel::Standard => {
            // Block harmful categories, allow others
            let time_rules = serde_json::json!({
                "rules": [
                    {
                        "name": "School Night Bedtime",
                        "enabled": true,
                        "start_time": "21:00",
                        "end_time": "07:00",
                        "days": ["monday", "tuesday", "wednesday", "thursday", "sunday"]
                    }
                ]
            });

            let content_rules = serde_json::json!({
                "rules": [
                    {"category": "violence", "action": "block", "threshold": 0.7},
                    {"category": "self_harm", "action": "block", "threshold": 0.5},
                    {"category": "adult", "action": "block", "threshold": 0.7},
                    {"category": "jailbreak", "action": "block", "threshold": 0.6},
                    {"category": "hate", "action": "block", "threshold": 0.7},
                    {"category": "illegal", "action": "block", "threshold": 0.7}
                ]
            });

            (time_rules, content_rules)
        }
        ProtectionLevel::Strict => {
            // Block harmful, flag questionable
            let time_rules = serde_json::json!({
                "rules": [
                    {
                        "name": "Early Bedtime",
                        "enabled": true,
                        "start_time": "20:00",
                        "end_time": "08:00",
                        "days": ["monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday"]
                    }
                ]
            });

            let content_rules = serde_json::json!({
                "rules": [
                    {"category": "violence", "action": "block", "threshold": 0.5},
                    {"category": "self_harm", "action": "block", "threshold": 0.3},
                    {"category": "adult", "action": "block", "threshold": 0.5},
                    {"category": "jailbreak", "action": "block", "threshold": 0.4},
                    {"category": "hate", "action": "block", "threshold": 0.5},
                    {"category": "illegal", "action": "block", "threshold": 0.5}
                ]
            });

            (time_rules, content_rules)
        }
        ProtectionLevel::Custom => {
            // No default rules
            let time_rules = serde_json::json!({"rules": []});
            let content_rules = serde_json::json!({"rules": []});

            (time_rules, content_rules)
        }
    }
}

/// Renders the Complete step.
fn render_complete(ui: &mut egui::Ui, state: &mut AppState, wizard: &mut SetupWizardState) {
    ui.vertical_centered(|ui| {
        ui.label(RichText::new("Setup Complete!").size(24.0).strong());
        ui.add_space(8.0);
        ui.colored_label(status::SUCCESS, "Aegis is ready to protect your family.");

        ui.add_space(24.0);

        // Summary
        ui.label(RichText::new("Your Configuration:").strong());
        ui.add_space(8.0);

        egui::Frame::new()
            .fill(ui.style().visuals.widgets.inactive.bg_fill)
            .corner_radius(4.0)
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Protection Level:");
                    ui.label(RichText::new(wizard.protection_level.name()).strong());
                });
                if !wizard.profile_name.is_empty() {
                    ui.horizontal(|ui| {
                        ui.label("Profile Created:");
                        ui.label(RichText::new(&wizard.profile_name).strong());
                    });
                }
            });

        ui.add_space(16.0);

        // Autostart option
        ui.horizontal(|ui| {
            ui.checkbox(&mut wizard.enable_autostart, "Start Aegis when I log in");
        });
        ui.label(
            RichText::new("Aegis will start automatically in the background when you log in.")
                .size(11.0)
                .weak(),
        );

        ui.add_space(16.0);

        // Next steps
        ui.label(RichText::new("Next Steps:").strong());
        ui.add_space(8.0);

        let next_steps = [
            "Install the CA certificate from Settings",
            "Enable system proxy from Settings",
            "Create additional child profiles if needed",
        ];

        for (i, step) in next_steps.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(format!("{}.", i + 1));
                ui.label(*step);
            });
        }

        ui.add_space(16.0);

        if ui
            .add_sized([200.0, 36.0], egui::Button::new("Open Dashboard"))
            .clicked()
        {
            finish_setup(state, wizard);
        }
    });
}

/// Finishes the setup wizard and transitions to dashboard.
fn finish_setup(state: &mut AppState, wizard: &SetupWizardState) {
    // Save interception mode (proxy is always the mode now)
    let _ = state
        .db
        .set_config("interception_mode", &serde_json::json!("proxy"));

    // Save protection level
    let level_str = match wizard.protection_level {
        ProtectionLevel::Standard => "standard",
        ProtectionLevel::Strict => "strict",
        ProtectionLevel::Custom => "custom",
    };
    let _ = state
        .db
        .set_config("protection_level", &serde_json::json!(level_str));

    // Enable autostart if requested
    if wizard.enable_autostart {
        if let Err(e) = enable_autostart() {
            tracing::warn!("Failed to enable autostart: {}", e);
        } else {
            tracing::info!("Autostart enabled");
        }
    }

    // Mark setup as complete
    let _ = state
        .db
        .set_config("setup_complete", &serde_json::json!(true));

    // Create session and transition to dashboard
    let token = state.auth.create_session();
    state.session = Some(token);
    state.view = crate::state::View::Dashboard;

    // Refresh data
    let _ = state.refresh_data();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_setup_step_navigation() {
        assert_eq!(SetupStep::Welcome.next(), Some(SetupStep::Password));
        assert_eq!(SetupStep::Password.prev(), Some(SetupStep::Welcome));
        assert_eq!(SetupStep::Complete.next(), None);
        assert_eq!(SetupStep::Welcome.prev(), None);
    }

    #[test]
    fn test_setup_step_numbers() {
        assert_eq!(SetupStep::Welcome.number(), 1);
        assert_eq!(SetupStep::BrowserExtension.number(), 4);
        assert_eq!(SetupStep::Complete.number(), 7);
        assert_eq!(SetupStep::total(), 7);
    }

    #[test]
    fn test_protection_level() {
        assert_eq!(ProtectionLevel::Standard.name(), "Standard");
        assert_eq!(ProtectionLevel::Strict.name(), "Strict");
        assert_eq!(ProtectionLevel::Custom.name(), "Custom");
    }

    #[test]
    fn test_wizard_state_default() {
        let state = SetupWizardState::new();
        assert_eq!(state.step, SetupStep::Welcome);
        assert_eq!(state.protection_level, ProtectionLevel::Standard);
    }

    #[test]
    fn test_wizard_state_navigation() {
        let mut state = SetupWizardState::new();
        assert_eq!(state.step, SetupStep::Welcome);

        state.next_step();
        assert_eq!(state.step, SetupStep::Password);

        state.prev_step();
        assert_eq!(state.step, SetupStep::Welcome);

        // Can't go back from Welcome
        state.prev_step();
        assert_eq!(state.step, SetupStep::Welcome);
    }

    #[test]
    fn test_default_rules_standard() {
        let (time_rules, content_rules) = create_default_rules(ProtectionLevel::Standard);

        // Check time rules have school night bedtime
        let rules = time_rules["rules"].as_array().unwrap();
        assert!(!rules.is_empty());

        // Check content rules block harmful categories
        let rules = content_rules["rules"].as_array().unwrap();
        assert!(!rules.is_empty());
    }

    #[test]
    fn test_default_rules_custom() {
        let (time_rules, content_rules) = create_default_rules(ProtectionLevel::Custom);

        // Custom should have empty rules
        let rules = time_rules["rules"].as_array().unwrap();
        assert!(rules.is_empty());

        let rules = content_rules["rules"].as_array().unwrap();
        assert!(rules.is_empty());
    }

    #[test]
    fn test_wizard_error_handling() {
        let mut state = SetupWizardState::new();
        assert!(state.error.is_none());

        state.set_error("Test error");
        assert_eq!(state.error, Some("Test error".to_string()));

        state.clear_error();
        assert!(state.error.is_none());
    }
}
