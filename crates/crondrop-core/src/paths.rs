use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::AppConfig;

const APP_DIR_NAME: &str = "crondrop";
const CONFIG_FILE_NAME: &str = "config.toml";
const STATE_FILE_NAME: &str = "state.toml";
const COMMAND_INBOX_DIR_NAME: &str = "commands";
const CONFIG_DIR_ENV: &str = "CRONDROP_CONFIG_DIR";
const RUNTIME_DIR_ENV: &str = "CRONDROP_RUNTIME_DIR";

pub fn config_dir() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os(CONFIG_DIR_ENV) {
        return Ok(PathBuf::from(path));
    }

    dirs::config_dir()
        .map(|path| path.join(APP_DIR_NAME))
        .context("failed to resolve user config directory")
}

pub fn runtime_dir() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os(RUNTIME_DIR_ENV) {
        return Ok(PathBuf::from(path));
    }

    if let Some(data_dir) = dirs::data_local_dir() {
        return Ok(data_dir.join(APP_DIR_NAME));
    }

    config_dir()
}

pub fn config_file_path() -> Result<PathBuf> {
    Ok(config_dir()?.join(CONFIG_FILE_NAME))
}

pub fn state_file_path() -> Result<PathBuf> {
    Ok(runtime_dir()?.join(STATE_FILE_NAME))
}

pub fn command_inbox_dir() -> Result<PathBuf> {
    Ok(runtime_dir()?.join(COMMAND_INBOX_DIR_NAME))
}

pub fn ensure_app_dirs() -> Result<()> {
    fs::create_dir_all(config_dir()?).context("failed to create config directory")?;
    fs::create_dir_all(runtime_dir()?).context("failed to create runtime directory")?;
    fs::create_dir_all(command_inbox_dir()?).context("failed to create command inbox directory")?;
    Ok(())
}

pub fn load_config() -> Result<AppConfig> {
    let path = config_file_path()?;
    if !path.exists() {
        return Ok(AppConfig::default());
    }

    let contents = fs::read_to_string(&path)
        .with_context(|| format!("failed to read config file at {}", path.display()))?;
    let config = toml::from_str::<AppConfig>(&contents)
        .with_context(|| format!("failed to parse config file at {}", path.display()))?;

    Ok(config)
}

pub fn save_config(config: &AppConfig) -> Result<PathBuf> {
    ensure_app_dirs()?;
    let path = config_file_path()?;
    let contents = toml::to_string_pretty(config).context("failed to serialize config")?;
    fs::write(&path, contents)
        .with_context(|| format!("failed to write config file at {}", path.display()))?;
    Ok(path)
}
