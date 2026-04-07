# Development

This page collects local setup, source builds, and maintainer-oriented notes that do not belong on the customer-facing README.

## Prerequisites

- Rust and Cargo
- A desktop session if you want popup windows and tray support

Check your toolchain:

```bash
rustc --version
cargo --version
```

## Build From Source

Build the project:

```bash
cargo build --release
```

The release binary will be available at:

```bash
target/release/crondrop
```

## Install The CLI From Source

Install from the local source tree:

```bash
cargo install --path crates/crondrop-cli
```

Then run:

```bash
crondrop --help
```

If you do not want to install it globally, you can run it directly from the repo with:

```bash
cargo run -p crondrop -- --help
```

## Local Development Runtime

To avoid touching your real user config during local development, use repo-scoped config and runtime directories:

```bash
export CRONDROP_CONFIG_DIR="$PWD/.crondrop-dev/config"
export CRONDROP_RUNTIME_DIR="$PWD/.crondrop-dev/runtime"
mkdir -p "$CRONDROP_CONFIG_DIR" "$CRONDROP_RUNTIME_DIR"
```

Then run commands from the repository, for example:

```bash
cargo run -p crondrop -- init
cargo run -p crondrop -- schedule every 1h --from 08:00 --to 22:00
cargo run -p crondrop -- status
```

## Tests

Run the full workspace tests:

```bash
cargo test --workspace
```

## Packaging And CI

- Branch pushes and pull requests run formatting and tests through GitHub Actions
- Tagged releases build Linux, macOS, and Windows artifacts
- Packaging assets live under [`packaging/`](../packaging)
