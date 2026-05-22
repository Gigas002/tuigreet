use std::path::PathBuf;

use super::*;
use crate::config::LogLevel;
use crate::theme::GreetAlign;

const EXAMPLE_CONFIG: &str = include_str!("../../../examples/config.toml");
const EXAMPLE_THEME: &str = include_str!("../../../examples/theme.toml");

#[test]
fn load_defaults_without_files() {
    let settings = Settings::load(&CliOverrides::default()).unwrap();
    assert_eq!(settings.ui.width, 80);
    assert_eq!(settings.keybindings.power, 12);
    assert!(!settings.logging.debug);
    assert!(!settings.ui.show_time);
}

#[test]
fn cli_debug_overrides_logging() {
    let cli = CliOverrides {
        debug: Some(DebugOverride::LogFile("/var/log/ratgreet.log".into())),
        ..CliOverrides::default()
    };
    let settings = Settings::load(&cli).unwrap();
    assert!(settings.logging.debug);
    assert_eq!(settings.logging.file, "/var/log/ratgreet.log");
}

#[test]
fn load_from_explicit_config_and_theme_paths() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    let theme_path = dir.path().join("theme.toml");
    std::fs::write(&config_path, EXAMPLE_CONFIG).unwrap();
    std::fs::write(&theme_path, EXAMPLE_THEME).unwrap();

    let cli = CliOverrides {
        config: Some(config_path),
        theme: Some(theme_path),
        ..CliOverrides::default()
    };

    let settings = Settings::load(&cli).unwrap();
    assert_eq!(settings.ui.width, 80);
    assert_eq!(settings.ui.greet_align, GreetAlign::Center);
    assert!(settings.ui.show_time);
    assert_eq!(settings.logging.level, LogLevel::Info);
    assert_eq!(
        settings.session.xsession_wrapper.as_deref(),
        Some(crate::config::DEFAULT_XSESSION_WRAPPER)
    );
}

#[test]
fn bad_explicit_config_path_falls_back_to_defaults() {
    let cli = CliOverrides {
        config: Some(PathBuf::from("/nonexistent/ratgreet/config.toml")),
        ..CliOverrides::default()
    };
    let settings = Settings::load(&cli).unwrap();
    assert_eq!(settings.ui.width, 80);
}

#[test]
fn bad_explicit_theme_path_falls_back_to_defaults() {
    let cli = CliOverrides {
        theme: Some(PathBuf::from("/nonexistent/ratgreet/theme.toml")),
        ..CliOverrides::default()
    };
    let settings = Settings::load(&cli).unwrap();
    assert_eq!(settings.ui.width, 80);
    assert!(!settings.ui.show_time);
}

#[test]
fn cli_theme_path_wins_over_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let theme_path = dir.path().join("theme.toml");
    std::fs::write(
        &theme_path,
        r#"
[ui]
width = 120
"#,
    )
    .unwrap();

    let cli = CliOverrides {
        theme: Some(theme_path.clone()),
        ..CliOverrides::default()
    };

    let settings = Settings::load(&cli).unwrap();
    assert_eq!(settings.ui.width, 120);
    assert_eq!(settings.theme_path, Some(theme_path));
}
