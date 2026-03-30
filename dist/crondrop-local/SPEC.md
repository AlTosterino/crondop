# crondrop

## Goal

`crondrop` is a cross-platform desktop reminder application for eye drops.

It is designed around two constraints:

- configuration must happen through a CLI
- the reminder popup must feel warm, calm, polished, and user-friendly

The application must run on Linux, macOS, and Windows.

## Product Summary

`crondrop` is a Rust application with:

- a CLI for setup and day-to-day control
- a background process that evaluates reminder schedules
- a fully Rust-native popup window for reminders
- optional tray integration for quick actions and status

The primary use case is simple:

- the user wants a reminder every hour to put in eye drops

Example:

```bash
crondrop init
crondrop schedule every 1h --from 08:00 --to 22:00
crondrop theme cozy
crondrop start
```

## Non-Negotiable Constraints

- language: Rust
- popup implementation: Rust-native, no JavaScript
- configuration surface: CLI-first
- supported platforms: Linux, macOS, Windows
- UX direction: cosy, calm, low-friction, visually polished

## Recommended Stack

### Core Language

- Rust

### CLI

- `clap` for command parsing

### Config and Serialization

- `serde`
- `toml`

### Time and Scheduling

- `time` or `chrono`

### Logging and Diagnostics

- `tracing`
- `tracing-subscriber`

### Popup and Desktop UI

- `iced`

This is the key design decision. The popup should stay fully in Rust, and `iced` is the best fit for a polished custom UI without bringing in JavaScript or a browser runtime. It supports native desktop applications and gives enough control to build a soft, visually intentional reminder window.

### Background Process / Single-App Coordination

- a single Rust binary with subcommands
- background mode for the scheduler
- local IPC for CLI-to-daemon control, likely domain sockets on Unix and named pipes on Windows

### Optional Platform Helpers

- tray/menu bar support via a Rust system tray crate if stable enough for the chosen UI/runtime combination
- launch-at-login support through platform-specific setup handled by Rust

## Why `iced`

The popup needs to be "beautiful" and also fully Rust-native.

That rules out:

- Tauri, because it introduces web tech and JavaScript
- Electron, for the same reason
- most browser-based wrappers

Why `iced` is a strong fit:

- pure Rust UI
- suitable for custom-styled popup windows
- enough layout and drawing flexibility for a warm, branded reminder card
- cross-platform target support

Alternative worth keeping in reserve:

- `egui` via `eframe`

`egui` is faster to iterate with, but `iced` is the better default choice if visual polish and deliberate styling matter more than raw implementation speed.

## Product Architecture

`crondrop` should be implemented as one Rust workspace with multiple crates or modules.

Suggested structure:

```text
crondrop/
  crates/
    crondrop-cli/
    crondrop-core/
    crondrop-daemon/
    crondrop-ui/
    crondrop-platform/
  SPEC.md
  Cargo.toml
```

### `crondrop-core`

Responsibilities:

- config schema
- schedule model
- reminder state machine
- time calculations
- persistence models

This crate should contain the logic that can be tested without any UI.

### `crondrop-cli`

Responsibilities:

- parse commands
- validate user input
- read/write config
- communicate with daemon
- trigger popup tests

### `crondrop-daemon`

Responsibilities:

- run in the background
- load config
- evaluate schedules
- open popup when reminder is due
- manage snooze, skip, pause, done actions

### `crondrop-ui`

Responsibilities:

- reminder popup window
- cosy theme system
- button actions and event handling
- optional settings preview window later

### `crondrop-platform`

Responsibilities:

- OS-specific startup behavior
- autostart integration
- tray integration
- foreground window behavior
- platform notification fallback if ever needed

## Primary User Experience

The app should not feel clinical. It should feel gentle and supportive.

### Reminder Window Behavior

When a reminder is due:

- open a small focused popup window
- keep it prominent without being aggressive
- show warm colors and clear typography
- present a single strong primary action

Recommended actions:

- `Done`
- `Snooze 10m`
- `Skip`
- `Pause today`

### Visual Direction

The popup should aim for:

- warm neutral background tones
- soft green or blue accents
- rounded corners
- subtle shadows
- gentle motion on appearance
- a simple illustration or icon motif for the eye drop reminder
- large readable text

### Tone of Copy

Copy should be short and calm.

Examples:

- `Time for your eye drops`
- `A short pause now helps later`
- `You can snooze if this is a bad moment`

### Accessibility

The popup should support:

- keyboard navigation
- high-contrast theme variant later
- clear focus state
- large enough hit targets
- reduced-motion option later

## CLI Design

The CLI is the source of truth for configuration.

Suggested commands:

```bash
crondrop init
crondrop schedule every 1h
crondrop schedule every 1h --from 08:00 --to 22:00
crondrop schedule add --at 09:00 --at 13:00 --at 18:00
crondrop theme cozy
crondrop start
crondrop stop
crondrop restart
crondrop status
crondrop pause --today
crondrop resume
crondrop test-popup
crondrop config show
crondrop config path
```

### CLI Principles

- all important behavior must be configurable without opening a settings window
- commands should be explicit and scriptable
- output should be human-readable by default
- a machine-readable mode can be added later with `--json`

## Configuration Model

Store configuration in a user-level TOML file in the appropriate platform config directory.

Examples:

- Linux: XDG config directory
- macOS: `~/Library/Application Support/crondrop`
- Windows: `%AppData%\\crondrop`

Example config:

```toml
[schedule]
mode = "interval"
every_minutes = 60
active_from = "08:00"
active_to = "22:00"
weekdays_only = false

[ui]
theme = "cozy"
always_on_top = true
animation = "gentle"

[popup]
title = "Time for your eye drops"
body = "Take a short pause and put them in."
show_snooze = true
snooze_minutes = 10
show_pause_today = true

[behavior]
start_on_login = false
minimize_to_tray = true
sound = true
```

## Scheduling Model

Version 1 should support:

- repeating interval reminders
- optional active time window
- snooze
- skip
- pause for the rest of the day

This directly solves the target use case:

- reminder every hour to put in eye drops

### Reminder State Machine

Each due reminder can move through states such as:

- scheduled
- due
- shown
- completed
- snoozed
- skipped
- paused

The daemon should keep this simple and deterministic.

## Cross-Platform Strategy

### Linux

Main risk:

- desktop environment fragmentation

Approach:

- use the custom Rust popup window as the primary UX
- avoid relying on desktop-native notifications for the main reminder experience

### macOS

Main concerns:

- app activation and focus behavior
- menu bar integration
- launch-at-login setup

Approach:

- package as a normal app bundle
- support a menu bar presence
- ensure popup behavior feels native and non-disruptive

### Windows

Main concerns:

- tray behavior
- installer packaging
- popup focus and z-order

Approach:

- provide a standard installer
- support system tray actions
- ensure reminder window is visible without feeling invasive

## Delivery Approach

Build this in phases.

### Phase 1: Core Spec and CLI Skeleton

Deliverables:

- Rust workspace
- config schema
- CLI command layout
- config read/write
- basic schedule parser

Success criteria:

- user can define an hourly schedule from CLI
- config is persisted correctly

### Phase 2: Daemon and Scheduler

Deliverables:

- background process mode
- schedule evaluation loop
- due reminder generation
- daemon status reporting

Success criteria:

- reminders trigger reliably on time
- restart behavior is predictable

### Phase 3: Rust-Native Popup

Deliverables:

- `iced` reminder popup
- cosy theme system
- primary and secondary actions
- polished window sizing and placement

Success criteria:

- popup looks intentionally designed
- action flow works end to end

### Phase 4: Tray and Lifecycle

Deliverables:

- tray/menu bar presence
- quick actions from tray
- start/stop/pause integration
- launch-at-login support

Success criteria:

- app can live quietly in the background
- user has lightweight control without using the CLI every time

### Phase 5: Packaging

Deliverables:

- Linux package strategy
- macOS app bundle
- Windows installer

Success criteria:

- clean install and launch on all three platforms

### Phase 6: Hardening

Deliverables:

- tests for schedule logic
- edge-case handling around sleep/resume
- timezone correctness
- accessibility pass

Success criteria:

- stable reminder behavior across common real-world conditions

## Testing Strategy

### Unit Tests

Cover:

- interval schedule calculations
- active-hours filtering
- snooze behavior
- pause-for-today behavior
- config parsing and validation

### Integration Tests

Cover:

- CLI writing config
- daemon reading config
- reminder state transitions

### Manual Cross-Platform Validation

Verify:

- popup visibility
- correct window focus behavior
- tray behavior
- autostart
- packaging/install flow

## Major Risks

### 1. Native UI Polish in Rust

Risk:

- fully Rust-native UI is feasible, but high visual polish takes more work than web-based UI

Mitigation:

- keep popup scope narrow
- design one excellent reminder card instead of a large settings UI

### 2. Cross-Platform Window Behavior

Risk:

- popup positioning, focus, and always-on-top behavior vary by OS

Mitigation:

- isolate platform-specific behavior in `crondrop-platform`
- test window behavior early, before expanding feature scope

### 3. Background Process Semantics

Risk:

- service lifecycle differs between Linux, macOS, and Windows

Mitigation:

- start with a user-launched background mode
- add deeper OS integration after the core flow is stable

## MVP Definition

The MVP should include only what is needed to be genuinely useful:

- CLI config
- hourly or interval reminders
- active-hours range
- daemon/background process
- cosy Rust-native popup
- actions: done, snooze, skip, pause today
- persistent config

The MVP should not include:

- sync
- cloud storage
- user accounts
- analytics
- mobile app
- complicated multi-medication workflows

## Recommended First Milestone

Build the smallest complete loop:

1. `crondrop init`
2. `crondrop schedule every 1h --from 08:00 --to 22:00`
3. `crondrop start`
4. daemon triggers a basic `iced` popup
5. user clicks `Done` or `Snooze`

If this loop works well, the rest of the application becomes straightforward.

## Final Recommendation

Build `crondrop` as a fully Rust-native desktop utility:

- Rust for everything
- `clap` for CLI
- `serde` + `toml` for config
- `iced` for the popup UI
- one background daemon process for scheduling

This is the correct approach if the priorities are:

- no JavaScript
- cross-platform support
- CLI-first configuration
- a popup that feels warm, polished, and intentional
