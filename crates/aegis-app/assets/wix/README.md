# WiX Installer Assets

This directory contains assets for the Windows MSI installer built with WiX Toolset.

## Required Files

### banner.bmp
- Size: 493x58 pixels
- Used at the top of installer wizard pages
- Should include Aegis logo/branding on the left

### dialog.bmp
- Size: 493x312 pixels
- Used as background on welcome and completion pages
- Should include Aegis branding

## Creating the BMPs

### Using ImageMagick

```bash
# Create banner (493x58)
convert -size 493x58 gradient:#4A90D9-#2E5A8C \
  -fill white -gravity west -pointsize 24 \
  -annotate +20+0 'Aegis AI Safety' banner.bmp

# Create dialog (493x312)
convert -size 493x312 gradient:#4A90D9-#2E5A8C \
  -fill white -gravity northwest -pointsize 36 \
  -annotate +30+30 'Aegis\nAI Safety' dialog.bmp
```

### Using any image editor
1. Create images with exact dimensions
2. Save as 24-bit BMP format
3. Use brand colors (recommended: #4A90D9 blue)

## WiX Build Process

The MSI is built using cargo-wix:

```bash
# Install cargo-wix
cargo install cargo-wix

# Initialize WiX configuration (creates main.wxs)
cargo wix init --package aegis-app

# Build the MSI installer
cargo wix --package aegis-app
```

The generated MSI will be in `target/wix/`.
