// Build script to embed Windows resources (icon) into the executable
// This ensures the app icon shows in Start Menu, taskbar, and File Explorer

fn main() {
    // Only run on Windows
    #[cfg(target_os = "windows")]
    {
        // Embed the Windows icon resource
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icons/icon.ico");

        // Set version info from Cargo.toml
        res.set("ProductName", "Aegis AI Safety");
        res.set("FileDescription", "AI Safety Platform for Families");
        res.set("LegalCopyright", "Copyright © 2024 Aegis Team");

        if let Err(e) = res.compile() {
            eprintln!("Warning: Failed to compile Windows resources: {}", e);
            // Don't fail the build, just warn
        }
    }
}
