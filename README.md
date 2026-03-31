# Cron Drop

Cron Drop is a desktop eye-drop reminder app with a CLI-first workflow.

It lets you define a reminder schedule from the terminal, keeps a background daemon running quietly, and shows a native popup when it is time to use your eye drops. A tray process provides quick status and control without reopening the terminal.

## What It Does

- Shows popup reminders at configured intervals or fixed times
- Runs in the background after you set a schedule
- Supports a system tray for quick control
- Lets you pause reminders for the rest of the day
- Supports launch-at-login setup
- Keeps configuration in a local config file you can inspect

## How It Works

Cron Drop has three main parts:

- `crondrop` CLI for setup and control
- a background daemon that tracks the next reminder
- a native popup UI for the actual reminder window

Typical flow:

1. Initialize the config once.
2. Set a schedule.
3. Cron Drop starts in the background.
4. A popup appears when the next reminder is due.
5. Use the tray or CLI to check status, pause, resume, or stop it.

## Installation

## Homebrew

The easiest install path on macOS or Linux is Homebrew:

```bash
brew install --formula https://raw.githubusercontent.com/AlTosterino/crondop/main/Formula/crondrop.rb
```

Then verify:

```bash
crondrop --help
```

The formula installs from the published GitHub release archives for the current version.

If you want the cleaner tap flow:

```bash
brew tap AlTosterino/crondrop
brew install crondrop
```

create a separate repository named `AlTosterino/homebrew-crondrop`.

This repository already contains:

- a tap-ready formula generator in [`scripts/render-homebrew-formula.sh`](./scripts/render-homebrew-formula.sh)
- a tap repo updater in [`scripts/update-homebrew-tap.sh`](./scripts/update-homebrew-tap.sh)
- a release workflow that can update the tap automatically after a tagged release

To enable automatic tap updates from GitHub Actions, set these repository secrets in `AlTosterino/crondop`:

- `HOMEBREW_TAP_REPOSITORY` with value `AlTosterino/homebrew-crondrop`
- `HOMEBREW_TAP_GITHUB_TOKEN` with a token that can push to that tap repo

## Prerequisites

- Rust and Cargo
- A desktop session if you want popup windows and tray support

Check your toolchain:

```bash
rustc --version
cargo --version
```

## Build From Source

Clone the repo and build it:

```bash
cargo build --release
```

The release binary will be available at:

```bash
target/release/crondrop
```

## Install the CLI

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

## Quick Start

Initialize the app:

```bash
crondrop init
```

Set a repeating schedule every hour between 08:00 and 22:00:

```bash
crondrop schedule every 1h --from 08:00 --to 22:00
```

Or use fixed reminder times:

```bash
crondrop schedule add --at 09:00 --at 13:00 --at 18:00
```

By default, setting a schedule also starts Cron Drop in the background.

## CLI Usage

## Core Commands

Initialize config:

```bash
crondrop init
```

Start the daemon:

```bash
crondrop start
```

`run` is an alias for `start`:

```bash
crondrop run
```

Stop the daemon:

```bash
crondrop stop
```

Restart the daemon:

```bash
crondrop restart
```

Show current status:

```bash
crondrop status
```

## Schedule Commands

Repeat every hour:

```bash
crondrop schedule every 1h
```

Repeat every 30 minutes during a time window:

```bash
crondrop schedule every 30m --from 08:00 --to 22:00
```

Use weekdays only:

```bash
crondrop schedule every 1h --from 08:00 --to 18:00 --weekdays-only
```

Set fixed reminder times:

```bash
crondrop schedule add --at 09:00 --at 13:00 --at 18:00
```

Save the schedule without starting the app:

```bash
crondrop schedule every 1h --no-start
```

## Popup And Tray

Open the popup immediately to preview the reminder UI:

```bash
crondrop preview
```

The following aliases also work:

```bash
crondrop popup
crondrop show-popup
crondrop test
```

Start the tray manually:

```bash
crondrop tray
```

## Pause And Resume

Pause reminders for the rest of today:

```bash
crondrop pause --today
```

Resume reminders:

```bash
crondrop resume
```

## Config And Theme

Show the active config:

```bash
crondrop config show
```

Print the config file path:

```bash
crondrop config path
```

Change the popup theme:

```bash
crondrop theme cozy
```

## Autostart

Install launch-at-login:

```bash
crondrop autostart install
```

Check autostart status:

```bash
crondrop autostart status
```

Remove launch-at-login:

```bash
crondrop autostart remove
```

## Development

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

Run tests:

```bash
cargo test --workspace
```

## Packaging And CI

- Branch pushes and pull requests run formatting and tests through GitHub Actions
- Tagged releases build Linux, macOS, and Windows artifacts
- Packaging assets live under [`packaging/`](./packaging)

## Status

Cron Drop is currently source-first. The repository builds a working Rust binary, and the packaging layer is oriented around future platform-native distribution.
