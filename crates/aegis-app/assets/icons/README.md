# Aegis Application Icons

This directory contains the application icons generated from the Aegis shield logo.

## Generated Files

### Source
- `aegis-logo.svg` - Original vector logo (shield with protective hands)

### Application Icons
- `icon.ico` - Windows icon file (16, 24, 32, 48, 64, 128, 256px)
- `icon.icns` - macOS icon file (16-1024px with @2x retina variants)
- `icon-16.png` through `icon-1024.png` - PNG icons at various sizes

### System Tray Icons
Located in the `tray/` subdirectory:
- `tray-{size}.png` - Base tray icons
- `tray-protected-{size}.png` - Green status dot (protection active)
- `tray-paused-{size}.png` - Yellow status dot (protection paused)
- `tray-error-{size}.png` - Red status dot (service error)

## Regenerating Icons

To regenerate icons from the SVG source:

```bash
# From the workspace root
cargo run -p aegis-icon-gen --release -- --verbose
```

This will:
1. Generate PNG icons at all required sizes
2. Create the Windows .ico file
3. Create the macOS .icns file (with @2x retina variants)
4. Generate tray icons with status indicators

## Icon Design

The Aegis icon features:
- A **shield** representing protection and safety
- **Hands** cradling the shield, symbolizing care and parental protection
- **Blue gradient** colors conveying trust and technology
- **Status indicators** (small colored dots) for tray icons showing:
  - Green = Protected (filtering active)
  - Yellow = Paused (temporarily disabled)
  - Red = Error (service issue)

## Platform Usage

| Platform | Files Used |
|----------|------------|
| Windows | `icon.ico` (app), `tray-*.png` (tray) |
| macOS | `icon.icns` (app), `tray-*.png` (tray) |
| Linux | `icon-*.png` (app), `tray-*.png` (tray) |
