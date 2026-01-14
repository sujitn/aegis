# F024: Chrome Web Store Deployment

| Status | Priority | Crate |
|--------|----------|-------|
| `ready` | high | extension |

## Description

Deploy the Aegis browser extension to Chrome Web Store for easy one-click installation by end users. This eliminates the need for manual "Load unpacked" installation and enables automatic updates.

## Dependencies

- **Requires**: F010 (Browser Extension)
- **Blocks**: None

## Acceptance Criteria

### Chrome Web Store Submission

- [ ] Chrome Web Store developer account created ($5 registration)
- [ ] Extension manifest includes all required fields
- [ ] Privacy policy document created and hosted
- [ ] Store listing assets prepared:
  - [ ] 128x128 icon (store listing)
  - [ ] 440x280 small promo tile
  - [ ] 1280x800 large promo tile (optional)
  - [ ] 640x400 marquee tile (optional)
  - [ ] Screenshots (1280x800 or 640x400)
- [ ] Detailed description written (supports markdown)
- [ ] Category selected (Productivity or Family)
- [ ] Extension submitted for review
- [ ] Extension approved and published

### App Integration

- [ ] Setup wizard updated to open Chrome Web Store link
- [ ] Settings page updated to open Chrome Web Store link
- [ ] Extension ID stored in app configuration
- [ ] Fallback to manual install if store unavailable

### Store Listing Content

**Title:** Aegis AI Safety

**Short Description (132 chars max):**
Parental controls for AI chatbots. Protects children from harmful content on ChatGPT, Claude, and Gemini.

**Category:** Family

**Language:** English

## Implementation

### 1. Chrome Web Store Developer Account

1. Go to https://chrome.google.com/webstore/devconsole/
2. Pay $5 one-time registration fee
3. Verify email and account

### 2. Prepare Extension Package

```bash
cd extension
# Ensure dist is built
npm run build
# Create ZIP (exclude node_modules, src, etc.)
zip -r aegis-extension.zip manifest.json popup.html popup.css overlay.css icons/ dist/
```

### 3. Store Listing Assets

Required images:
- `store/icon-128.png` - 128x128 store icon
- `store/promo-small.png` - 440x280 promo tile
- `store/screenshot-1.png` - 1280x800 screenshot (popup)
- `store/screenshot-2.png` - 1280x800 screenshot (blocking overlay)
- `store/screenshot-3.png` - 1280x800 screenshot (dashboard integration)

### 4. Update App for Web Store Install

```rust
// In setup.rs and settings.rs
const EXTENSION_STORE_URL: &str =
    "https://chrome.google.com/webstore/detail/aegis-ai-safety/EXTENSION_ID";

fn install_from_store() -> std::io::Result<()> {
    open_url(EXTENSION_STORE_URL)
}
```

### 5. Submission Checklist

Before submitting:
- [ ] Test extension works correctly
- [ ] Verify all permissions are justified
- [ ] Privacy policy URL is accessible
- [ ] No console errors in extension
- [ ] Icons display correctly at all sizes

### 6. Review Process

- Initial review: 1-3 business days
- May request clarifications about:
  - localhost permission (explain: local app communication)
  - Content script injection (explain: prompt interception for safety)
- Resubmission if rejected: address feedback and resubmit

## Store Description (Full)

```markdown
# Aegis AI Safety - Parental Controls for AI Chatbots

Protect your children from harmful AI interactions with Aegis, the comprehensive
parental control solution for AI chatbots.

## Features

- **Real-time Protection**: Monitors prompts before they're sent to AI
- **Smart Filtering**: Uses ML-based classification to detect harmful content
- **Multiple AI Support**: Works with ChatGPT, Claude, and Gemini
- **Local Processing**: All analysis happens on your device - no cloud required
- **Parent Dashboard**: Review blocked content and adjust settings

## How It Works

1. Install the Aegis desktop app (required)
2. Add this extension to Chrome
3. Create child profiles with appropriate restrictions
4. Children use AI chatbots safely

## What Gets Blocked

- Violence and self-harm content
- Adult/explicit material
- Jailbreak attempts
- Other harmful categories (configurable)

## Privacy First

- All processing happens locally on your device
- No data sent to external servers
- Parents control what gets logged
- Full data export and deletion available

## Requirements

- Aegis desktop application (Windows/macOS/Linux)
- Google Chrome browser

## Support

- Documentation: https://github.com/anthropics/aegis
- Issues: https://github.com/anthropics/aegis/issues
```

## Notes

### Review Considerations

The extension uses `host_permissions` for localhost (127.0.0.1) which may raise questions during review. Justification:

> "The extension communicates with the Aegis desktop application running locally
> on the user's machine. This local-only approach ensures all data processing
> happens on-device without sending any user data to external servers, providing
> maximum privacy protection."

### Future Enhancements

- Firefox Add-ons support (F025)
- Edge Add-ons support (automatic via Chrome Web Store)
- Safari extension (requires separate implementation)

### Versioning

- Follow semantic versioning (1.0.0, 1.0.1, 1.1.0, etc.)
- Increment version in manifest.json before each submission
- Keep changelog of extension updates
