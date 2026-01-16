# Aegis Application Icons

This directory contains the application icons generated from the Aegis shield logo.

## Generated Files

### Source
- `aegis-logo.svg` - Original vector logo (shield with protective hands)

### Application Icons
- `icon.ico` - Windows icon file (16, 24, 32, 48, 64, 128, 256px)
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
3. Generate tray icons with status indicators

## macOS .icns Generation

On macOS, after running the icon generator, create `icon.icns`:

```bash
cd crates/aegis-app/assets/icons
mkdir -p icon.iconset
sips -z 16 16 icon-1024.png --out icon.iconset/icon_16x16.png
sips -z 32 32 icon-1024.png --out icon.iconset/icon_16x16@2x.png
sips -z 32 32 icon-1024.png --out icon.iconset/icon_32x32.png
sips -z 64 64 icon-1024.png --out icon.iconset/icon_32x32@2x.png
sips -z 128 128 icon-1024.png --out icon.iconset/icon_128x128.png
sips -z 256 256 icon-1024.png --out icon.iconset/icon_128x128@2x.png
sips -z 256 256 icon-1024.png --out icon.iconset/icon_256x256.png
sips -z 512 512 icon-1024.png --out icon.iconset/icon_256x256@2x.png
sips -z 512 512 icon-1024.png --out icon.iconset/icon_512x512.png
sips -z 1024 1024 icon-1024.png --out icon.iconset/icon_512x512@2x.png
iconutil -c icns icon.iconset -o icon.icns
rm -rf icon.iconset
```

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
