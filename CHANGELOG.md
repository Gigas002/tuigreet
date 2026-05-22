# Changelog

## Unreleased

### Changed

- CLI is minimal (`clap`): `--help`, `--version`, `--config`, `--theme`, `--debug` only. All former greeter flags live in `config.toml` / `theme.toml` (see migration table below).

- CI: wau-style workflows renamed for the `tuigreet` package (`build`, `test`, `fmt-clippy`, `doc`, `typos`, `deny`, `deploy`).
- Version output uses `CARGO_PKG_VERSION` (removed `build.rs` git script).
- UI strings are English-only (`src/ui/strings.rs`); removed Fluent/i18n embedding.
- `nix` replaced with `rustix` for `uname(2)`; `lazy_static` replaced with `std::sync::LazyLock` in `info.rs`.
- CI workflows use a single default build/test/clippy matrix (no optional feature flags).
- README screenshots live under `docs/images/`.

### Removed

- `build.rs`, `i18n.toml`, `examples/toc/` (wau leftovers).
- `contrib/` directory (locales, fixtures, man page, screenshots, helper scripts).
- `nsswrapper` Cargo feature and NSS-wrapper-based tests.
- `getopts` and the legacy long-option CLI surface (`Greeter::options()`, `parse_options()`).

### CLI → config migration

| Former CLI | New location |
| --- | --- |
| `--cmd`, `--env` | `[session]` |
| `--sessions`, `--xsessions`, `--session-wrapper`, `--xsession-wrapper`, `--no-xsession-wrapper` | `[session]` |
| `--width`, `--window-padding`, `--container-padding`, `--prompt-padding`, `--greet-align` | `[ui]` |
| `--issue`, `--greeting`, `--time`, `--time-format` | `[ui]` |
| `--remember`, `--remember-session`, `--remember-user-session` | `[remember]` |
| `--user-menu`, `--user-menu-min-uid`, `--user-menu-max-uid` | `[user_menu]` |
| `--asterisks`, `--asterisks-char` | `[secrets]` |
| `--theme` (inline colors) | `theme.toml` / `--theme PATH` |
| `--power-shutdown`, `--power-reboot`, `--power-no-setsid` | `[power]` |
| `--kb-command`, `--kb-sessions`, `--kb-power` | `[keybindings]` |
| `-d` / `--debug` | `[logging]` + CLI `--debug [FILE]` |
