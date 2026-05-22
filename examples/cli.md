# ratgreet command line

The binary exposes only flags operators need at startup. Everything else lives in
[`config.toml`](config.toml) and [`theme.toml`](theme.toml).

## Options

| Short | Long | Description |
|-------|------|-------------|
| `-h` | `--help` | Show usage and exit |
| `-V` | `--version` | Print version and exit |
| | `--config PATH` | Load config from `PATH` only (see resolution below) |
| | `--theme PATH` | Load theme from `PATH` only |
| `-d` | `--debug [FILE]` | Enable tracing; optional log file (default `/tmp/ratgreet.log` or `[logging].file`) |

There are no subcommands.

## File resolution

**Without** `--config` / `--theme`, ratgreet merges layers in order (later wins):

1. Built-in defaults
2. `/etc/ratgreet/config.toml` or `theme.toml`
3. `$XDG_CONFIG_HOME/ratgreet/` (or `~/.config/ratgreet/`)

**With** `--config PATH` or `--theme PATH`, only that file is loaded (no `/etc` or XDG merge).

If a chosen file is missing, unreadable, invalid TOML, or empty, that layer is skipped and
defaults apply. The greeter still starts.

## Examples

```bash
# Production: install TOML under /etc/ratgreet/, no flags
ratgreet

# Try the shipped examples from a git checkout
ratgreet --config ./examples/config.toml --theme ./examples/theme.toml

# Debug logging to a file
ratgreet --debug /tmp/ratgreet.log
```

## greetd

greetd should invoke the binary with minimal arguments. Put session command and UI options
in ratgreet config, not on the greetd command line.

greetd (`/etc/greetd/config.toml`):

```toml
[terminal]
vt = 1

[default_session]
command = "ratgreet"
user = "greeter"
```

ratgreet (`/etc/ratgreet/config.toml`):

```toml
[session]
cmd = "sway"
```

See [greetd’s documentation](https://man.sr.ht/~kennylevinsen/greetd/).

## Migrating from tuigreet

The project was renamed **tuigreet → ratgreet**; update greetd `command`, config paths, and packager units accordingly.

Legacy long options (`--cmd`, `--width`, `--theme container=blue;…`, etc.) were removed.
See [`CHANGELOG.md`](../CHANGELOG.md) for the CLI → TOML mapping table.
