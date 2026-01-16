# F021: Build & Release Pipeline

| Status | Priority | Crate |
|--------|----------|-------|
| `complete` | high | infrastructure |

## Description

CI/CD pipeline for cross-platform native binaries. GitHub Actions with automated releases.

## Dependencies

- **Requires**: F001
- **Blocks**: F022

## Acceptance Criteria

- [ ] GitHub Actions workflow
- [ ] Build matrix: macOS (x64, ARM), Windows (x64), Linux (x64)
- [ ] Automated tests on PR
- [ ] Release on tag push (v*)
- [ ] Sign binaries (macOS notarization, Windows signing)
- [ ] Generate checksums
- [ ] Upload artifacts to GitHub Releases
- [ ] Build browser extension (.zip, .crx)

## Build Targets

| OS | Arch | Target |
|----|------|--------|
| macOS | x64 | x86_64-apple-darwin |
| macOS | ARM | aarch64-apple-darwin |
| Windows | x64 | x86_64-pc-windows-msvc |
| Linux | x64 | x86_64-unknown-linux-gnu |

## Notes

Tools: cross (cross-compilation), cargo-bundle (app bundles), tauri-action (optional). Secrets: APPLE_CERTIFICATE, APPLE_ID, WINDOWS_CERTIFICATE.
