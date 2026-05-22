use super::*;
use crate::config::{GreetAlign, LogLevel};

const EXAMPLE_CONFIG: &str = include_str!("../../examples/config.toml");
const EXAMPLE_THEME: &str = include_str!("../../examples/theme.toml");

#[test]
fn load_defaults_without_files() {
    let settings = Settings::load(&CliOverrides::default()).unwrap();
    assert_eq!(settings.ui.width, 80);
    assert_eq!(settings.keybindings.power, 12);
    assert!(!settings.logging.debug);
}

#[test]
fn cli_debug_overrides_logging() {
    let cli = CliOverrides {
        debug: Some(DebugOverride::LogFile("/var/log/tuigreet.log".into())),
        ..CliOverrides::default()
    };
    let settings = Settings::load(&cli).unwrap();
    assert!(settings.logging.debug);
    assert_eq!(settings.logging.file, "/var/log/tuigreet.log");
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
    assert_eq!(settings.logging.level, LogLevel::Info);
    assert_eq!(
        settings.session.xsession_wrapper.as_deref(),
        Some(crate::config::DEFAULT_XSESSION_WRAPPER)
    );
}

#[test]
fn cli_config_path_wins_over_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    std::fs::write(
        &config_path,
        r#"
[ui]
width = 120
"#,
    )
    .unwrap();

    let cli = CliOverrides {
        config: Some(config_path.clone()),
        ..CliOverrides::default()
    };

    let settings = Settings::load(&cli).unwrap();
    assert_eq!(settings.ui.width, 120);
    assert_eq!(settings.config_path, Some(config_path));
}
