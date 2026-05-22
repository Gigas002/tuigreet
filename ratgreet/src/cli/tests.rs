use std::path::PathBuf;

use clap::Parser;

use super::Cli;
use crate::{
    config,
    settings::{CliOverrides, DebugOverride},
};

#[test]
fn parses_minimal_flags() {
    let cli = Cli::try_parse_from(["ratgreet"]).unwrap();
    let overrides = CliOverrides::from(&cli);
    assert!(overrides.config.is_none());
    assert!(overrides.theme.is_none());
    assert!(overrides.debug.is_none());
}

#[test]
fn parses_config_and_theme_paths() {
    let cli = Cli::try_parse_from([
        "ratgreet",
        "--config",
        "/etc/ratgreet/config.toml",
        "--theme",
        "/etc/ratgreet/theme.toml",
    ])
    .unwrap();
    let overrides = CliOverrides::from(&cli);
    assert_eq!(
        overrides.config,
        Some(PathBuf::from("/etc/ratgreet/config.toml"))
    );
    assert_eq!(
        overrides.theme,
        Some(PathBuf::from("/etc/ratgreet/theme.toml"))
    );
}

#[test]
fn debug_flag_without_file_uses_default_log_path() {
    let cli = Cli::try_parse_from(["ratgreet", "--debug"]).unwrap();
    let overrides = CliOverrides::from(&cli);
    assert_eq!(
        overrides.debug,
        Some(DebugOverride::LogFile(config::DEFAULT_LOG_FILE.into()))
    );
}

#[test]
fn debug_flag_with_file_sets_path() {
    let cli = Cli::try_parse_from(["ratgreet", "-d", "/var/log/ratgreet.log"]).unwrap();
    let overrides = CliOverrides::from(&cli);
    assert_eq!(
        overrides.debug,
        Some(DebugOverride::LogFile("/var/log/ratgreet.log".into()))
    );
}
