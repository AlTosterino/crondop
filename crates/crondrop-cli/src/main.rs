use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use crondrop_core::{AppConfig, Theme, config_file_path, load_config, save_config};
use std::fs;
use std::process::Stdio;
use std::thread;
use std::time::Duration;

#[derive(Debug, Parser)]
#[command(
    name = "crondrop",
    version,
    about = "A friendly CLI-first eye drop reminder",
    long_about = "Cron Drop helps you schedule recurring eye drop reminders from the command line while keeping the daily experience gentle and low-friction. Set a schedule once, let the daemon run quietly, and use the tray for quick control.",
    after_help = "Examples:\n  crondrop init\n  crondrop schedule every 1h --from 08:00 --to 22:00\n  crondrop schedule add --at 09:00 --at 13:00 --at 18:00\n  crondrop popup\n  crondrop preview\n  crondrop status",
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Create a default configuration file.
    Init,
    /// Create or replace the active reminder schedule.
    Schedule(ScheduleCommand),
    /// Change the popup visual theme.
    Theme(ThemeCommand),
    /// Start Cron Drop in the background.
    Start,
    /// Alias for `start`.
    Run,
    /// Stop the background daemon.
    Stop,
    /// Quit Cron Drop completely, including the tray process.
    Quit,
    /// Restart the background daemon.
    Restart,
    /// Show the current Cron Drop status.
    Status,
    /// Pause reminders for the rest of today.
    Pause(PauseCommand),
    /// Resume reminders after a pause.
    Resume,
    /// Open the reminder popup immediately so you can preview the design.
    #[command(visible_aliases = ["popup", "preview", "show-popup", "test"])]
    TestPopup,
    /// Inspect the current configuration.
    Config(ConfigCommand),
    /// Manage launch-at-login integration.
    Autostart(AutostartCommand),
    /// Run the tray process in the foreground.
    Tray,
    #[command(hide = true, name = "__daemon-run")]
    DaemonRun,
    #[command(hide = true, name = "__popup")]
    Popup(PopupCommand),
    #[command(hide = true, name = "__tray-run")]
    TrayRun,
}

#[derive(Debug, Args)]
#[command(arg_required_else_help = true)]
struct ScheduleCommand {
    #[command(subcommand)]
    command: ScheduleSubcommand,
}

#[derive(Debug, Subcommand)]
enum ScheduleSubcommand {
    /// Repeat a reminder on an interval like `2m` or `1h`.
    Every {
        /// Interval in minutes or hours, for example `2m` or `1h`.
        interval: String,
        #[arg(long)]
        /// Optional start time in HH:MM format.
        from: Option<String>,
        #[arg(long)]
        /// Optional end time in HH:MM format.
        to: Option<String>,
        #[arg(long, default_value_t = false)]
        /// Limit reminders to Monday-Friday.
        weekdays_only: bool,
        #[arg(long, default_value_t = false)]
        /// Save the schedule without starting Cron Drop.
        no_start: bool,
    },
    /// Run reminders at specific clock times.
    Add {
        #[arg(long = "at", required = true)]
        /// One or more times in HH:MM format.
        at: Vec<String>,
        #[arg(long, default_value_t = false)]
        /// Limit reminders to Monday-Friday.
        weekdays_only: bool,
        #[arg(long, default_value_t = false)]
        /// Save the schedule without starting Cron Drop.
        no_start: bool,
    },
}

#[derive(Debug, Args)]
struct ThemeCommand {
    /// Theme name, for example `cozy`, `dawn`, or `mist`.
    name: String,
}

#[derive(Debug, Args)]
struct PauseCommand {
    #[arg(long, default_value_t = false)]
    /// Pause reminders until tomorrow.
    today: bool,
}

#[derive(Debug, Args)]
#[command(arg_required_else_help = true)]
struct ConfigCommand {
    #[command(subcommand)]
    command: ConfigSubcommand,
}

#[derive(Debug, Subcommand)]
enum ConfigSubcommand {
    /// Print the loaded configuration.
    Show,
    /// Print the config file path.
    Path,
}

#[derive(Debug, Args)]
struct PopupCommand {
    #[arg(long)]
    /// Internal reminder identifier used by spawned popup windows.
    reminder_id: String,
}

#[derive(Debug, Args)]
#[command(arg_required_else_help = true)]
struct AutostartCommand {
    #[command(subcommand)]
    command: AutostartSubcommand,
}

#[derive(Debug, Subcommand)]
enum AutostartSubcommand {
    /// Install launch-at-login for the current user.
    Install,
    /// Remove launch-at-login for the current user.
    Remove,
    /// Show the autostart target path and whether it exists.
    Status,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .without_time()
        .with_max_level(tracing::Level::WARN)
        .init();

    let cli = Cli::parse();
    run(cli)
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Init => init_config(),
        Commands::Schedule(schedule) => schedule_config(schedule),
        Commands::Theme(theme) => set_theme(theme),
        Commands::Start => start_daemon(),
        Commands::Run => start_daemon(),
        Commands::Stop => stop_daemon(),
        Commands::Quit => quit(),
        Commands::Restart => restart_daemon(),
        Commands::Status => status(),
        Commands::Pause(pause) => pause_command(pause),
        Commands::Resume => resume(),
        Commands::TestPopup => test_popup(),
        Commands::Config(config) => config_command(config),
        Commands::Autostart(command) => autostart_command(command),
        Commands::Tray => tray_command(),
        Commands::DaemonRun => daemon_run(),
        Commands::Popup(command) => popup_command(command),
        Commands::TrayRun => tray_run(),
    }
}

fn init_config() -> Result<()> {
    let config = AppConfig::default();
    let path = save_config(&config)?;
    println!("Cron Drop is ready.");
    println!("Config file: {}", path.display());
    println!("Next step: run `crondrop schedule every 1h`");
    Ok(())
}

fn schedule_config(command: ScheduleCommand) -> Result<()> {
    let mut config = load_config()?;
    let auto_start = match command.command {
        ScheduleSubcommand::Every {
            interval,
            from,
            to,
            weekdays_only,
            no_start,
        } => {
            let minutes = parse_interval_minutes(&interval)?;
            config.schedule.every_minutes = minutes;
            config.schedule.mode = crondrop_core::ScheduleMode::Interval;
            config.schedule.fixed_times.clear();

            if let Some(from) = from {
                config.schedule.active_from = from;
            }

            if let Some(to) = to {
                config.schedule.active_to = to;
            }

            config.schedule.weekdays_only = weekdays_only;
            !no_start
        }
        ScheduleSubcommand::Add {
            at,
            weekdays_only,
            no_start,
        } => {
            config.schedule.mode = crondrop_core::ScheduleMode::FixedTimes;
            config.schedule.fixed_times = at;
            config.schedule.weekdays_only = weekdays_only;
            !no_start
        }
    };

    let path = save_config(&config)?;
    println!("Cron Drop schedule updated.");
    match config.schedule.mode {
        crondrop_core::ScheduleMode::Interval => {
            println!("Pattern: every {} minutes", config.schedule.every_minutes,)
        }
        crondrop_core::ScheduleMode::FixedTimes => println!(
            "Pattern: fixed times [{}]",
            config.schedule.fixed_times.join(", "),
        ),
    }
    println!(
        "Active hours: {} to {}",
        config.schedule.active_from, config.schedule.active_to
    );
    println!("Weekdays only: {}", yes_no(config.schedule.weekdays_only));
    println!("Config file: {}", path.display());
    maybe_start_after_schedule(auto_start)?;
    Ok(())
}

fn set_theme(command: ThemeCommand) -> Result<()> {
    let mut config = load_config()?;
    config.ui.theme = Theme::parse(&command.name);
    let path = save_config(&config)?;
    println!("Cron Drop theme updated.");
    println!("Theme: {}", config.ui.theme.as_str());
    println!("Config file: {}", path.display());
    Ok(())
}

fn start_daemon() -> Result<()> {
    let current_exe = std::env::current_exe().context("failed to resolve current executable")?;
    let config = load_config()?;
    crondrop_daemon::start(&current_exe)?;
    if config.behavior.minimize_to_tray {
        ensure_tray_running(&current_exe)?;
    }
    let state = crondrop_daemon::load_state()?;
    println!("Cron Drop is running.");
    println!("Daemon: {}", yes_no(state.running));
    println!("Tray: {}", yes_no(tray_pid_is_live(state.tray_pid)));
    println!(
        "Next drop: {}",
        humanize_next_due(state.next_due_at.as_deref().unwrap_or("none"))
    );
    Ok(())
}

fn stop_daemon() -> Result<()> {
    let state = crondrop_daemon::stop()?;
    println!("Cron Drop stopped.");
    println!("Daemon: {}", yes_no(state.running));
    Ok(())
}

fn quit() -> Result<()> {
    let mut state = crondrop_daemon::load_state()?;

    if let Some(pid) = state.tray_pid.filter(|_| tray_pid_is_live(state.tray_pid)) {
        crondrop_platform::terminate_process(pid)
            .with_context(|| format!("failed to stop tray process {pid}"))?;

        for _ in 0..20 {
            if !crondrop_platform::process_is_running(pid) {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
    }

    if state.tray_pid.is_some() {
        state = crondrop_daemon::reset_tray_pid()?;
    }

    if state.running {
        state = crondrop_daemon::stop()?;
    }

    println!("Cron Drop quit.");
    println!("Daemon: {}", yes_no(state.running));
    println!("Tray: {}", yes_no(tray_pid_is_live(state.tray_pid)));
    Ok(())
}

fn restart_daemon() -> Result<()> {
    let current_exe = std::env::current_exe().context("failed to resolve current executable")?;
    crondrop_daemon::stop()?;
    let state = crondrop_daemon::start(&current_exe)?;
    println!("Cron Drop restarted.");
    println!("Daemon: {}", yes_no(state.running));
    println!(
        "Next drop: {}",
        humanize_next_due(state.next_due_at.as_deref().unwrap_or("none"))
    );
    Ok(())
}

fn status() -> Result<()> {
    let config_path = config_file_path()?;
    let mut state = crondrop_daemon::load_state()?;
    if state.tray_pid.is_some() && !tray_pid_is_live(state.tray_pid) {
        state = crondrop_daemon::reset_tray_pid()?;
    }
    println!("Cron Drop status");
    println!("Platform: {}", crondrop_platform::platform_name());
    println!("Config: {}", config_path.display());
    println!("Daemon running: {}", yes_no(state.running));
    println!("Tray running: {}", yes_no(tray_pid_is_live(state.tray_pid)));
    println!(
        "Next drop: {}",
        humanize_next_due(state.next_due_at.as_deref().unwrap_or("none"))
    );
    println!("Paused today: {}", yes_no(state.is_paused_today()));
    println!(
        "Last event: {}",
        state.last_action.as_deref().unwrap_or("none")
    );
    Ok(())
}

fn pause_command(command: PauseCommand) -> Result<()> {
    if !command.today {
        anyhow::bail!("only `crondrop pause --today` is supported right now");
    }

    let state = crondrop_daemon::pause_today()?;
    println!("Cron Drop paused for today.");
    println!(
        "Paused until: {}",
        state.paused_until.as_deref().unwrap_or("unknown")
    );
    Ok(())
}

fn resume() -> Result<()> {
    let state = crondrop_daemon::resume()?;
    println!("Cron Drop resumed.");
    println!(
        "Paused until: {}",
        state.paused_until.as_deref().unwrap_or("none")
    );
    Ok(())
}

fn test_popup() -> Result<()> {
    let config = load_config()?;
    println!("Opening a Cron Drop popup...");
    crondrop_ui::show_popup(config, "test-popup".to_string()).context("failed to open popup window")
}

fn config_command(command: ConfigCommand) -> Result<()> {
    match command.command {
        ConfigSubcommand::Show => {
            let config = load_config()?;
            let rendered =
                toml::to_string_pretty(&config).context("failed to render config as TOML")?;
            println!("# Cron Drop configuration");
            println!("{rendered}");
        }
        ConfigSubcommand::Path => {
            let path = config_file_path()?;
            println!("{}", path.display());
        }
    }

    Ok(())
}

fn parse_interval_minutes(value: &str) -> Result<u32> {
    let trimmed = value.trim().to_ascii_lowercase();

    if let Some(hours) = trimmed.strip_suffix('h') {
        let value = hours.parse::<u32>().context("invalid hour interval")?;
        return Ok(value * 60);
    }

    if let Some(minutes) = trimmed.strip_suffix('m') {
        let value = minutes.parse::<u32>().context("invalid minute interval")?;
        return Ok(value);
    }

    trimmed
        .parse::<u32>()
        .context("interval must be a number, or end with `m` or `h`")
}

fn maybe_start_after_schedule(auto_start: bool) -> Result<()> {
    if !auto_start {
        return Ok(());
    }

    start_daemon()
}

fn wait_for_tray_registration() -> Result<()> {
    for _ in 0..40 {
        let state = crondrop_daemon::load_state()?;
        if tray_pid_is_live(state.tray_pid) {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(100));
    }

    Ok(())
}

fn daemon_run() -> Result<()> {
    let current_exe = std::env::current_exe().context("failed to resolve current executable")?;
    crondrop_daemon::daemon_loop(&current_exe)
}

fn popup_command(command: PopupCommand) -> Result<()> {
    let config = load_config()?;
    crondrop_ui::show_popup(config, command.reminder_id).context("failed to open popup window")
}

fn tray_command() -> Result<()> {
    let state = crondrop_daemon::load_state()?;
    if tray_pid_is_live(state.tray_pid) {
        println!("Cron Drop tray is already running.");
        return Ok(());
    }
    if state.tray_pid.is_some() {
        crondrop_daemon::reset_tray_pid()?;
    }

    let current_exe = std::env::current_exe().context("failed to resolve current executable")?;
    std::process::Command::new(&current_exe)
        .arg("__tray-run")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("failed to spawn tray via {}", current_exe.display()))?;
    wait_for_tray_registration()?;
    println!("Cron Drop tray started.");
    Ok(())
}

fn tray_run() -> Result<()> {
    let pid = std::process::id();
    crondrop_daemon::set_tray_pid(pid)?;
    let current_exe = std::env::current_exe().context("failed to resolve current executable")?;
    let result = crondrop_platform::run_tray(&current_exe);
    let _ = crondrop_daemon::clear_tray_pid(pid);
    result
}

fn autostart_command(command: AutostartCommand) -> Result<()> {
    let current_exe = std::env::current_exe().context("failed to resolve current executable")?;
    let target = crondrop_platform::autostart_target("crondrop")?;

    match command.command {
        AutostartSubcommand::Install => {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).with_context(|| {
                    format!(
                        "failed to create autostart directory at {}",
                        parent.display()
                    )
                })?;
            }

            let contents = crondrop_platform::autostart_contents(
                "crondrop",
                &current_exe.display().to_string(),
            );
            fs::write(&target, contents).with_context(|| {
                format!("failed to write autostart file at {}", target.display())
            })?;
            println!("Cron Drop autostart installed.");
            println!("Target: {}", target.display());
        }
        AutostartSubcommand::Remove => {
            if target.exists() {
                fs::remove_file(&target).with_context(|| {
                    format!("failed to remove autostart file at {}", target.display())
                })?;
            }
            println!("Cron Drop autostart removed.");
            println!("Target: {}", target.display());
        }
        AutostartSubcommand::Status => {
            println!("Cron Drop autostart status");
            println!("Target: {}", target.display());
            println!("Installed: {}", yes_no(target.exists()));
        }
    }

    Ok(())
}

fn ensure_tray_running(current_exe: &std::path::Path) -> Result<()> {
    let state = crondrop_daemon::load_state()?;
    if tray_pid_is_live(state.tray_pid) {
        return Ok(());
    }

    if state.tray_pid.is_some() {
        crondrop_daemon::reset_tray_pid()?;
    }

    std::process::Command::new(current_exe)
        .arg("__tray-run")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("failed to spawn tray via {}", current_exe.display()))?;
    wait_for_tray_registration()?;
    Ok(())
}

fn tray_pid_is_live(pid: Option<u32>) -> bool {
    pid.is_some_and(crondrop_platform::process_is_running)
}

fn humanize_next_due(value: &str) -> String {
    if value == "none" {
        return "none".to_string();
    }

    value.replace('T', " ")
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}
