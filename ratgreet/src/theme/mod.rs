//! `theme.toml` types, path resolution, and conversion to UI `Theme`.
#![allow(dead_code)] // Consumed by `settings` and Greeter in Phase 2.

use std::{
    io,
    path::{Path, PathBuf},
};

use ratatui::style::Color;
use serde::{Deserialize, Serialize};

use crate::{color::parse_css_color, ui::common::style::Theme};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThemeFile {
    #[serde(default)]
    pub colors: ThemeColors,
    #[serde(default)]
    pub ui: ThemeUi,
}

/// Layout, banner, and clock (visual presentation — lives in `theme.toml`, not `config.toml`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeUi {
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

fn default_width() -> u16 {
    80
}

fn default_container_padding() -> u16 {
    1
}

fn default_prompt_padding() -> u16 {
    1
}

impl Default for ThemeUi {
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThemeColors {
    pub container: Option<String>,
    pub time: Option<String>,
    pub text: Option<String>,
    pub border: Option<String>,
    pub title: Option<String>,
    pub greet: Option<String>,
    pub prompt: Option<String>,
    pub input: Option<String>,
    pub action: Option<String>,
    pub button: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ThemeError {
    #[error("theme not found at {path}")]
    NotFound { path: PathBuf },

    #[error("IO: {0}")]
    Io(#[from] io::Error),

    #[error("TOML deserialize: {0}")]
    TomlDeserialize(#[from] toml::de::Error),

    #[error("invalid color '{name}': {value}")]
    InvalidColor { name: String, value: String },

    #[error("{0}")]
    Validation(String),
}

pub fn system_path() -> PathBuf {
    PathBuf::from("/etc/ratgreet/theme.toml")
}

pub fn user_path() -> PathBuf {
    theme_base_dir().join("theme.toml")
}

fn theme_base_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from("/"))
        .join("ratgreet")
}

pub fn resolved_paths(override_path: Option<&Path>) -> Vec<PathBuf> {
    if let Some(path) = override_path {
        return vec![path.to_path_buf()];
    }

    vec![system_path(), user_path()]
}

pub fn load(path: &Path) -> Result<ThemeFile, ThemeError> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        if e.kind() == io::ErrorKind::NotFound {
            ThemeError::NotFound {
                path: path.to_path_buf(),
            }
        } else {
            ThemeError::Io(e)
        }
    })?;
    parse(&content)
}

pub fn load_if_exists(path: &Path) -> Result<Option<ThemeFile>, ThemeError> {
    match load(path) {
        Ok(theme) => Ok(Some(theme)),
        Err(ThemeError::NotFound { .. }) => Ok(None),
        Err(err) => Err(err),
    }
}

/// Merges theme file(s) over built-in defaults.
///
/// Missing, unreadable, invalid, or empty files are skipped with a warning; the
/// result is always a valid [`ThemeFile`] (built-in defaults at minimum).
pub fn load_layered(override_path: Option<&Path>) -> ThemeFile {
    let Ok(mut value) = theme_as_value(&ThemeFile::default()) else {
        tracing::warn!("failed to serialize default theme; using struct defaults");
        return ThemeFile::default();
    };

    let paths: Vec<PathBuf> = match override_path {
        Some(path) => vec![path.to_path_buf()],
        None => vec![system_path(), user_path()],
    };

    for path in paths {
        merge_theme_layer(&mut value, &path);
    }

    value_from_toml(value)
        .ok()
        .filter(|theme| theme.validate().is_ok())
        .unwrap_or_else(|| {
            tracing::warn!("merged theme is invalid; using built-in defaults");
            ThemeFile::default()
        })
}

fn merge_theme_layer(base: &mut toml::Value, path: &Path) {
    match load_if_exists(path) {
        Ok(Some(layer)) => match theme_as_value(&layer) {
            Ok(overlay) => merge_toml(base, overlay),
            Err(err) => {
                tracing::warn!(path = %path.display(), "ignoring theme layer: {err}");
            }
        },
        Ok(None) => {}
        Err(err) => tracing::warn!(path = %path.display(), "ignoring theme layer: {err}"),
    }
}

fn theme_as_value(theme: &ThemeFile) -> Result<toml::Value, ThemeError> {
    let serialized = toml::to_string(theme).map_err(|e| ThemeError::InvalidColor {
        name: "serialize".into(),
        value: e.to_string(),
    })?;
    toml::from_str(&serialized).map_err(ThemeError::TomlDeserialize)
}

fn value_from_toml(value: toml::Value) -> Result<ThemeFile, ThemeError> {
    let serialized = toml::to_string(&value).map_err(|e| ThemeError::InvalidColor {
        name: "serialize".into(),
        value: e.to_string(),
    })?;
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

pub fn parse(s: &str) -> Result<ThemeFile, ThemeError> {
    let theme: ThemeFile = toml::from_str(s)?;
    theme.validate()?;
    Ok(theme)
}

impl ThemeFile {
    pub fn validate(&self) -> Result<(), ThemeError> {
        if self.ui.issue && self.ui.greeting.is_some() {
            return Err(ThemeError::Validation(
                "only one of [ui].issue and [ui].greeting may be set".into(),
            ));
        }

        if let Some(format) = &self.ui.time_format {
            use chrono::format::{Item, StrftimeItems};
            if StrftimeItems::new(format).any(|item| item == Item::Error) {
                return Err(ThemeError::Validation(
                    "invalid strftime format in [ui].time_format".into(),
                ));
            }
        }

        let fields = [
            ("container", &self.colors.container),
            ("time", &self.colors.time),
            ("text", &self.colors.text),
            ("border", &self.colors.border),
            ("title", &self.colors.title),
            ("greet", &self.colors.greet),
            ("prompt", &self.colors.prompt),
            ("input", &self.colors.input),
            ("action", &self.colors.action),
            ("button", &self.colors.button),
        ];

        for (name, value) in fields {
            if let Some(value) = value {
                parse_color(name, value)?;
            }
        }

        Ok(())
    }

    /// Builds a UI [`Theme`] from semantic color roles (same inheritance as inline `--theme`).
    pub fn to_ui_theme(&self) -> Result<Theme, ThemeError> {
        let mut spec = String::new();

        for (key, value) in [
            ("container", &self.colors.container),
            ("time", &self.colors.time),
            ("text", &self.colors.text),
            ("border", &self.colors.border),
            ("title", &self.colors.title),
            ("greet", &self.colors.greet),
            ("prompt", &self.colors.prompt),
            ("input", &self.colors.input),
            ("action", &self.colors.action),
            ("button", &self.colors.button),
        ] {
            if let Some(value) = value {
                if !spec.is_empty() {
                    spec.push(';');
                }
                spec.push_str(key);
                spec.push('=');
                spec.push_str(value);
            }
        }

        Ok(Theme::parse(&spec))
    }
}

fn parse_color(name: &str, value: &str) -> Result<Color, ThemeError> {
    parse_css_color(value).map_err(|_| ThemeError::InvalidColor {
        name: name.to_string(),
        value: value.to_string(),
    })
}
