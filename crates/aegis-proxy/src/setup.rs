//! Cross-platform proxy setup automation.
//!
//! Handles:
//! - CA certificate installation into system trust stores
//! - System proxy configuration
//! - Setup verification

use std::path::Path;
use std::process::Command;

/// Proxy setup configuration.
#[derive(Debug, Clone)]
pub struct ProxySetup {
    /// Proxy host.
    pub host: String,
    /// Proxy port.
    pub port: u16,
    /// Path to CA certificate.
    pub ca_cert_path: std::path::PathBuf,
}

impl ProxySetup {
    /// Creates a new setup configuration.
    pub fn new(
        host: impl Into<String>,
        port: u16,
        ca_cert_path: impl Into<std::path::PathBuf>,
    ) -> Self {
        Self {
            host: host.into(),
            port,
            ca_cert_path: ca_cert_path.into(),
        }
    }

    /// Returns the proxy URL.
    pub fn proxy_url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }
}

/// Result of a setup operation.
#[derive(Debug, Clone)]
pub struct SetupResult {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Human-readable message.
    pub message: String,
    /// Whether admin/root privileges are required.
    pub needs_admin: bool,
}

impl SetupResult {
    fn success(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            needs_admin: false,
        }
    }

    fn failure(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            needs_admin: false,
        }
    }

    #[allow(dead_code)]
    fn needs_admin(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            needs_admin: true,
        }
    }
}

// ============================================================================
// CA Certificate Installation
// ============================================================================

/// Installs the CA certificate into the system trust store.
pub fn install_ca_certificate(cert_path: &Path) -> SetupResult {
    if !cert_path.exists() {
        return SetupResult::failure(format!(
            "CA certificate not found at: {}",
            cert_path.display()
        ));
    }

    #[cfg(target_os = "windows")]
    {
        install_ca_windows(cert_path)
    }

    #[cfg(target_os = "macos")]
    {
        install_ca_macos(cert_path)
    }

    #[cfg(target_os = "linux")]
    {
        install_ca_linux(cert_path)
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        SetupResult::failure("Unsupported operating system")
    }
}

/// Uninstalls the CA certificate from the system trust store.
pub fn uninstall_ca_certificate(cert_path: &Path) -> SetupResult {
    #[cfg(target_os = "windows")]
    {
        uninstall_ca_windows(cert_path)
    }

    #[cfg(target_os = "macos")]
    {
        uninstall_ca_macos()
    }

    #[cfg(target_os = "linux")]
    {
        uninstall_ca_linux()
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = cert_path;
        SetupResult::failure("Unsupported operating system")
    }
}

/// Checks if the CA certificate is installed.
pub fn is_ca_installed(cert_path: &Path) -> bool {
    #[cfg(target_os = "windows")]
    {
        is_ca_installed_windows(cert_path)
    }

    #[cfg(target_os = "macos")]
    {
        is_ca_installed_macos()
    }

    #[cfg(target_os = "linux")]
    {
        is_ca_installed_linux()
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = cert_path;
        false
    }
}

// ============================================================================
// System Proxy Configuration
// ============================================================================

/// Enables the system proxy.
pub fn enable_system_proxy(host: &str, port: u16) -> SetupResult {
    #[cfg(target_os = "windows")]
    {
        enable_proxy_windows(host, port)
    }

    #[cfg(target_os = "macos")]
    {
        enable_proxy_macos(host, port)
    }

    #[cfg(target_os = "linux")]
    {
        enable_proxy_linux(host, port)
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = (host, port);
        SetupResult::failure("Unsupported operating system")
    }
}

/// Disables the system proxy.
pub fn disable_system_proxy() -> SetupResult {
    #[cfg(target_os = "windows")]
    {
        disable_proxy_windows()
    }

    #[cfg(target_os = "macos")]
    {
        disable_proxy_macos()
    }

    #[cfg(target_os = "linux")]
    {
        disable_proxy_linux()
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        SetupResult::failure("Unsupported operating system")
    }
}

/// Checks if the system proxy is enabled and pointing to our proxy.
pub fn is_proxy_enabled(host: &str, port: u16) -> bool {
    #[cfg(target_os = "windows")]
    {
        is_proxy_enabled_windows(host, port)
    }

    #[cfg(target_os = "macos")]
    {
        is_proxy_enabled_macos(host, port)
    }

    #[cfg(target_os = "linux")]
    {
        is_proxy_enabled_linux(host, port)
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = (host, port);
        false
    }
}

// ============================================================================
// Full Setup/Teardown
// ============================================================================

/// Performs full proxy setup (CA + system proxy).
pub fn setup_proxy(config: &ProxySetup) -> Vec<SetupResult> {
    let mut results = Vec::new();

    // Step 1: Install CA certificate
    let ca_result = install_ca_certificate(&config.ca_cert_path);
    results.push(ca_result.clone());

    // Step 2: Configure system proxy (only if CA succeeded)
    if ca_result.success {
        let proxy_result = enable_system_proxy(&config.host, config.port);
        results.push(proxy_result);
    }

    results
}

/// Removes proxy setup (disables proxy + optionally removes CA).
pub fn teardown_proxy(config: &ProxySetup, remove_ca: bool) -> Vec<SetupResult> {
    let mut results = Vec::new();

    // Step 1: Disable system proxy
    let proxy_result = disable_system_proxy();
    results.push(proxy_result);

    // Step 2: Remove CA certificate if requested
    if remove_ca {
        let ca_result = uninstall_ca_certificate(&config.ca_cert_path);
        results.push(ca_result);
    }

    results
}

// ============================================================================
// Windows Implementation
// ============================================================================

#[cfg(target_os = "windows")]
fn install_ca_windows(cert_path: &Path) -> SetupResult {
    use std::os::windows::process::CommandExt;

    let cert_path_str = cert_path.to_string_lossy();
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    // First, remove any old CA certificates (both old "rcgen" and existing "Aegis")
    let names_to_remove = ["rcgen self signed cert", "Aegis Root CA"];
    for name in &names_to_remove {
        // Try user store first (doesn't need admin)
        let _ = Command::new("certutil")
            .args(["-delstore", "-user", "Root", name])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
        // Try machine store (may fail without admin, that's ok)
        let _ = Command::new("certutil")
            .args(["-delstore", "Root", name])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
    }

    // Try machine store first with UAC elevation (for multi-user support)
    // Use PowerShell Start-Process with -Verb RunAs to trigger UAC
    let ps_script = format!(
        r#"
        $certPath = '{}'
        $process = Start-Process -FilePath 'certutil' -ArgumentList '-addstore', 'Root', $certPath -Verb RunAs -Wait -PassThru -WindowStyle Hidden
        exit $process.ExitCode
        "#,
        cert_path_str.replace('\'', "''")
    );

    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &ps_script,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    match output {
        Ok(out) => {
            if out.status.success() {
                return SetupResult::success("CA certificate installed to machine trust store");
            }

            // UAC was cancelled or failed, try user store as fallback
            let user_output = Command::new("certutil")
                .args(["-addstore", "-user", "Root", &cert_path_str])
                .creation_flags(CREATE_NO_WINDOW)
                .output();

            match user_output {
                Ok(u_out) => {
                    if u_out.status.success() {
                        SetupResult::success(
                            "CA certificate installed to user trust store (current user only)",
                        )
                    } else {
                        let stderr = String::from_utf8_lossy(&u_out.stderr);
                        SetupResult::failure(format!(
                            "UAC elevation cancelled and user store failed: {}",
                            stderr.trim()
                        ))
                    }
                }
                Err(e) => SetupResult::failure(format!("Failed to run certutil: {}", e)),
            }
        }
        Err(e) => SetupResult::failure(format!("Failed to request elevation: {}", e)),
    }
}

#[cfg(target_os = "windows")]
fn uninstall_ca_windows(_cert_path: &Path) -> SetupResult {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    // Remove both old and new CA names from root store
    let names_to_remove = ["rcgen self signed cert", "Aegis Root CA"];

    // Build PowerShell script to remove from machine store with UAC
    let names_list = names_to_remove
        .iter()
        .map(|n| format!("'{}'", n))
        .collect::<Vec<_>>()
        .join(", ");

    let ps_script = format!(
        r#"
        $names = @({})
        foreach ($name in $names) {{
            certutil -delstore Root $name 2>$null
        }}
        "#,
        names_list
    );

    // Request elevation for machine store removal
    let elevated_script = format!(
        r#"Start-Process -FilePath 'powershell' -ArgumentList '-NoProfile', '-ExecutionPolicy', 'Bypass', '-Command', '{}' -Verb RunAs -Wait -WindowStyle Hidden"#,
        ps_script.replace('\'', "''")
    );

    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &elevated_script,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    // Also remove from user store (doesn't need elevation)
    for name in &names_to_remove {
        let _ = Command::new("certutil")
            .args(["-delstore", "-user", "Root", name])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
    }

    match output {
        Ok(_) => SetupResult::success("CA certificate(s) removed from trust store"),
        Err(e) => SetupResult::failure(format!("Failed to remove certificates: {}", e)),
    }
}

#[cfg(target_os = "windows")]
fn is_ca_installed_windows(_cert_path: &Path) -> bool {
    // Check both user and machine stores for "Aegis Root CA"
    let stores = [vec!["-store", "Root"], vec!["-store", "-user", "Root"]];

    for store_args in &stores {
        let output = Command::new("certutil").args(store_args).output();

        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            // Check for the new CA name
            if stdout.contains("Aegis Root CA") {
                return true;
            }
        }
    }

    false
}

#[cfg(target_os = "windows")]
fn enable_proxy_windows(host: &str, port: u16) -> SetupResult {
    use std::os::windows::process::CommandExt;

    let proxy_server = format!("{}:{}", host, port);

    // Use PowerShell to set the proxy via registry
    let ps_script = format!(
        r#"
        $regPath = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Internet Settings'
        Set-ItemProperty -Path $regPath -Name ProxyEnable -Value 1
        Set-ItemProperty -Path $regPath -Name ProxyServer -Value '{}'
        Set-ItemProperty -Path $regPath -Name ProxyOverride -Value '<local>'
        "#,
        proxy_server
    );

    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &ps_script,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    match output {
        Ok(out) => {
            if out.status.success() {
                // Notify WinINet of the change
                let _ = Command::new("powershell")
                    .args(["-NoProfile", "-Command",
                        "[System.Runtime.InteropServices.RuntimeEnvironment]::FromGlobalAccessCache([System.Net.WebRequest])"
                    ])
                    .creation_flags(CREATE_NO_WINDOW)
                    .output();

                SetupResult::success(format!("System proxy enabled: {}", proxy_server))
            } else {
                let stderr = String::from_utf8_lossy(&out.stderr);
                SetupResult::failure(format!("Failed to enable proxy: {}", stderr))
            }
        }
        Err(e) => SetupResult::failure(format!("Failed to run PowerShell: {}", e)),
    }
}

#[cfg(target_os = "windows")]
fn disable_proxy_windows() -> SetupResult {
    use std::os::windows::process::CommandExt;

    let ps_script = r#"
        $regPath = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Internet Settings'
        Set-ItemProperty -Path $regPath -Name ProxyEnable -Value 0
    "#;

    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            ps_script,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    match output {
        Ok(out) => {
            if out.status.success() {
                SetupResult::success("System proxy disabled")
            } else {
                let stderr = String::from_utf8_lossy(&out.stderr);
                SetupResult::failure(format!("Failed to disable proxy: {}", stderr))
            }
        }
        Err(e) => SetupResult::failure(format!("Failed to run PowerShell: {}", e)),
    }
}

#[cfg(target_os = "windows")]
fn is_proxy_enabled_windows(host: &str, port: u16) -> bool {
    use std::os::windows::process::CommandExt;

    let ps_script = r#"
        $regPath = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Internet Settings'
        $enabled = (Get-ItemProperty -Path $regPath -Name ProxyEnable -ErrorAction SilentlyContinue).ProxyEnable
        $server = (Get-ItemProperty -Path $regPath -Name ProxyServer -ErrorAction SilentlyContinue).ProxyServer
        Write-Output "$enabled|$server"
    "#;

    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            ps_script,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let expected_server = format!("{}:{}", host, port);
            stdout.contains("1|") && stdout.contains(&expected_server)
        }
        Err(_) => false,
    }
}

// ============================================================================
// macOS Implementation
// ============================================================================

#[cfg(target_os = "macos")]
fn install_ca_macos(cert_path: &Path) -> SetupResult {
    let cert_path_str = cert_path.to_string_lossy();

    // Try system keychain with admin prompt (for multi-user support)
    // Uses osascript to show native macOS admin password dialog
    let script = format!(
        r#"do shell script "security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain '{}'" with administrator privileges"#,
        cert_path_str.replace('\'', "'\\''")
    );

    let output = Command::new("osascript").args(["-e", &script]).output();

    match output {
        Ok(out) => {
            if out.status.success() {
                return SetupResult::success("CA certificate installed to system keychain");
            }

            // Admin prompt cancelled or failed, try user keychain as fallback
            let user_output = Command::new("security")
                .args([
                    "add-trusted-cert",
                    "-r",
                    "trustRoot",
                    "-k",
                    &format!(
                        "{}/Library/Keychains/login.keychain-db",
                        std::env::var("HOME").unwrap_or_default()
                    ),
                    &cert_path_str,
                ])
                .output();

            match user_output {
                Ok(u_out) => {
                    if u_out.status.success() {
                        SetupResult::success(
                            "CA certificate installed to user keychain (current user only)",
                        )
                    } else {
                        let stderr = String::from_utf8_lossy(&u_out.stderr);
                        SetupResult::failure(format!(
                            "Admin prompt cancelled and user keychain failed: {}",
                            stderr.trim()
                        ))
                    }
                }
                Err(e) => SetupResult::failure(format!("Failed to run security command: {}", e)),
            }
        }
        Err(e) => SetupResult::failure(format!("Failed to request admin privileges: {}", e)),
    }
}

#[cfg(target_os = "macos")]
fn uninstall_ca_macos() -> SetupResult {
    // Try to remove from system keychain with admin prompt
    let script = r#"do shell script "security delete-certificate -c 'Aegis Root CA' /Library/Keychains/System.keychain 2>/dev/null; security delete-certificate -c 'rcgen self signed cert' /Library/Keychains/System.keychain 2>/dev/null" with administrator privileges"#;

    let output = Command::new("osascript").args(["-e", script]).output();

    // Also try to remove from user keychain (doesn't need admin)
    let _ = Command::new("security")
        .args(["delete-certificate", "-c", "Aegis Root CA", "-t"])
        .output();
    let _ = Command::new("security")
        .args(["delete-certificate", "-c", "rcgen self signed cert", "-t"])
        .output();

    match output {
        Ok(_) => SetupResult::success("CA certificate removed from keychain"),
        Err(e) => SetupResult::failure(format!("Failed to remove certificate: {}", e)),
    }
}

#[cfg(target_os = "macos")]
fn is_ca_installed_macos() -> bool {
    let output = Command::new("security")
        .args(["find-certificate", "-c", "rcgen self signed cert"])
        .output();

    match output {
        Ok(out) => out.status.success(),
        Err(_) => false,
    }
}

#[cfg(target_os = "macos")]
fn get_active_network_service() -> Option<String> {
    // Get the primary network service
    let output = Command::new("networksetup")
        .args(["-listallnetworkservices"])
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Try common service names
    for service in ["Wi-Fi", "Ethernet", "USB 10/100/1000 LAN"] {
        if stdout.contains(service) {
            return Some(service.to_string());
        }
    }

    // Return first non-asterisk service
    stdout
        .lines()
        .skip(1) // Skip header
        .find(|line| !line.starts_with('*'))
        .map(|s| s.to_string())
}

#[cfg(target_os = "macos")]
fn enable_proxy_macos(host: &str, port: u16) -> SetupResult {
    let service = match get_active_network_service() {
        Some(s) => s,
        None => return SetupResult::failure("Could not find active network service"),
    };

    // Set HTTP proxy
    let http_result = Command::new("networksetup")
        .args(["-setwebproxy", &service, host, &port.to_string()])
        .output();

    // Set HTTPS proxy
    let https_result = Command::new("networksetup")
        .args(["-setsecurewebproxy", &service, host, &port.to_string()])
        .output();

    // Enable both
    let _ = Command::new("networksetup")
        .args(["-setwebproxystate", &service, "on"])
        .output();
    let _ = Command::new("networksetup")
        .args(["-setsecurewebproxystate", &service, "on"])
        .output();

    match (http_result, https_result) {
        (Ok(h), Ok(s)) if h.status.success() && s.status.success() => SetupResult::success(
            format!("System proxy enabled on {} ({}:{})", service, host, port),
        ),
        _ => SetupResult::needs_admin("Failed to set proxy. May need administrator privileges."),
    }
}

#[cfg(target_os = "macos")]
fn disable_proxy_macos() -> SetupResult {
    let service = match get_active_network_service() {
        Some(s) => s,
        None => return SetupResult::failure("Could not find active network service"),
    };

    let _ = Command::new("networksetup")
        .args(["-setwebproxystate", &service, "off"])
        .output();
    let _ = Command::new("networksetup")
        .args(["-setsecurewebproxystate", &service, "off"])
        .output();

    SetupResult::success("System proxy disabled")
}

#[cfg(target_os = "macos")]
fn is_proxy_enabled_macos(host: &str, port: u16) -> bool {
    let service = match get_active_network_service() {
        Some(s) => s,
        None => return false,
    };

    let output = Command::new("networksetup")
        .args(["-getwebproxy", &service])
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout.contains("Enabled: Yes")
                && stdout.contains(&format!("Server: {}", host))
                && stdout.contains(&format!("Port: {}", port))
        }
        Err(_) => false,
    }
}

// ============================================================================
// Linux Implementation
// ============================================================================

#[cfg(target_os = "linux")]
fn install_ca_linux(cert_path: &Path) -> SetupResult {
    let cert_path_str = cert_path.to_string_lossy();

    // Determine which elevation tool to use (pkexec for GUI, sudo for terminal)
    let elevation_cmd =
        if std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok() {
            "pkexec" // GUI environment - use PolicyKit for graphical prompt
        } else {
            "sudo" // Terminal - use sudo
        };

    // Detect distro and use appropriate method
    if Path::new("/usr/local/share/ca-certificates").exists() {
        // Debian/Ubuntu
        let dest = "/usr/local/share/ca-certificates/aegis-ca.crt";

        let copy_result = Command::new(elevation_cmd)
            .args(["cp", &cert_path_str, dest])
            .output();

        match copy_result {
            Ok(out) if out.status.success() => {
                let update_result = Command::new(elevation_cmd)
                    .args(["update-ca-certificates"])
                    .output();

                match update_result {
                    Ok(u) if u.status.success() => {
                        SetupResult::success("CA certificate installed (Debian/Ubuntu)")
                    }
                    _ => SetupResult::failure("Failed to update CA certificates"),
                }
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                if stderr.contains("dismissed") || stderr.contains("cancelled") {
                    SetupResult::failure("Authentication cancelled by user")
                } else {
                    SetupResult::failure(format!("Failed to install certificate: {}", stderr))
                }
            }
            Err(e) => SetupResult::failure(format!("Failed to run {}: {}", elevation_cmd, e)),
        }
    } else if Path::new("/etc/pki/ca-trust/source/anchors").exists() {
        // Fedora/RHEL/CentOS
        let dest = "/etc/pki/ca-trust/source/anchors/aegis-ca.crt";

        let copy_result = Command::new(elevation_cmd)
            .args(["cp", &cert_path_str, dest])
            .output();

        match copy_result {
            Ok(out) if out.status.success() => {
                let update_result = Command::new(elevation_cmd)
                    .args(["update-ca-trust", "extract"])
                    .output();

                match update_result {
                    Ok(u) if u.status.success() => {
                        SetupResult::success("CA certificate installed (Fedora/RHEL)")
                    }
                    _ => SetupResult::failure("Failed to update CA trust"),
                }
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                if stderr.contains("dismissed") || stderr.contains("cancelled") {
                    SetupResult::failure("Authentication cancelled by user")
                } else {
                    SetupResult::failure(format!("Failed to install certificate: {}", stderr))
                }
            }
            Err(e) => SetupResult::failure(format!("Failed to run {}: {}", elevation_cmd, e)),
        }
    } else if Path::new("/etc/ca-certificates/trust-source/anchors").exists() {
        // Arch Linux
        let dest = "/etc/ca-certificates/trust-source/anchors/aegis-ca.crt";

        let copy_result = Command::new(elevation_cmd)
            .args(["cp", &cert_path_str, dest])
            .output();

        match copy_result {
            Ok(out) if out.status.success() => {
                let update_result = Command::new(elevation_cmd)
                    .args(["trust", "extract-compat"])
                    .output();

                match update_result {
                    Ok(u) if u.status.success() => {
                        SetupResult::success("CA certificate installed (Arch Linux)")
                    }
                    _ => SetupResult::failure("Failed to extract trust"),
                }
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                if stderr.contains("dismissed") || stderr.contains("cancelled") {
                    SetupResult::failure("Authentication cancelled by user")
                } else {
                    SetupResult::failure(format!("Failed to install certificate: {}", stderr))
                }
            }
            Err(e) => SetupResult::failure(format!("Failed to run {}: {}", elevation_cmd, e)),
        }
    } else {
        SetupResult::failure(
            "Unknown Linux distribution. Please install the CA certificate manually.",
        )
    }
}

#[cfg(target_os = "linux")]
fn uninstall_ca_linux() -> SetupResult {
    // Determine which elevation tool to use (pkexec for GUI, sudo for terminal)
    let elevation_cmd =
        if std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok() {
            "pkexec" // GUI environment - use PolicyKit for graphical prompt
        } else {
            "sudo" // Terminal - use sudo
        };

    // Try all known locations
    let locations = [
        "/usr/local/share/ca-certificates/aegis-ca.crt",
        "/etc/pki/ca-trust/source/anchors/aegis-ca.crt",
        "/etc/ca-certificates/trust-source/anchors/aegis-ca.crt",
    ];

    let mut removed = false;
    for loc in &locations {
        if Path::new(loc).exists() {
            let result = Command::new(elevation_cmd).args(["rm", loc]).output();
            if let Ok(out) = result {
                if out.status.success() {
                    removed = true;
                }
            }
        }
    }

    // Update certificates based on distro
    if Path::new("/usr/local/share/ca-certificates").exists() {
        let _ = Command::new(elevation_cmd)
            .args(["update-ca-certificates"])
            .output();
    }
    if Path::new("/etc/pki/ca-trust").exists() {
        let _ = Command::new(elevation_cmd)
            .args(["update-ca-trust", "extract"])
            .output();
    }
    if Path::new("/etc/ca-certificates/trust-source").exists() {
        let _ = Command::new(elevation_cmd)
            .args(["trust", "extract-compat"])
            .output();
    }

    if removed {
        SetupResult::success("CA certificate removed")
    } else {
        SetupResult::failure("No certificate found to remove (may not be installed)")
    }
}

#[cfg(target_os = "linux")]
fn is_ca_installed_linux() -> bool {
    let locations = [
        "/usr/local/share/ca-certificates/aegis-ca.crt",
        "/etc/pki/ca-trust/source/anchors/aegis-ca.crt",
        "/etc/ca-certificates/trust-source/anchors/aegis-ca.crt",
    ];

    locations.iter().any(|loc| Path::new(loc).exists())
}

#[cfg(target_os = "linux")]
fn enable_proxy_linux(host: &str, port: u16) -> SetupResult {
    let proxy_url = format!("http://{}:{}", host, port);

    // Try GNOME settings first
    let gnome_result = Command::new("gsettings")
        .args(["set", "org.gnome.system.proxy", "mode", "manual"])
        .output();

    if gnome_result.is_ok() {
        let _ = Command::new("gsettings")
            .args(["set", "org.gnome.system.proxy.http", "host", host])
            .output();
        let _ = Command::new("gsettings")
            .args([
                "set",
                "org.gnome.system.proxy.http",
                "port",
                &port.to_string(),
            ])
            .output();
        let _ = Command::new("gsettings")
            .args(["set", "org.gnome.system.proxy.https", "host", host])
            .output();
        let _ = Command::new("gsettings")
            .args([
                "set",
                "org.gnome.system.proxy.https",
                "port",
                &port.to_string(),
            ])
            .output();

        return SetupResult::success(format!("GNOME proxy configured: {}", proxy_url));
    }

    // Fallback: suggest environment variables
    SetupResult::success(format!(
        "Add to ~/.bashrc or /etc/environment:\nexport http_proxy={}\nexport https_proxy={}\nexport HTTP_PROXY={}\nexport HTTPS_PROXY={}",
        proxy_url, proxy_url, proxy_url, proxy_url
    ))
}

#[cfg(target_os = "linux")]
fn disable_proxy_linux() -> SetupResult {
    // Try GNOME settings
    let _ = Command::new("gsettings")
        .args(["set", "org.gnome.system.proxy", "mode", "none"])
        .output();

    SetupResult::success("GNOME proxy disabled. Remove environment variables manually if set.")
}

#[cfg(target_os = "linux")]
fn is_proxy_enabled_linux(host: &str, port: u16) -> bool {
    // Check GNOME settings
    let output = Command::new("gsettings")
        .args(["get", "org.gnome.system.proxy", "mode"])
        .output();

    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        if stdout.contains("manual") {
            // Check host/port
            let host_out = Command::new("gsettings")
                .args(["get", "org.gnome.system.proxy.http", "host"])
                .output();
            let port_out = Command::new("gsettings")
                .args(["get", "org.gnome.system.proxy.http", "port"])
                .output();

            if let (Ok(h), Ok(p)) = (host_out, port_out) {
                let h_str = String::from_utf8_lossy(&h.stdout);
                let p_str = String::from_utf8_lossy(&p.stdout);
                return h_str.contains(host) && p_str.contains(&port.to_string());
            }
        }
    }

    // Check environment variables
    std::env::var("http_proxy")
        .or_else(|_| std::env::var("HTTP_PROXY"))
        .map(|v| v.contains(&format!("{}:{}", host, port)))
        .unwrap_or(false)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_setup_new() {
        let setup = ProxySetup::new("127.0.0.1", 8766, "/tmp/ca.crt");
        assert_eq!(setup.host, "127.0.0.1");
        assert_eq!(setup.port, 8766);
        assert_eq!(setup.proxy_url(), "http://127.0.0.1:8766");
    }

    #[test]
    fn test_setup_result_success() {
        let result = SetupResult::success("Test success");
        assert!(result.success);
        assert!(!result.needs_admin);
    }

    #[test]
    fn test_setup_result_failure() {
        let result = SetupResult::failure("Test failure");
        assert!(!result.success);
        assert!(!result.needs_admin);
    }

    #[test]
    fn test_setup_result_needs_admin() {
        let result = SetupResult::needs_admin("Need admin");
        assert!(!result.success);
        assert!(result.needs_admin);
    }
}
