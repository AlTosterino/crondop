use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::Stdio;
use std::thread;
use std::time::Duration as StdDuration;

use anyhow::{Context, Result};
use chrono::{DateTime, Local, NaiveDate, TimeZone};
use crondrop_core::{
    ActionOutcome, AppConfig, ReminderAction, command_inbox_dir, ensure_app_dirs, load_config,
    next_reminder_after_with_anchor, state_file_path,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DaemonState {
    pub running: bool,
    pub paused_until: Option<String>,
    pub snoozed_until: Option<String>,
    pub active_reminder_id: Option<String>,
    pub next_due_at: Option<String>,
    pub cycle_started_at: Option<String>,
    pub last_action: Option<String>,
    pub daemon_pid: Option<u32>,
    pub tray_pid: Option<u32>,
    pub last_popup_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DaemonCommand {
    Stop,
    PauseToday,
    Resume,
    PopupAction {
        reminder_id: String,
        action: ReminderAction,
    },
}

impl DaemonState {
    pub fn is_paused_today(&self) -> bool {
        let today = Local::now().date_naive();
        self.paused_until
            .as_deref()
            .and_then(parse_date)
            .is_some_and(|date| date >= today)
    }

    pub fn snoozed_until_dt(&self) -> Option<DateTime<Local>> {
        self.snoozed_until.as_deref().and_then(parse_datetime)
    }

    pub fn next_due_dt(&self) -> Option<DateTime<Local>> {
        self.next_due_at.as_deref().and_then(parse_datetime)
    }

    pub fn cycle_started_dt(&self) -> Option<DateTime<Local>> {
        self.cycle_started_at.as_deref().and_then(parse_datetime)
    }

    pub fn last_popup_dt(&self) -> Option<DateTime<Local>> {
        self.last_popup_at.as_deref().and_then(parse_datetime)
    }
}

pub fn load_state() -> Result<DaemonState> {
    let path = state_file_path()?;
    if !path.exists() {
        return Ok(DaemonState::default());
    }

    let contents = fs::read_to_string(&path)
        .with_context(|| format!("failed to read state file at {}", path.display()))?;
    let state = toml::from_str::<DaemonState>(&contents)
        .with_context(|| format!("failed to parse state file at {}", path.display()))?;

    Ok(state)
}

pub fn save_state(state: &DaemonState) -> Result<()> {
    ensure_app_dirs()?;
    let path = state_file_path()?;
    let contents = toml::to_string_pretty(state).context("failed to serialize daemon state")?;
    fs::write(&path, contents)
        .with_context(|| format!("failed to write state file at {}", path.display()))?;
    Ok(())
}

pub fn start(current_exe: &Path) -> Result<DaemonState> {
    let mut state = load_state()?;
    if state.running {
        return Ok(state);
    }

    ensure_app_dirs()?;
    clear_command_inbox()?;
    Command::new(current_exe)
        .arg("__daemon-run")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("failed to spawn daemon via {}", current_exe.display()))?;

    state.running = true;
    state.active_reminder_id = None;
    state.next_due_at = None;
    state.cycle_started_at = Some(Local::now().to_rfc3339());
    state.snoozed_until = None;
    state.last_popup_at = None;
    state.last_action = Some(format!(
        "daemon spawn requested at {}",
        Local::now().to_rfc3339()
    ));
    save_state(&state)?;

    Ok(state)
}

pub fn stop() -> Result<DaemonState> {
    send_command(&DaemonCommand::Stop)?;
    for _ in 0..20 {
        let state = load_state()?;
        if !state.running {
            return Ok(state);
        }
        thread::sleep(StdDuration::from_millis(100));
    }

    let mut state = load_state()?;
    state.running = false;
    state.active_reminder_id = None;
    state.next_due_at = None;
    state.cycle_started_at = None;
    state.snoozed_until = None;
    state.last_action = Some(format!("stop requested at {}", Local::now().to_rfc3339()));
    save_state(&state)?;
    Ok(state)
}

pub fn pause_today() -> Result<DaemonState> {
    send_command(&DaemonCommand::PauseToday)?;
    let mut state = load_state()?;
    state.paused_until = Some(Local::now().date_naive().to_string());
    state.active_reminder_id = None;
    state.next_due_at = None;
    state.last_action = Some(format!("pause requested at {}", Local::now().to_rfc3339()));
    save_state(&state)?;
    Ok(state)
}

pub fn resume() -> Result<DaemonState> {
    send_command(&DaemonCommand::Resume)?;
    let mut state = load_state()?;
    state.paused_until = None;
    state.active_reminder_id = None;
    state.next_due_at = None;
    if state.cycle_started_at.is_none() {
        state.cycle_started_at = Some(Local::now().to_rfc3339());
    }
    state.last_action = Some(format!("resume requested at {}", Local::now().to_rfc3339()));
    save_state(&state)?;
    Ok(state)
}

pub fn send_popup_action(reminder_id: String, action: ReminderAction) -> Result<()> {
    send_command(&DaemonCommand::PopupAction {
        reminder_id,
        action,
    })?;
    Ok(())
}

pub fn set_tray_pid(pid: u32) -> Result<DaemonState> {
    let mut state = load_state()?;
    state.tray_pid = Some(pid);
    state.last_action = Some(format!("tray started at {}", Local::now().to_rfc3339()));
    save_state(&state)?;
    Ok(state)
}

pub fn clear_tray_pid(pid: u32) -> Result<DaemonState> {
    let mut state = load_state()?;
    if state.tray_pid == Some(pid) {
        state.tray_pid = None;
        state.last_action = Some(format!("tray stopped at {}", Local::now().to_rfc3339()));
        save_state(&state)?;
    }
    Ok(state)
}

pub fn reset_tray_pid() -> Result<DaemonState> {
    let mut state = load_state()?;
    if state.tray_pid.is_some() {
        state.tray_pid = None;
        state.last_action = Some(format!(
            "stale tray cleared at {}",
            Local::now().to_rfc3339()
        ));
        save_state(&state)?;
    }
    Ok(state)
}

pub fn daemon_loop(current_exe: &Path) -> Result<()> {
    ensure_app_dirs()?;

    let mut state = load_state()?;
    state.running = true;
    state.daemon_pid = Some(std::process::id());
    state
        .cycle_started_at
        .get_or_insert_with(|| Local::now().to_rfc3339());
    state.last_action = Some(format!("daemon started at {}", Local::now().to_rfc3339()));
    save_state(&state)?;

    loop {
        let config = load_config()?;
        let commands = drain_commands()?;

        if apply_commands(&mut state, &config, commands)? {
            break;
        }

        if !state.is_paused_today() && state.active_reminder_id.is_none() {
            let now = Local::now();
            let next_due = next_reminder_after_with_anchor(
                &config,
                now,
                state.snoozed_until_dt(),
                state.cycle_started_dt(),
            )?;
            state.next_due_at = Some(next_due.at.to_rfc3339());

            if next_due.at <= now + chrono::Duration::seconds(1) {
                let reminder_id = format!("reminder-{}", now.timestamp_millis());
                spawn_popup(current_exe, &reminder_id)?;
                state.active_reminder_id = Some(reminder_id);
                state.last_popup_at = Some(now.to_rfc3339());
                state.last_action = Some(format!("popup shown at {}", now.to_rfc3339()));
                state.snoozed_until = None;
            }
        } else if state.is_paused_today() {
            state.next_due_at = None;
        }

        save_state(&state)?;
        thread::sleep(StdDuration::from_secs(1));
    }

    state.running = false;
    state.active_reminder_id = None;
    state.daemon_pid = None;
    state.next_due_at = None;
    state.last_action = Some(format!("daemon stopped at {}", Local::now().to_rfc3339()));
    save_state(&state)?;

    Ok(())
}

fn apply_commands(
    state: &mut DaemonState,
    config: &AppConfig,
    commands: Vec<DaemonCommand>,
) -> Result<bool> {
    let now = Local::now();

    for command in commands {
        match command {
            DaemonCommand::Stop => {
                state.last_action = Some(format!("stop command applied at {}", now.to_rfc3339()));
                return Ok(true);
            }
            DaemonCommand::PauseToday => {
                state.paused_until = Some(now.date_naive().to_string());
                state.active_reminder_id = None;
                state.next_due_at = None;
                state.last_action = Some(format!("paused for today at {}", now.to_rfc3339()));
            }
            DaemonCommand::Resume => {
                state.paused_until = None;
                if state.cycle_started_at.is_none() {
                    state.cycle_started_at = Some(now.to_rfc3339());
                }
                state.last_action = Some(format!("resumed at {}", now.to_rfc3339()));
            }
            DaemonCommand::PopupAction {
                reminder_id,
                action,
            } => {
                if state.active_reminder_id.as_deref() != Some(reminder_id.as_str()) {
                    continue;
                }

                match crondrop_core::outcome_for_action(config, now, action.clone()) {
                    ActionOutcome::ClearActive => {
                        state.active_reminder_id = None;
                        state.snoozed_until = None;
                        state.cycle_started_at = Some(now.to_rfc3339());
                    }
                    ActionOutcome::SnoozedUntil(until) => {
                        state.active_reminder_id = None;
                        state.snoozed_until = Some(until.to_rfc3339());
                        state.cycle_started_at = Some(now.to_rfc3339());
                    }
                    ActionOutcome::PausedUntil(until) => {
                        state.active_reminder_id = None;
                        state.paused_until = Some(until.to_string());
                        state.snoozed_until = None;
                        state.cycle_started_at = None;
                    }
                }

                state.last_action = Some(format!(
                    "popup action {:?} for {} at {}",
                    action,
                    reminder_id,
                    now.to_rfc3339()
                ));
            }
        }
    }

    Ok(false)
}

fn drain_commands() -> Result<Vec<DaemonCommand>> {
    ensure_app_dirs()?;
    let dir = command_inbox_dir()?;
    let mut paths = fs::read_dir(&dir)
        .with_context(|| format!("failed to read command inbox at {}", dir.display()))?
        .filter_map(|entry| entry.ok().map(|value| value.path()))
        .collect::<Vec<_>>();

    paths.sort();

    let mut commands = Vec::new();
    for path in paths {
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }

        let contents = fs::read_to_string(&path)
            .with_context(|| format!("failed to read command file at {}", path.display()))?;
        let command = serde_json::from_str::<DaemonCommand>(&contents)
            .with_context(|| format!("failed to parse command file at {}", path.display()))?;
        fs::remove_file(&path)
            .with_context(|| format!("failed to remove command file at {}", path.display()))?;
        commands.push(command);
    }

    Ok(commands)
}

fn send_command(command: &DaemonCommand) -> Result<PathBuf> {
    ensure_app_dirs()?;
    let dir = command_inbox_dir()?;
    let file_name = format!(
        "{}-{}.json",
        Local::now().timestamp_millis(),
        std::process::id()
    );
    let path = dir.join(file_name);
    let contents = serde_json::to_string(command).context("failed to serialize daemon command")?;
    fs::write(&path, contents)
        .with_context(|| format!("failed to write command file at {}", path.display()))?;
    Ok(path)
}

fn clear_command_inbox() -> Result<()> {
    let dir = command_inbox_dir()?;
    if !dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(&dir)
        .with_context(|| format!("failed to read command inbox at {}", dir.display()))?
    {
        let path = entry
            .with_context(|| format!("failed to read entry in {}", dir.display()))?
            .path();
        if path.is_file() {
            fs::remove_file(&path)
                .with_context(|| format!("failed to remove stale command at {}", path.display()))?;
        }
    }

    Ok(())
}

fn spawn_popup(current_exe: &Path, reminder_id: &str) -> Result<()> {
    Command::new(current_exe)
        .arg("__popup")
        .arg("--reminder-id")
        .arg(reminder_id)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("failed to spawn popup via {}", current_exe.display()))?;
    Ok(())
}

fn parse_datetime(value: &str) -> Option<DateTime<Local>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .and_then(|value| Local.from_local_datetime(&value.naive_local()).single())
}

fn parse_date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
}

#[cfg(test)]
mod tests {
    use chrono::Local;

    use super::DaemonState;

    #[test]
    fn paused_state_detects_today() {
        let state = DaemonState {
            paused_until: Some(Local::now().date_naive().to_string()),
            ..DaemonState::default()
        };

        assert!(state.is_paused_today());
    }
}
