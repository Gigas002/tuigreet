//! Minimal command-line interface (`--help`, `--version`, `--config`, `--theme`, `--debug`).

use std::path::PathBuf;

use clap::Parser;

use crate::settings::{CliOverrides, DebugOverride};

#[cfg(test)]
mod tests;

/// tuigreet — greetd TUI greeter.
#[derive(Debug, Parser)]
#[command(name = "tuigreet", about = "Terminal greeter for greetd", version)]
pub struct Cli {
    /// Load `config.toml` from `PATH` (overrides XDG/`/etc` search).
    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,

    /// Load `theme.toml` from `PATH`.
    #[arg(long, value_name = "PATH")]
    pub theme: Option<PathBuf>,

    /// Enable tracing; optional log file (default from config or `/tmp/tuigreet.log`).
    #[arg(
        short = 'd',
        long = "debug",
        value_name = "FILE",
        num_args = 0..=1,
        default_missing_value = crate::config::DEFAULT_LOG_FILE
    )]
    pub debug: Option<Option<String>>,
}

impl From<&Cli> for CliOverrides {
    fn from(cli: &Cli) -> Self {
        let debug = cli.debug.as_ref().map(|file| {
            file.as_ref()
                .map(|path| DebugOverride::LogFile(path.clone()))
                .unwrap_or(DebugOverride::Enabled)
        });

        CliOverrides {
            config: cli.config.clone(),
            theme: cli.theme.clone(),
            debug,
        }
    }
}
