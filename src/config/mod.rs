//! `config.toml` types, path resolution, and validation.
#![allow(dead_code)] // Consumed by `settings` and Greeter in Phase 2.

use std::{
    io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

pub const DEFAULT_LOG_FILE: &str = "/tmp/tuigreet.log";
pub const DEFAULT_XSESSION_WRAPPER: &str = "startx /usr/bin/env";
pub const DEFAULT_ASTERISKS_CHAR: &str = "*";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub logging: Logging,
    #[serde(default)]
    pub session: Session,
    #[serde(default)]
    pub ui: Ui,
    #[serde(default)]
    pub remember: Remember,
    #[serde(default)]
    pub user_menu: UserMenu,
    #[serde(default)]
    pub secrets: Secrets,
    #[serde(default)]
    pub keybindings: Keybindings,
    #[serde(default)]
    pub power: Power,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Logging {
    #[serde(default = "default_log_level")]
    pub level: LogLevel,
    pub file: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub cmd: Option<String>,
    #[serde(default)]
    pub env: Vec<String>,
    #[serde(default = "default_wayland_dirs")]
    pub wayland_dirs: Vec<PathBuf>,
    #[serde(default = "default_x11_dirs")]
    pub x11_dirs: Vec<PathBuf>,
    pub session_wrapper: Option<String>,
    #[serde(default = "default_xsession_wrapper")]
    pub xsession_wrapper: String,
    #[serde(default)]
    pub no_xsession_wrapper: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ui {
    #[serde(default = "default_width")]
    pub width: u16,
    #[serde(default)]
    pub window_padding: u16,
    #[serde(default = "default_container_padding")]
    pub container_padding: u16,
    #[serde(default = "default_prompt_padding")]
    pub prompt_padding: u16,
    #[serde(default)]
    pub greet_align: GreetAlign,
    #[serde(default)]
    pub show_time: bool,
    pub time_format: Option<String>,
    #[serde(default)]
    pub issue: bool,
    pub greeting: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GreetAlign {
    Left,
    #[default]
    Center,
    Right,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Remember {
    #[serde(default)]
    pub username: bool,
    #[serde(default)]
    pub session: bool,
    #[serde(default)]
    pub user_session: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserMenu {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_min_uid")]
    pub min_uid: u16,
    #[serde(default = "default_max_uid")]
    pub max_uid: u16,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Secrets {
    #[serde(default)]
    pub mask: bool,
    #[serde(default = "default_mask_char")]
    pub mask_char: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keybindings {
    #[serde(default = "default_kb_command")]
    pub command: u8,
    #[serde(default = "default_kb_sessions")]
    pub sessions: u8,
    #[serde(default = "default_kb_power")]
    pub power: u8,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Power {
    pub shutdown: Option<String>,
    pub reboot: Option<String>,
    #[serde(default)]
    pub no_setsid: bool,
}

fn default_log_level() -> LogLevel {
    LogLevel::Info
}

fn default_wayland_dirs() -> Vec<PathBuf> {
    vec![PathBuf::from("/usr/share/wayland-sessions")]
}

fn default_x11_dirs() -> Vec<PathBuf> {
    vec![PathBuf::from("/usr/share/xsessions")]
}

fn default_xsession_wrapper() -> String {
    DEFAULT_XSESSION_WRAPPER.to_string()
}

fn default_width() -> u16 {
    80
}

fn default_container_padding() -> u16 {
    1
}

fn default_prompt_padding() -> u16 {
    1
}

fn default_min_uid() -> u16 {
    1000
}

fn default_max_uid() -> u16 {
    60000
}

fn default_mask_char() -> String {
    DEFAULT_ASTERISKS_CHAR.to_string()
}

fn default_kb_command() -> u8 {
    2
}

fn default_kb_sessions() -> u8 {
    3
}

fn default_kb_power() -> u8 {
    12
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("config not found at {path}")]
    NotFound { path: PathBuf },

    #[error("IO: {0}")]
    Io(#[from] io::Error),

    #[error("TOML deserialize: {0}")]
    TomlDeserialize(#[from] toml::de::Error),

    #[error("{0}")]
    Validation(String),
}

/// Packager path: `/etc/tuigreet/config.toml`.
pub fn system_path() -> PathBuf {
    PathBuf::from("/etc/tuigreet/config.toml")
}

/// User path: `$XDG_CONFIG_HOME/tuigreet/config.toml` (fallback `~/.config/...`).
pub fn user_path() -> PathBuf {
    config_base_dir().join("config.toml")
}

fn config_base_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from("/"))
        .join("tuigreet")
}

/// Returns the explicit override path, or the standard search order (system then user).
pub fn resolved_paths(override_path: Option<&Path>) -> Vec<PathBuf> {
    if let Some(path) = override_path {
        return vec![path.to_path_buf()];
    }

    vec![system_path(), user_path()]
}

/// Loads config from `path` (error if missing).
pub fn load(path: &Path) -> Result<Config, ConfigError> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        if e.kind() == io::ErrorKind::NotFound {
            ConfigError::NotFound {
                path: path.to_path_buf(),
            }
        } else {
            ConfigError::Io(e)
        }
    })?;
    parse(&content)
}

/// Loads config when the file exists; returns `None` if not found.
pub fn load_if_exists(path: &Path) -> Result<Option<Config>, ConfigError> {
    match load(path) {
        Ok(config) => Ok(Some(config)),
        Err(ConfigError::NotFound { .. }) => Ok(None),
        Err(err) => Err(err),
    }
}

/// Merges system and user config files over built-in defaults.
pub fn load_layered(override_path: Option<&Path>) -> Result<Config, ConfigError> {
    if let Some(path) = override_path {
        return load(path);
    }

    let mut value = config_as_value(&Config::default())?;

    for path in [system_path(), user_path()] {
        if let Some(layer) = load_if_exists(&path)? {
            merge_toml(&mut value, config_as_value(&layer)?);
        }
    }

    let config: Config = value_from_toml(value)?;
    config.validate()?;
    Ok(config)
}

fn config_as_value(config: &Config) -> Result<toml::Value, ConfigError> {
    let serialized = toml::to_string(config).map_err(|e| ConfigError::Validation(e.to_string()))?;
    toml::from_str(&serialized).map_err(ConfigError::TomlDeserialize)
}

fn value_from_toml(value: toml::Value) -> Result<Config, ConfigError> {
    let serialized = toml::to_string(&value).map_err(|e| ConfigError::Validation(e.to_string()))?;
    parse(&serialized)
}

fn merge_toml(base: &mut toml::Value, overlay: toml::Value) {
    match (base, overlay) {
        (toml::Value::Table(base), toml::Value::Table(overlay)) => {
            for (key, value) in overlay {
                match base.get_mut(&key) {
                    Some(existing) => merge_toml(existing, value),
                    None => {
                        base.insert(key, value);
                    }
                }
            }
        }
        (base_slot, overlay) => *base_slot = overlay,
    }
}

/// Parses config from a TOML string.
pub fn parse(s: &str) -> Result<Config, ConfigError> {
    let config: Config = toml::from_str(s)?;
    config.validate()?;
    Ok(config)
}

#[allow(clippy::derivable_impls)]
impl Default for Config {
    fn default() -> Self {
        Self {
            logging: Logging::default(),
            session: Session::default(),
            ui: Ui::default(),
            remember: Remember::default(),
            user_menu: UserMenu::default(),
            secrets: Secrets::default(),
            keybindings: Keybindings::default(),
            power: Power::default(),
        }
    }
}

impl Default for Session {
    fn default() -> Self {
        Self {
            cmd: None,
            env: Vec::new(),
            wayland_dirs: default_wayland_dirs(),
            x11_dirs: default_x11_dirs(),
            session_wrapper: None,
            xsession_wrapper: default_xsession_wrapper(),
            no_xsession_wrapper: false,
        }
    }
}

impl Default for Ui {
    fn default() -> Self {
        Self {
            width: default_width(),
            window_padding: 0,
            container_padding: default_container_padding(),
            prompt_padding: default_prompt_padding(),
            greet_align: GreetAlign::default(),
            show_time: false,
            time_format: None,
            issue: false,
            greeting: None,
        }
    }
}

impl Default for Keybindings {
    fn default() -> Self {
        Self {
            command: default_kb_command(),
            sessions: default_kb_sessions(),
            power: default_kb_power(),
        }
    }
}

impl Config {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.ui.issue && self.ui.greeting.is_some() {
            return Err(ConfigError::Validation(
                "only one of [ui].issue and [ui].greeting may be set".into(),
            ));
        }

        if self.remember.session && self.remember.user_session {
            return Err(ConfigError::Validation(
                "only one of [remember].session and [remember].user_session may be enabled".into(),
            ));
        }

        if self.remember.user_session && !self.remember.username {
            return Err(ConfigError::Validation(
                "[remember].user_session requires [remember].username".into(),
            ));
        }

        if self.user_menu.enabled && self.user_menu.min_uid >= self.user_menu.max_uid {
            return Err(ConfigError::Validation(format!(
                "[user_menu].min_uid ({}) must be less than [user_menu].max_uid ({})",
                self.user_menu.min_uid, self.user_menu.max_uid
            )));
        }

        if self.secrets.mask && self.secrets.mask_char.is_empty() {
            return Err(ConfigError::Validation(
                "[secrets].mask_char must have at least one character".into(),
            ));
        }

        let keys = [
            self.keybindings.command,
            self.keybindings.sessions,
            self.keybindings.power,
        ];
        if keys[0] == keys[1] || keys[1] == keys[2] || keys[0] == keys[2] {
            return Err(ConfigError::Validation(
                "[keybindings] command, sessions, and power must be distinct".into(),
            ));
        }

        for key in keys {
            if !(1..=12).contains(&key) {
                return Err(ConfigError::Validation(
                    "[keybindings] values must be between 1 and 12".into(),
                ));
            }
        }

        if let Some(format) = &self.ui.time_format {
            use chrono::format::{Item, StrftimeItems};
            if StrftimeItems::new(format).any(|item| item == Item::Error) {
                return Err(ConfigError::Validation(
                    "invalid strftime format in [ui].time_format".into(),
                ));
            }
        }

        for env in &self.session.env {
            if !env.contains('=') {
                return Err(ConfigError::Validation(format!(
                    "malformed environment variable in [session].env: '{env}'"
                )));
            }
        }

        Ok(())
    }
}
