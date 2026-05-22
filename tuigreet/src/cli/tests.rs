use std::path::PathBuf;

use clap::Parser;

use super::Cli;
use crate::{
    config,
    settings::{CliOverrides, DebugOverride},
};

#[test]
fn parses_minimal_flags() {
    let cli = Cli::try_parse_from(["tuigreet"]).unwrap();
    let overrides = CliOverrides::from(&cli);
    assert!(overrides.config.is_none());
    assert!(overrides.theme.is_none());
    assert!(overrides.debug.is_none());
}

#[test]
fn parses_config_and_theme_paths() {
    let cli = Cli::try_parse_from([
        "tuigreet",
        "--config",
        "/etc/tuigreet/config.toml",
        "--theme",
        "/etc/tuigreet/theme.toml",
    ])
    .unwrap();
    let overrides = CliOverrides::from(&cli);
    assert_eq!(
        overrides.config,
        Some(PathBuf::from("/etc/tuigreet/config.toml"))
    );
    assert_eq!(
        overrides.theme,
        Some(PathBuf::from("/etc/tuigreet/theme.toml"))
    );
}

#[test]
fn debug_flag_without_file_uses_default_log_path() {
    let cli = Cli::try_parse_from(["tuigreet", "--debug"]).unwrap();
    let overrides = CliOverrides::from(&cli);
    assert_eq!(
        overrides.debug,
        Some(DebugOverride::LogFile(config::DEFAULT_LOG_FILE.into()))
    );
}

#[test]
fn debug_flag_with_file_sets_path() {
    let cli = Cli::try_parse_from(["tuigreet", "-d", "/var/log/tuigreet.log"]).unwrap();
    let overrides = CliOverrides::from(&cli);
    assert_eq!(
        overrides.debug,
        Some(DebugOverride::LogFile("/var/log/tuigreet.log".into()))
    );
}
