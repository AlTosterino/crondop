use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use crondrop_core::{load_config, next_reminder_after_with_anchor};
use crondrop_daemon::{
    load_state, pause_today as daemon_pause_today, resume as daemon_resume,
    start as daemon_start, stop as daemon_stop,
};
use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder, TrayIconEvent};
use winit::application::ApplicationHandler;
use winit::event::StartCause;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};

pub fn platform_name() -> &'static str {
    std::env::consts::OS
}

pub fn process_is_running(pid: u32) -> bool {
    process_is_running_impl(pid)
}

pub fn autostart_target(app_name: &str) -> Result<PathBuf> {
    match std::env::consts::OS {
        "macos" => {
            let home = dirs::home_dir().context("failed to resolve home directory")?;
            Ok(home
                .join("Library")
                .join("LaunchAgents")
                .join(format!("com.{app_name}.plist")))
        }
        "windows" => {
            let app_data =
                dirs::data_dir().context("failed to resolve roaming application data directory")?;
            Ok(app_data
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs")
                .join("Startup")
                .join(format!("{app_name}.cmd")))
        }
        _ => {
            let config_dir =
                dirs::config_dir().context("failed to resolve XDG config directory")?;
            Ok(config_dir
                .join("autostart")
                .join(format!("{app_name}.desktop")))
        }
    }
}

pub fn autostart_contents(app_name: &str, executable: &str) -> String {
    match std::env::consts::OS {
        "macos" => format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
  <dict>
    <key>Label</key>
    <string>com.{app_name}</string>
    <key>ProgramArguments</key>
    <array>
      <string>{executable}</string>
      <string>start</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <false/>
  </dict>
</plist>
"#
        ),
        "windows" => format!(
            r#"@echo off
"{executable}" start
"#
        ),
        _ => format!(
            r#"[Desktop Entry]
Type=Application
Version=1.0
Name={app_name}
Exec={executable} start
Terminal=false
X-GNOME-Autostart-enabled=true
"#
        ),
    }
}

pub fn run_tray(current_exe: &Path) -> Result<()> {
    configure_macos_tray_app();

    let event_loop = EventLoop::<UserEvent>::with_user_event()
        .build()
        .context("failed to create tray event loop")?;

    let proxy = event_loop.create_proxy();
    TrayIconEvent::set_event_handler(Some({
        let proxy = proxy.clone();
        move |event| {
            let _ = proxy.send_event(UserEvent::Tray(event));
        }
    }));
    MenuEvent::set_event_handler(Some(move |event| {
        let _ = proxy.send_event(UserEvent::Menu(event));
    }));

    let mut app = TrayApplication::new(current_exe.to_path_buf());
    let result = event_loop.run_app(&mut app);

    TrayIconEvent::set_event_handler::<fn(TrayIconEvent)>(None);
    MenuEvent::set_event_handler::<fn(MenuEvent)>(None);

    result.context("tray event loop failed")
}

#[derive(Debug)]
enum UserEvent {
    Menu(MenuEvent),
    Tray(TrayIconEvent),
}

struct TrayApplication {
    current_exe: PathBuf,
    tray_icon: Option<TrayIcon>,
    menu: Option<TrayMenu>,
    last_refresh: Instant,
    stop_daemon_on_exit: bool,
}

impl TrayApplication {
    fn new(current_exe: PathBuf) -> Self {
        Self {
            current_exe,
            tray_icon: None,
            menu: None,
            last_refresh: Instant::now() - Duration::from_secs(5),
            stop_daemon_on_exit: true,
        }
    }

    fn create_tray(&mut self) -> Result<()> {
        if self.tray_icon.is_some() {
            return Ok(());
        }

        let menu = TrayMenu::new()?;
        let icon = build_tray_icon(0.0)?;
        let mut builder = TrayIconBuilder::new()
            .with_tooltip("Cron Drop")
            .with_icon(icon)
            .with_menu(Box::new(menu.menu.clone()))
            .with_menu_on_left_click(true);

        if std::env::consts::OS == "macos" {
            builder = builder.with_icon_as_template(true).with_menu_on_left_click(true);
        }

        let tray_icon = builder.build().context("failed to build tray icon")?;
        tray_icon.set_show_menu_on_left_click(true);
        self.menu = Some(menu);
        self.tray_icon = Some(tray_icon);
        self.refresh()?;
        wake_macos_runloop();
        Ok(())
    }

    fn refresh(&mut self) -> Result<()> {
        let Some(menu) = self.menu.as_ref() else {
            return Ok(());
        };

        let snapshot = load_tray_snapshot()?;
        refresh_menu_items(
            &menu.status_item,
            &menu.next_due_item,
            &menu.pause_resume_item,
            &snapshot,
        );
        if let Some(tray_icon) = self.tray_icon.as_ref() {
            let icon = build_tray_icon(snapshot.fill_fraction)?;
            tray_icon.set_icon_with_as_template(Some(icon), std::env::consts::OS == "macos")?;
        }
        self.last_refresh = Instant::now();
        Ok(())
    }

    fn handle_menu_event(&mut self, event_loop: &ActiveEventLoop, id: MenuId) {
        let Some(menu) = self.menu.as_ref() else {
            return;
        };

        let result = if id == menu.start_item.id() {
            daemon_start(&self.current_exe).map(|_| ())
        } else if id == menu.stop_item.id() {
            daemon_stop().map(|_| ())
        } else if id == menu.pause_resume_item.id() {
            match load_tray_snapshot() {
                Ok(snapshot) if snapshot.paused_today => daemon_resume().map(|_| ()),
                Ok(_) => daemon_pause_today().map(|_| ()),
                Err(error) => Err(error),
            }
        } else if id == menu.test_popup_item.id() {
            run_command(&self.current_exe, &["test-popup"])
        } else if id == menu.quit_item.id() {
            event_loop.exit();
            Ok(())
        } else {
            Ok(())
        };

        if let Err(error) = result {
            if let Some(menu) = self.menu.as_ref() {
                menu.status_item.set_text(format!("Status: error ({error})"));
            }
        }
    }
}

impl ApplicationHandler<UserEvent> for TrayApplication {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        if cause == StartCause::Init {
            if let Err(error) = self.create_tray() {
                eprintln!("failed to initialize tray: {error:#}");
                event_loop.exit();
                return;
            }
        }

        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_secs(1),
        ));
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Menu(event) => self.handle_menu_event(event_loop, event.id),
            UserEvent::Tray(_event) => {}
        }

        if let Err(error) = self.refresh() {
            if let Some(menu) = self.menu.as_ref() {
                menu.status_item.set_text(format!("Status: error ({error})"));
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.last_refresh.elapsed() >= Duration::from_secs(1) {
            if let Err(error) = self.refresh() {
                if let Some(menu) = self.menu.as_ref() {
                    menu.status_item.set_text(format!("Status: error ({error})"));
                }
            }
        }

        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_secs(1),
        ));
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        if self.stop_daemon_on_exit {
            let _ = daemon_stop();
        }
        self.tray_icon = None;
        self.menu = None;
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        _event: winit::event::WindowEvent,
    ) {
    }
}

#[derive(Clone)]
struct TrayMenu {
    menu: Menu,
    status_item: MenuItem,
    next_due_item: MenuItem,
    start_item: MenuItem,
    stop_item: MenuItem,
    pause_resume_item: MenuItem,
    test_popup_item: MenuItem,
    quit_item: MenuItem,
}

impl TrayMenu {
    fn new() -> Result<Self> {
        let menu = Menu::new();

        let status_item = MenuItem::new("Status: loading...", false, None);
        let next_due_item = MenuItem::new("Next drop: unknown", false, None);
        let separator_top = PredefinedMenuItem::separator();
        let start_item = MenuItem::new("Start reminders", true, None);
        let stop_item = MenuItem::new("Stop reminders", true, None);
        let pause_resume_item = MenuItem::new("Pause today", true, None);
        let test_popup_item = MenuItem::new("Show popup now", true, None);
        let separator_bottom = PredefinedMenuItem::separator();
        let quit_item = MenuItem::new("Quit Cron Drop", true, None);

        menu.append_items(&[
            &status_item,
            &next_due_item,
            &separator_top,
            &start_item,
            &stop_item,
            &pause_resume_item,
            &test_popup_item,
            &separator_bottom,
            &quit_item,
        ])?;

        Ok(Self {
            menu,
            status_item,
            next_due_item,
            start_item,
            stop_item,
            pause_resume_item,
            test_popup_item,
            quit_item,
        })
    }
}

fn refresh_menu_items(
    status_item: &MenuItem,
    next_due_item: &MenuItem,
    pause_resume_item: &MenuItem,
    snapshot: &TraySnapshot,
) {
    let status = if snapshot.running && snapshot.paused_today {
        "paused today"
    } else if snapshot.running {
        "running"
    } else {
        "stopped"
    };

    status_item.set_text(format!("Status: {status}"));
    next_due_item.set_text(format!(
        "Next drop: {}",
        relative_due_label(snapshot.next_due_at)
    ));
    pause_resume_item.set_text(if snapshot.paused_today {
        "Resume"
    } else {
        "Pause today"
    });
}

fn run_command(current_exe: &Path, args: &[&str]) -> Result<()> {
    Command::new(current_exe)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("failed to run {:?} via {}", args, current_exe.display()))?;
    Ok(())
}

fn build_tray_icon(fill_fraction: f32) -> Result<Icon> {
    let width = 22;
    let height = 22;
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    let fill_fraction = fill_fraction.clamp(0.0, 1.0);
    let fill_threshold = (height as f32 - 2.0) * (1.0 - fill_fraction);

    for y in 0..height {
        for x in 0..width {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let is_drop = drop_mask(px, py, false);
            let is_inner = drop_mask(px, py, true);
            let is_outline = is_drop && !is_inner;
            let is_fill = is_inner && (y as f32) >= fill_threshold;
            let alpha = if is_outline || is_fill { 255 } else { 0 };
            let value = 0;
            rgba.extend_from_slice(&[value, value, value, alpha]);
        }
    }

    Icon::from_rgba(rgba, width, height).context("failed to build tray icon")
}

fn drop_mask(x: f32, y: f32, inner: bool) -> bool {
    let center_x = 11.0;
    let bulb_center_y = if inner { 13.6 } else { 13.8 };
    let bulb_radius = if inner { 4.2 } else { 5.4 };
    let dx = x - center_x;
    let dy = y - bulb_center_y;

    let in_bulb = dx * dx + dy * dy <= bulb_radius * bulb_radius;

    let body_top = if inner { 3.0 } else { 2.0 };
    let body_bottom = if inner { 12.8 } else { 13.2 };
    let in_body = if y >= body_top && y <= body_bottom {
        let t = (y - body_top) / (body_bottom - body_top);
        let start_half = if inner { 0.6 } else { 1.2 };
        let end_half = if inner { 3.6 } else { 4.8 };
        let half_width = start_half + (end_half - start_half) * t;
        dx.abs() <= half_width
    } else {
        false
    };

    let tip_top = if inner { 1.0 } else { 0.0 };
    let tip_bottom = if inner { 3.0 } else { 2.0 };
    let in_tip = if y >= tip_top && y <= tip_bottom {
        let t = (y - tip_top) / (tip_bottom - tip_top);
        let base_half = if inner { 0.6 } else { 1.2 };
        let half_width = base_half * t;
        dx.abs() <= half_width
    } else {
        false
    };

    in_bulb || in_body || in_tip
}

#[derive(Debug, Clone)]
struct TraySnapshot {
    running: bool,
    paused_today: bool,
    next_due_at: Option<DateTime<Local>>,
    fill_fraction: f32,
}

fn load_tray_snapshot() -> Result<TraySnapshot> {
    let config = load_config()?;
    let state = load_state()?;
    let now = Local::now();
    let mut next_due = state.next_due_dt().or_else(|| {
        if state.running && !state.is_paused_today() {
            next_reminder_after_with_anchor(
                &config,
                now,
                state.snoozed_until_dt(),
                state.cycle_started_dt(),
            )
                .ok()
                .map(|next| next.at)
        } else {
            None
        }
    });

    if state.running && !state.is_paused_today() && next_due.is_some_and(|value| value <= now) {
        next_due = next_reminder_after_with_anchor(
            &config,
            now,
            state.snoozed_until_dt(),
            state.cycle_started_dt(),
        )
            .ok()
            .map(|next| next.at);
    }

    let fill_fraction = match next_due {
        Some(next_due) => reminder_fill_fraction(&state, now, next_due),
        None => 0.0,
    };

    Ok(TraySnapshot {
        running: state.running,
        paused_today: state.is_paused_today(),
        next_due_at: next_due,
        fill_fraction,
    })
}

fn reminder_fill_fraction(state: &crondrop_daemon::DaemonState, now: DateTime<Local>, next_due: DateTime<Local>) -> f32 {
    if next_due <= now {
        return 1.0;
    }

    let previous = state
        .cycle_started_dt()
        .or_else(|| state.last_popup_dt())
        .filter(|value| *value < next_due);
    let Some(previous) = previous else {
        return 0.0;
    };

    let total = (next_due - previous).num_seconds().max(1) as f32;
    let elapsed = (now - previous).num_seconds().clamp(0, total as i64) as f32;
    (elapsed / total).clamp(0.0, 1.0)
}

fn relative_due_label(next_due: Option<DateTime<Local>>) -> String {
    let Some(next_due) = next_due else {
        return "none".to_string();
    };

    let now = Local::now();
    let seconds = (next_due - now).num_seconds();
    if seconds <= 0 {
        return "now".to_string();
    }
    if seconds < 60 {
        return format!("in {}s", seconds);
    }

    let minutes = (seconds + 59) / 60;
    if minutes < 60 {
        return format!("in {}m", minutes);
    }

    let hours = minutes / 60;
    let rem_minutes = minutes % 60;
    if rem_minutes == 0 {
        format!("in {}h", hours)
    } else {
        format!("in {}h {}m", hours, rem_minutes)
    }
}

#[cfg(unix)]
fn process_is_running_impl(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }

    let result = unsafe { libc::kill(pid as i32, 0) };
    if result == 0 {
        return true;
    }

    std::io::Error::last_os_error()
        .raw_os_error()
        .is_some_and(|code| code == libc::EPERM)
}

#[cfg(windows)]
fn process_is_running_impl(_pid: u32) -> bool {
    true
}

#[cfg(target_os = "macos")]
fn wake_macos_runloop() {
    use objc2_core_foundation::CFRunLoop;

    if let Some(run_loop) = CFRunLoop::main() {
        run_loop.wake_up();
    }
}

#[cfg(target_os = "macos")]
fn configure_macos_tray_app() {
    use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
    use objc2_foundation::MainThreadMarker;

    if let Some(marker) = MainThreadMarker::new() {
        let app = NSApplication::sharedApplication(marker);
        let _ = app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    }
}

#[cfg(not(target_os = "macos"))]
fn configure_macos_tray_app() {}

#[cfg(not(target_os = "macos"))]
fn wake_macos_runloop() {}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Local};
    use crondrop_daemon::DaemonState;

    use super::{autostart_contents, relative_due_label, reminder_fill_fraction};

    #[test]
    fn autostart_contents_embed_start_command() {
        let rendered = autostart_contents("crondrop", "/tmp/crondrop");
        assert!(rendered.contains("crondrop"));
        assert!(rendered.contains("start"));
    }

    #[test]
    fn interval_fill_fraction_stays_in_range() {
        let next_due = Local::now() + Duration::minutes(20);
        let fill = reminder_fill_fraction(&DaemonState::default(), Local::now(), next_due);
        assert!((0.0..=1.0).contains(&fill));
    }

    #[test]
    fn relative_due_label_renders_remaining_time() {
        let label = relative_due_label(Some(Local::now() + Duration::minutes(5)));
        assert!(label.starts_with("in "));
    }
}
