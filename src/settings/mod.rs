//! Merged runtime settings: CLI overrides > config/theme files > built-in defaults.
#![allow(dead_code)] // Wired into `main` / Greeter in Phase 2.

use std::path::{Path, PathBuf};

use crate::{
    config::{self, Config, ConfigError, GreetAlign, LogLevel},
    theme::{self, ThemeError},
    ui::common::style::Theme,
};

#[cfg(test)]
mod tests;

/// CLI overrides (Phase 2 will populate this from `clap`).
#[derive(Debug, Default, Clone)]
pub struct CliOverrides {
    pub config: Option<PathBuf>,
    pub theme: Option<PathBuf>,
    pub debug: Option<DebugOverride>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebugOverride {
    Enabled,
    LogFile(String),
}

#[derive(Clone)]
pub struct Settings {
    pub logging: LoggingSettings,
    pub session: SessionSettings,
    pub ui: UiSettings,
    pub remember: RememberSettings,
    pub user_menu: UserMenuSettings,
    pub secrets: SecretsSettings,
    pub keybindings: KeybindingsSettings,
    pub power: PowerSettings,
    pub theme: Theme,
    pub config_path: Option<PathBuf>,
    pub theme_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct LoggingSettings {
    pub level: LogLevel,
    pub file: String,
    pub debug: bool,
}

#[derive(Debug, Clone)]
pub struct SessionSettings {
    pub cmd: Option<String>,
    pub env: Vec<String>,
    pub wayland_dirs: Vec<PathBuf>,
    pub x11_dirs: Vec<PathBuf>,
    pub session_wrapper: Option<String>,
    pub xsession_wrapper: Option<String>,
    pub session_paths: Vec<(PathBuf, SessionKind)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionKind {
    Wayland,
    X11,
}

#[derive(Debug, Clone)]
pub struct UiSettings {
    pub width: u16,
    pub window_padding: u16,
    pub container_padding: u16,
    pub prompt_padding: u16,
    pub greet_align: GreetAlign,
    pub show_time: bool,
    pub time_format: Option<String>,
    pub issue: bool,
    pub greeting: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RememberSettings {
    pub username: bool,
    pub session: bool,
    pub user_session: bool,
}

#[derive(Debug, Clone)]
pub struct UserMenuSettings {
    pub enabled: bool,
    pub min_uid: u16,
    pub max_uid: u16,
}

#[derive(Debug, Clone)]
pub struct SecretsSettings {
    pub mask: bool,
    pub mask_char: String,
}

#[derive(Debug, Clone)]
pub struct KeybindingsSettings {
    pub command: u8,
    pub sessions: u8,
    pub power: u8,
}

#[derive(Debug, Clone)]
pub struct PowerSettings {
    pub shutdown: Option<String>,
    pub reboot: Option<String>,
    pub setsid: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("{0}")]
    Config(#[from] ConfigError),

    #[error("{0}")]
    Theme(#[from] ThemeError),
}

impl Settings {
    pub fn load(cli: &CliOverrides) -> Result<Self, SettingsError> {
        let config = config::load_layered(cli.config.as_deref())?;
        let theme_file = theme::load_layered(cli.theme.as_deref())?;
        let theme = theme_file.to_ui_theme()?;

        let config_path = resolved_config_path(cli.config.as_deref());
        let theme_path = resolved_theme_path(cli.theme.as_deref());

        let mut settings = Self::from_config(config, theme, config_path, theme_path);
        settings.apply_cli(cli);
        Ok(settings)
    }

    fn from_config(
        config: Config,
        theme: Theme,
        config_path: Option<PathBuf>,
        theme_path: Option<PathBuf>,
    ) -> Self {
        let mut session_paths = Vec::new();
        for dir in &config.session.wayland_dirs {
            session_paths.push((dir.clone(), SessionKind::Wayland));
        }
        for dir in &config.session.x11_dirs {
            session_paths.push((dir.clone(), SessionKind::X11));
        }

        let xsession_wrapper = if config.session.no_xsession_wrapper {
            None
        } else {
            Some(config.session.xsession_wrapper.clone())
        };

        Self {
            logging: LoggingSettings {
                level: config.logging.level,
                file: config
                    .logging
                    .file
                    .unwrap_or_else(|| config::DEFAULT_LOG_FILE.to_string()),
                debug: false,
            },
            session: SessionSettings {
                cmd: config.session.cmd,
                env: config.session.env,
                wayland_dirs: config.session.wayland_dirs,
                x11_dirs: config.session.x11_dirs,
                session_wrapper: config.session.session_wrapper,
                xsession_wrapper,
                session_paths,
            },
            ui: UiSettings {
                width: config.ui.width,
                window_padding: config.ui.window_padding,
                container_padding: config.ui.container_padding,
                prompt_padding: config.ui.prompt_padding,
                greet_align: config.ui.greet_align,
                show_time: config.ui.show_time,
                time_format: config.ui.time_format,
                issue: config.ui.issue,
                greeting: config.ui.greeting,
            },
            remember: RememberSettings {
                username: config.remember.username,
                session: config.remember.session,
                user_session: config.remember.user_session,
            },
            user_menu: UserMenuSettings {
                enabled: config.user_menu.enabled,
                min_uid: config.user_menu.min_uid,
                max_uid: config.user_menu.max_uid,
            },
            secrets: SecretsSettings {
                mask: config.secrets.mask,
                mask_char: config.secrets.mask_char,
            },
            keybindings: KeybindingsSettings {
                command: config.keybindings.command,
                sessions: config.keybindings.sessions,
                power: config.keybindings.power,
            },
            power: PowerSettings {
                shutdown: config.power.shutdown,
                reboot: config.power.reboot,
                setsid: !config.power.no_setsid,
            },
            theme,
            config_path,
            theme_path,
        }
    }

    fn apply_cli(&mut self, cli: &CliOverrides) {
        if let Some(debug) = &cli.debug {
            self.logging.debug = true;
            if let DebugOverride::LogFile(path) = debug {
                self.logging.file = path.clone();
            }
        }
    }
}

fn resolved_config_path(override_path: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = override_path {
        return Some(path.to_path_buf());
    }

    config::resolved_paths(None)
        .into_iter()
        .find(|path| path.exists())
}

fn resolved_theme_path(override_path: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = override_path {
        return Some(path.to_path_buf());
    }

    theme::resolved_paths(None)
        .into_iter()
        .find(|path| path.exists())
}
