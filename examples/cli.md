# tuigreet CLI (minimal surface)

After the configuration overhaul, most options move to `config.toml` and `theme.toml`.
The binary keeps only entrypoints operators need on the command line.

## Commands

```text
tuigreet [OPTIONS]
```

There are no subcommands.

## Options

| Short | Long | Description |
|-------|------|-------------|
| `-h` | `--help` | Show usage and exit (provided by `clap`) |
| `-V` | `--version` | Print version (`CARGO_PKG_VERSION`) and exit (provided by `clap`) |
| | `--config PATH` | Load `config.toml` from `PATH` (overrides XDG/`/etc` search) |
| | `--theme PATH` | Load `theme.toml` from `PATH` |
| `-d` | `--debug [FILE]` | Enable tracing; optional log file (default from config or `/tmp/tuigreet.log`) |

## Examples

```bash
# Packager unit: config on disk, no flags
tuigreet

# Override config for testing
tuigreet --config ./examples/config.toml --theme ./examples/theme.toml

# Verbose run
tuigreet --debug /tmp/tuigreet.log
```

## Migration

See `docs/PLAN.md` §6 for mapping from legacy CLI flags to TOML keys.
