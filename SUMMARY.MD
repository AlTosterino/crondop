# Cron Drop Summary

## Purpose

`crondrop` is a Rust desktop reminder app for eye drops.

Core product shape:
- CLI-first configuration and control
- background daemon for scheduling
- native Rust popup UI with `iced`
- system tray/menu bar control surface
- cross-platform target: macOS, Linux, Windows

The user intent has been to make the tray the primary control point and to make the popup feel soft, minimal, and friendly.

## Workspace Layout

Rust workspace crates:
- `crates/crondrop-cli`
- `crates/crondrop-core`
- `crates/crondrop-daemon`
- `crates/crondrop-ui`
- `crates/crondrop-platform`

Other important paths:
- `README.md`
- `SPEC.md`
- `.github/workflows/ci.yml`
- `assets/sounds/water-drop.mp3`
- `assets/ATTRIBUTION.md`

## Crate Responsibilities

### `crondrop-cli`

Entry point and user-facing commands.

Implemented commands:
- `init`
- `schedule every ...`
- `schedule add --at ...`
- `theme`
- `start`
- `run`
- `stop`
- `restart`
- `status`
- `pause --today`
- `resume`
- `test-popup`
- aliases: `popup`, `preview`, `show-popup`, `test`
- `config show`
- `config path`
- `autostart install`
- `autostart remove`
- `autostart status`
- `tray`

Hidden internal commands:
- `__daemon-run`
- `__popup`
- `__tray-run`

Important CLI behavior:
- `schedule ...` can auto-start the daemon
- `tray` now launches the detached tray helper instead of owning the tray loop in the foreground
- stale tray PID state is cleaned up automatically

### `crondrop-core`

Shared domain logic:
- config structs and TOML serialization
- app path helpers
- schedule calculations

Important schedule behavior:
- `ScheduleMode::Interval`
- `ScheduleMode::FixedTimes`
- interval scheduling is now anchored to the reminder cycle start instead of snapping to wall-clock minute boundaries
- helper exported: `next_reminder_after_with_anchor`

This change was necessary because the user expected `every 1m` to mean “60 seconds from start / done / snooze”, not “next minute boundary”.

### `crondrop-daemon`

Background scheduler and persisted runtime state.

Current persisted state includes:
- `running`
- `paused_until`
- `snoozed_until`
- `active_reminder_id`
- `next_due_at`
- `cycle_started_at`
- `last_action`
- `daemon_pid`
- `tray_pid`
- `last_popup_at`

Control channel:
- file-based command inbox under runtime dir

Important daemon behavior:
- spawns popup when reminder becomes due
- updates state when popup actions are received
- preserves snooze across pause/resume
- tray quit is treated as scheduler shutdown

Important fixes made:
- daemon start resets stale cycle state so the tray does not begin already full from old runs
- pause/resume no longer destroys snooze timing
- interval next-due computation uses `cycle_started_at`

### `crondrop-ui`

Native popup built with `iced`.

Current UI direction:
- soft near-white background
- warm muted text colors
- minimal structure
- primary actions: `Done`, `Snooze`
- secondary text actions: `Skip`, `Pause today`

Popup behavior:
- sends actions back to daemon
- opens via `preview` / `popup` / `test-popup`
- plays a real water-drop audio clip when the popup opens if sound is enabled

Sound implementation:
- embedded MP3 asset via `include_bytes!`
- playback through `rodio`

### `crondrop-platform`

Platform integration:
- tray runtime
- autostart file generation
- process liveness checks

Tray behavior:
- uses a real event-loop based runtime
- left click opens the menu
- tray quit stops the daemon
- menu shows remaining time until next drop
- pause/resume is now one dynamic toggle item
- icon is intended to fill over the reminder cycle

macOS-specific work:
- tray helper uses accessory activation policy
- stale tray PID handling improved

## Scheduling Model

Current intent:
- fresh start of `every 1m` should begin near `1m`
- tray countdown should update every second
- tray fill should go from empty to full across the current cycle
- `Done` should start a new interval cycle
- `Snooze 10m` should create a true 10-minute countdown
- pause/resume should not destroy snooze state

## Sound Asset

Embedded popup sound:
- `assets/sounds/water-drop.mp3`

Attribution:
- stored in `assets/ATTRIBUTION.md`
- source used: SoundBible “Water Drop” by Mike Koenig
- source page: https://soundbible.com/1126-Water-Drop.html
- download URL used: http://soundbible.com/grab.php?id=1126&type=mp3
- license shown on source page: Attribution 3.0

## Build / Validation Status

Verified repeatedly during development:
- `cargo check`
- `cargo test`

CI:
- GitHub Actions workflow exists in `.github/workflows/ci.yml`
- covers formatting / tests / release packaging

## Local Run Notes

Typical local flow:

```bash
cargo run -p crondrop -- init
cargo run -p crondrop -- schedule every 1m
cargo run -p crondrop -- preview
cargo run -p crondrop -- status
cargo run -p crondrop -- tray
```

Useful aliases:

```bash
cargo run -p crondrop -- preview
cargo run -p crondrop -- popup
cargo run -p crondrop -- show-popup
cargo run -p crondrop -- test
```

## Important User Expectations Captured

The user has been very clear about the following:
- popup should be beautiful, calm, minimal, and cosy
- popup and overall app must be fully Rust, no JavaScript popup stack
- tray/menu bar behavior on macOS matters a lot
- tray should be the source of truth
- quitting tray should stop scheduling
- pause/resume should behave like a toggle
- remaining time should refresh every second
- tray icon should represent droplet fill over time
- sound should be a real droplet/water sound, not synthetic if avoidable

## Known Risk / Remaining Validation Area

The largest area that still needs real-world runtime validation is tray icon fill behavior on macOS:
- code now carries `cycle_started_at`
- interval reminders are cycle-anchored
- tray refreshes every second

But the user has repeatedly tested visual runtime details that are hard to prove from unit tests alone. If something still feels off, the next debugging target should be live tray state observation on macOS rather than more speculative refactoring.
