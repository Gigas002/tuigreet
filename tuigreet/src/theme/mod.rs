//! `theme.toml` types, path resolution, and conversion to UI `Theme`.
#![allow(dead_code)] // Consumed by `settings` and Greeter in Phase 2.

use std::{
    io,
    path::{Path, PathBuf},
    str::FromStr,
};

use serde::{Deserialize, Serialize};
use tui::style::Color;

use crate::ui::common::style::Theme;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThemeFile {
    #[serde(default)]
    pub colors: ThemeColors,
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
}

pub fn system_path() -> PathBuf {
    PathBuf::from("/etc/tuigreet/theme.toml")
}

pub fn user_path() -> PathBuf {
    theme_base_dir().join("theme.toml")
}

fn theme_base_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from("/"))
        .join("tuigreet")
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

pub fn load_layered(override_path: Option<&Path>) -> Result<ThemeFile, ThemeError> {
    if let Some(path) = override_path {
        return load(path);
    }

    let mut value = theme_as_value(&ThemeFile::default())?;

    for path in [system_path(), user_path()] {
        if let Some(layer) = load_if_exists(&path)? {
            merge_toml(&mut value, theme_as_value(&layer)?);
        }
    }

    let theme: ThemeFile = value_from_toml(value)?;
    theme.validate()?;
    Ok(theme)
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
    Color::from_str(value).map_err(|_| ThemeError::InvalidColor {
        name: name.to_string(),
        value: value.to_string(),
    })
}
