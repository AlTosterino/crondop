use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleMode {
    Interval,
    FixedTimes,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    Cozy,
    Dawn,
    Mist,
}

impl Theme {
    pub fn parse(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "dawn" => Self::Dawn,
            "mist" => Self::Mist,
            _ => Self::Cozy,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Cozy => "cozy",
            Self::Dawn => "dawn",
            Self::Mist => "mist",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleConfig {
    pub mode: ScheduleMode,
    pub every_minutes: u32,
    #[serde(default)]
    pub fixed_times: Vec<String>,
    pub active_from: String,
    pub active_to: String,
    pub weekdays_only: bool,
}

impl Default for ScheduleConfig {
    fn default() -> Self {
        Self {
            mode: ScheduleMode::Interval,
            every_minutes: 60,
            fixed_times: Vec::new(),
            active_from: "08:00".to_string(),
            active_to: "22:00".to_string(),
            weekdays_only: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub theme: Theme,
    pub always_on_top: bool,
    pub animation: String,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: Theme::Cozy,
            always_on_top: true,
            animation: "gentle".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopupConfig {
    pub title: String,
    pub body: String,
    pub show_snooze: bool,
    pub snooze_minutes: u32,
    pub show_pause_today: bool,
}

impl Default for PopupConfig {
    fn default() -> Self {
        Self {
            title: "Time for your eye drops".to_string(),
            body: "Take a short pause and put them in.".to_string(),
            show_snooze: true,
            snooze_minutes: 10,
            show_pause_today: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorConfig {
    pub start_on_login: bool,
    pub minimize_to_tray: bool,
    pub sound: bool,
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        Self {
            start_on_login: false,
            minimize_to_tray: true,
            sound: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub schedule: ScheduleConfig,
    pub ui: UiConfig,
    pub popup: PopupConfig,
    pub behavior: BehaviorConfig,
}
