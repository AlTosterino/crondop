# Packaging

This directory contains packaging-oriented assets for `crondrop`.

## Targets

- Linux desktop autostart entry: `linux/crondrop.desktop`
- macOS LaunchAgent template: `macos/com.crondrop.plist`
- Windows startup script template: `windows/crondrop.cmd`

## Build Notes

- Linux: `cargo build --release`
- macOS: `cargo build --release`
- Windows: `cargo build --release --target x86_64-pc-windows-msvc`

The current repository builds a single Rust binary named `crondrop`. A future packaging pass can wrap this in platform-native installers.

## GitHub Actions

The repository includes a GitHub Actions pipeline in `.github/workflows/ci.yml`.

Behavior:

- pull requests and branch pushes run `cargo fmt --check` and `cargo test --workspace`
- release builds are produced on Linux, macOS, and Windows
- version tags matching `v*` publish packaged artifacts to a GitHub release

Packaging scripts:

- Unix: `scripts/package-release.sh`
- Windows: `scripts/package-release.ps1`
