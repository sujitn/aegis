# Aegis

AI Safety for Families - Privacy-first parental controls for AI chatbots.

## Overview

Aegis protects children using AI chatbots (ChatGPT, Claude, Gemini, etc.) by:
- Blocking harmful prompts (jailbreaks, inappropriate content)
- Enforcing time limits per user profile
- Logging activity for parental review

**All processing is local. No cloud. No data collection.**

## Features

- **Tiered Classification**: Fast keyword matching + ML-based analysis
- **User Profiles**: Different rules per child (linked to OS username)
- **Two Interception Modes**:
  - Browser Extension (Chrome) - browser only
  - MITM Proxy - all applications
- **Parent Dashboard**: View logs, manage rules, configure profiles
- **System Tray**: Quick status and controls
- **Desktop Notifications**: Alerts when content is blocked

## Installation

### Quick Install (Recommended)

Download the installer for your platform from the [Releases](../../releases) page:

| Platform | Installer |
|----------|-----------|
| Windows | `aegis-x.x.x.msi` |
| macOS | `aegis-x.x.x.dmg` |
| Linux | `aegis-x.x.x.deb` / `.rpm` |

Run the installer and follow the first-run setup wizard.

### Build from Source

```bash
# Clone the repository
git clone https://github.com/your-org/aegis.git
cd aegis

# Build
cargo build --release

# Run
./target/release/aegis
```

## Interception Modes

### Browser Extension Mode

Only intercepts browser traffic. Simpler setup, no certificate installation required.

1. Install the Aegis Chrome extension from the Chrome Web Store
2. The extension communicates with the local Aegis service

### MITM Proxy Mode

Intercepts all application traffic (browsers, desktop apps, CLI tools).

Requires:
1. CA certificate installation (for HTTPS interception)
2. System proxy configuration

The setup wizard handles this automatically, but see below for manual setup.

---

## Manual CA Certificate Installation

The Aegis CA certificate is located at:
- **Windows**: `%APPDATA%\aegis\Aegis\data\ca\aegis-ca.crt`
- **macOS**: `~/Library/Application Support/com.aegis.Aegis/data/ca/aegis-ca.crt`
- **Linux**: `~/.local/share/aegis/Aegis/data/ca/aegis-ca.crt`

### Windows

#### Install CA Certificate

**Option 1: Machine Store (all users, requires admin)**
```powershell
# Run PowerShell as Administrator
certutil -addstore Root "%APPDATA%\aegis\Aegis\data\ca\aegis-ca.crt"
```

**Option 2: User Store (current user only)**
```powershell
certutil -addstore -user Root "%APPDATA%\aegis\Aegis\data\ca\aegis-ca.crt"
```

**Option 3: GUI Method**
1. Double-click the `aegis-ca.crt` file
2. Click "Install Certificate..."
3. Select "Local Machine" (all users) or "Current User"
4. Select "Place all certificates in the following store"
5. Click "Browse" and select "Trusted Root Certification Authorities"
6. Click "Next" then "Finish"

#### Uninstall CA Certificate

**Command Line:**
```powershell
# Machine store (requires admin)
certutil -delstore Root "Aegis Root CA"

# User store
certutil -delstore -user Root "Aegis Root CA"
```

**GUI Method:**
1. Press `Win+R`, type `certmgr.msc`, press Enter
2. Navigate to "Trusted Root Certification Authorities" > "Certificates"
3. Find "Aegis Root CA", right-click and delete

### macOS

#### Install CA Certificate

**Option 1: System Keychain (all users, requires admin password)**
```bash
sudo security add-trusted-cert -d -r trustRoot \
  -k /Library/Keychains/System.keychain \
  ~/Library/Application\ Support/com.aegis.Aegis/data/ca/aegis-ca.crt
```

**Option 2: User Keychain (current user only)**
```bash
security add-trusted-cert -r trustRoot \
  -k ~/Library/Keychains/login.keychain-db \
  ~/Library/Application\ Support/com.aegis.Aegis/data/ca/aegis-ca.crt
```

**GUI Method:**
1. Double-click the `aegis-ca.crt` file (opens Keychain Access)
2. Add to "System" keychain (all users) or "login" keychain (current user)
3. Find the certificate, double-click it
4. Expand "Trust" section
5. Set "When using this certificate" to "Always Trust"
6. Close and enter your password

#### Uninstall CA Certificate

**Command Line:**
```bash
# System keychain (requires admin)
sudo security delete-certificate -c "Aegis Root CA" /Library/Keychains/System.keychain

# User keychain
security delete-certificate -c "Aegis Root CA" ~/Library/Keychains/login.keychain-db
```

**GUI Method:**
1. Open "Keychain Access" (Applications > Utilities)
2. Search for "Aegis Root CA"
3. Right-click and delete

### Linux

#### Install CA Certificate

**Debian/Ubuntu:**
```bash
sudo cp ~/.local/share/aegis/Aegis/data/ca/aegis-ca.crt \
  /usr/local/share/ca-certificates/aegis-ca.crt
sudo update-ca-certificates
```

**Fedora/RHEL/CentOS:**
```bash
sudo cp ~/.local/share/aegis/Aegis/data/ca/aegis-ca.crt \
  /etc/pki/ca-trust/source/anchors/aegis-ca.crt
sudo update-ca-trust extract
```

**Arch Linux:**
```bash
sudo cp ~/.local/share/aegis/Aegis/data/ca/aegis-ca.crt \
  /etc/ca-certificates/trust-source/anchors/aegis-ca.crt
sudo trust extract-compat
```

#### Uninstall CA Certificate

**Debian/Ubuntu:**
```bash
sudo rm /usr/local/share/ca-certificates/aegis-ca.crt
sudo update-ca-certificates --fresh
```

**Fedora/RHEL/CentOS:**
```bash
sudo rm /etc/pki/ca-trust/source/anchors/aegis-ca.crt
sudo update-ca-trust extract
```

**Arch Linux:**
```bash
sudo rm /etc/ca-certificates/trust-source/anchors/aegis-ca.crt
sudo trust extract-compat
```

### Firefox (All Platforms)

Firefox uses its own certificate store. To add the CA:

1. Open Firefox Settings > Privacy & Security
2. Scroll to "Certificates" and click "View Certificates..."
3. Go to "Authorities" tab
4. Click "Import..." and select `aegis-ca.crt`
5. Check "Trust this CA to identify websites"
6. Click OK

To remove: Find "Aegis Root CA" in the list and click "Delete or Distrust..."

---

## Manual Proxy Configuration

Aegis proxy listens on `127.0.0.1:8766` by default.

### Windows

#### Enable System Proxy

**PowerShell:**
```powershell
# Enable proxy
$regPath = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Internet Settings"
Set-ItemProperty -Path $regPath -Name ProxyEnable -Value 1
Set-ItemProperty -Path $regPath -Name ProxyServer -Value "127.0.0.1:8766"
Set-ItemProperty -Path $regPath -Name ProxyOverride -Value "<local>"
```

**GUI Method:**
1. Open Settings > Network & Internet > Proxy
2. Under "Manual proxy setup", turn on "Use a proxy server"
3. Address: `127.0.0.1`
4. Port: `8766`
5. Check "Don't use the proxy server for local addresses"
6. Click Save

#### Disable System Proxy

**PowerShell:**
```powershell
$regPath = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Internet Settings"
Set-ItemProperty -Path $regPath -Name ProxyEnable -Value 0
```

**GUI Method:**
1. Open Settings > Network & Internet > Proxy
2. Turn off "Use a proxy server"

### macOS

#### Enable System Proxy

```bash
# Find your network service (usually "Wi-Fi" or "Ethernet")
networksetup -listallnetworkservices

# Enable HTTP proxy
networksetup -setwebproxy "Wi-Fi" 127.0.0.1 8766
networksetup -setwebproxystate "Wi-Fi" on

# Enable HTTPS proxy
networksetup -setsecurewebproxy "Wi-Fi" 127.0.0.1 8766
networksetup -setsecurewebproxystate "Wi-Fi" on
```

**GUI Method:**
1. Open System Preferences > Network
2. Select your network (Wi-Fi or Ethernet)
3. Click "Advanced..." > "Proxies" tab
4. Check "Web Proxy (HTTP)" and "Secure Web Proxy (HTTPS)"
5. Set server to `127.0.0.1` and port to `8766`
6. Click OK, then Apply

#### Disable System Proxy

```bash
networksetup -setwebproxystate "Wi-Fi" off
networksetup -setsecurewebproxystate "Wi-Fi" off
```

**GUI Method:**
1. Open System Preferences > Network
2. Select your network
3. Click "Advanced..." > "Proxies" tab
4. Uncheck "Web Proxy (HTTP)" and "Secure Web Proxy (HTTPS)"
5. Click OK, then Apply

### Linux

#### Enable System Proxy (GNOME)

```bash
gsettings set org.gnome.system.proxy mode 'manual'
gsettings set org.gnome.system.proxy.http host '127.0.0.1'
gsettings set org.gnome.system.proxy.http port 8766
gsettings set org.gnome.system.proxy.https host '127.0.0.1'
gsettings set org.gnome.system.proxy.https port 8766
```

**GUI Method (GNOME):**
1. Open Settings > Network > Network Proxy
2. Select "Manual"
3. Set HTTP and HTTPS Proxy to `127.0.0.1` port `8766`

#### Enable System Proxy (KDE)

1. Open System Settings > Network Settings > Proxy
2. Select "Use manually specified proxy configuration"
3. Set HTTP and HTTPS proxy to `127.0.0.1:8766`

#### Enable System Proxy (Environment Variables)

Add to `~/.bashrc` or `/etc/environment`:
```bash
export http_proxy="http://127.0.0.1:8766"
export https_proxy="http://127.0.0.1:8766"
export HTTP_PROXY="http://127.0.0.1:8766"
export HTTPS_PROXY="http://127.0.0.1:8766"
export no_proxy="localhost,127.0.0.1"
```

#### Disable System Proxy (GNOME)

```bash
gsettings set org.gnome.system.proxy mode 'none'
```

#### Disable System Proxy (Environment Variables)

Remove or comment out the proxy lines from `~/.bashrc` or `/etc/environment`.

---

## Troubleshooting

### Certificate Not Trusted

- Ensure the CA certificate is installed in the correct store
- Restart your browser after installing the certificate
- Firefox requires separate certificate installation (see above)

### Proxy Not Working

- Verify Aegis is running (check system tray)
- Ensure proxy is configured to `127.0.0.1:8766`
- Some applications ignore system proxy settings and need manual configuration

### Blocked Content Not Logged

- Check that the database path is writable
- Logs are stored in `%APPDATA%\aegis\aegis\data\aegis.db` (Windows) or equivalent

---

## Development

See `CLAUDE.md` for development workflow.

```bash
# Build
cargo build

# Run tests
cargo test

# Lint
cargo clippy -- -D warnings

# Format
cargo fmt

# Run in debug mode (shows console)
cargo run -- --debug --no-tray
```

## Architecture

```
aegis-core     - Classification, rules, profiles, auth
aegis-proxy    - MITM proxy, TLS, interception
aegis-server   - HTTP API for extension
aegis-storage  - SQLite persistence
aegis-ui       - Parent dashboard (egui)
aegis-tray     - System tray
aegis-app      - Main binary
```

## License

MIT
