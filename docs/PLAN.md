# ratgreet — Rust architecture + implementation plan

This document is both a **human roadmap** and an **agent playbook**: each step is sized for a focused implementation session, ends in a **verified** state (**build** + **fmt/clippy** + **tests** where applicable), and defines **how to verify** it.

It is modeled after the execution discipline in `docs/WAU_RS_PLAN.md` (wau workspace), adapted for a **greetd TUI greeter** — not an addon manager.

Reference plan:

- `docs/WAU_RS_PLAN.md` — quality gates, module/test layout, CI blueprint, dependency policy, phased verification

**Example artifacts** (schemas + minimal CLI) live under `examples/`:

- `examples/config.toml` — greetd paths, sessions, layout, power, keybindings, logging
- `examples/theme.toml` — layout, banner, clock, and colors
- `examples/cli.md` — retained CLI surface (help, version, config/theme paths, debug)

---

## 1. Goals and constraints

### 1.1 Goals

- **Workspace layout (wau-style)**: repository root is a **Cargo workspace** with **three members** — same pattern as wau’s `libwau` + `wau` + tests, adapted for a greeter:
  - **`libratgreet/`** — library crate: greeter, IPC, config/theme/settings, UI, and all unit tests (`src/<module>/tests.rs`). **No** `main`, **no** greetd-stub harness under `src/`.
  - **`ratgreet/`** — **thin binary** crate: `main.rs` (tracing, settings, event loop) only; depends on `libratgreet`. This is what packagers ship.
  - **`tests/`** — **`ratgreet-tests`** package: greetd-stub integration tests only; depends on **`libratgreet`** (not on in-process hacks inside the binary crate). **Not** released to users.
  - Splitting **`libratgreet`** is required so integration tests can live outside `ratgreet/src/` without a misnamed `src/integration/` tree — today’s harness calls `Greeter`, `Events`, and the test backend in-process.
- **Configuration-first UX**:
  - **`config.toml`**: everything operators configure today via long CLI flags (sessions, layout, power commands, keybindings, default command, wrappers, logging).
  - **`theme.toml`**: visual styling only (replaces `--theme` semicolon string).
  - **Minimal CLI**: `--help`, `--version`, `--config`, `--theme`, optional `--debug` — no duplicate knobs on the command line.
- **English-only UI**: drop Fluent/i18n embedding; user-visible strings live in one small module (constants or a tiny `strings` table).
- **Version without `build.rs`**: print `CARGO_PKG_VERSION` (+ optional `CARGO_PKG_VERSION_PRE` / git metadata via `vergen` only if we later need it — default is **no** build script).
- **Modern toolchain**: Rust **edition 2024**, dependency policy per §2.1, **`rustix`** instead of **`nix`** for `uname` and any future Unix syscalls.
- **Platform target**: Linux (greetd). No Windows/macOS scope in this overhaul.

### 1.2 Discipline (non-negotiable)

- **Core-first modules**: greetd IPC, session resolution, config merge, and “what to run after login” live under `libratgreet/src/{ipc,info,config,…}`; `libratgreet/src/ui/` draws widgets; **`ratgreet/src/main.rs`** only wires the loop. Avoid circular deps (UI imports core; core does not import UI).
- **Step sizing**: small, verifiable steps; each phase ends green on §7 quality gates.
- **Stay slim**: focused modules; prefer directory modules (`mod.rs` + `tests.rs`) over large monolithic files.
- **Naming**: short, descriptive; optimize for readers.

### 1.3 Non-goals

- GUI outside the terminal, tray apps, or non-greetd display managers.
- Multi-language UI in the first overhaul pass (no `i18n-embed`).
- Maintaining a `contrib/` tree (locales, fixtures, man page, screenshots, helper scripts).
- Optional `nsswrapper` Cargo feature and NSS-wrapper–based tests.
- Preserving every historical CLI flag for backward compatibility — document migration in `CHANGELOG.md` and `examples/config.toml`.
- Shipping **`libratgreet`** as a standalone product (deploy workflow ships only the **`ratgreet`** binary from the `ratgreet/` crate).

### 1.4 Definitions

- **Greeter**: the authentication + session-selection front-end talking to greetd over IPC.
- **Config**: machine-local `config.toml` (paths, behavior, layout, power, logging defaults).
- **Theme**: `theme.toml` mapping semantic roles (`container`, `title`, `prompt`, …) to colors.
- **Settings**: merged runtime view (`cli` overrides > config file > built-in defaults); after construction, downstream code uses **`Settings` only**, not raw `clap` or parsed TOML.

### 1.5 Compatibility reference

Behavior and UX should remain recognizable to existing ratgreet users and greetd packagers:

- [greetd](https://git.sr.ht/~kennylevinsen/greetd) IPC semantics (`greetd_ipc`)
- Prior ratgreet README options → mapped into `config.toml` / `theme.toml` (see §6 migration table)

---

## 2. Repository layout (target)

Mirror **wau**’s workspace shape: config and docs at the **repo root**; each crate in its own directory. **Do not** keep `src/` at the repository root after the workspace migration.

```text
ratgreet/                     # repository root (workspace)
  Cargo.toml                  # members = ["libratgreet", "ratgreet", "tests"]
  Cargo.lock
  deny.toml
  .typos.toml
  .github/
  examples/
  docs/
  libratgreet/                # library (production + unit tests)
    Cargo.toml
    src/
      lib.rs
      config/
      theme/
      settings/
      greeter/
      ipc/
      info/
      power/
      event/
      keyboard/
      ui/
        strings.rs
        …
  ratgreet/                   # binary only (shipped artifact)
    Cargo.toml
    src/
      main.rs                 # slim: init tracing, Settings, run loop
  tests/                      # ratgreet-tests (integration only)
    Cargo.toml
    src/
      lib.rs
      common/
      auth.rs
      display.rs
      …
```

**Current vs target**: today the repo is still a **single package** at the root (`Cargo.toml` + `src/`, plus misnamed `src/integration/`). Phase 3 performs the workspace split and deletes `src/integration/`.

**Workspace `Cargo.toml` (sketch)**:

```toml
[workspace]
members = ["libratgreet", "ratgreet", "tests"]
resolver = "3"
```

**`ratgreet/Cargo.toml` (sketch)**:

```toml
[package]
name = "ratgreet"

[[bin]]
name = "ratgreet"
path = "src/main.rs"

[dependencies]
libratgreet = { path = "../libratgreet" }
```

**`tests/Cargo.toml` (sketch)** — depends on the **library**, not the binary crate:

```toml
[package]
name = "ratgreet-tests"
publish = false

[dependencies]
libratgreet = { path = "../libratgreet" }
greetd-stub = "…"
tempfile = "…"
# tokio, crossterm, ratatui, … as needed by the harness
```

No `contrib/` directory — packager docs live in README + `examples/`; screenshots (if any) under `docs/` or external URLs only.

### 2.0.1 Crate boundary rules (workspace)

- **`libratgreet/`** owns greetd IPC, session discovery (`info`), greeter state (`greeter` + `model`), keyboard, events, power — **no** TOML parsing, **no** ratatui drawing, **no** `main`.
- **`ratgreet/`** (library + binary) owns **CLI**, **config/theme TOML**, **settings merge**, **logger**, **app loop**, and **UI**; depends on `libratgreet` (mirror `wau` calling `libwau`).
- **`tests/`** (`ratgreet-tests`) owns **only** greetd-stub integration tests; depends on **`libratgreet`** so the harness can keep in-process `Greeter` / `IntegrationRunner` without living under `ratgreet/src/`.
- **`tests/`** is not published; CI runs `cargo test --workspace`.

Module boundary rules:

- **`libratgreet/src/{ipc,info,greeter,model,power,keyboard,event}/`** — core only; no `ratatui`, `toml`, or `clap`.
- **`ratgreet/src/{cli,config,theme,settings,logger,app,ui}/`** — operator config and TUI; `settings` builds `Greeter` from merged config; `app` runs the loop with `Theme` + `libratgreet::Greeter`.
- After `Settings` exists, downstream drawing uses **`Greeter`** + **`Theme`** (theme is not stored on `Greeter`).

### 2.0 Module + tests file policy (mandatory)

Same as wau §2.0: **tests never live in the same `.rs` file as production logic.**

```text
src/config/
  mod.rs
  tests.rs
```

```rust
// mod.rs
#[cfg(test)]
mod tests;
```

- **Unit tests**: sibling `tests.rs` per module under **`libratgreet/src/<module>/`**; run with `cargo test -p libratgreet`.
- **Greetd-stub / UI flow tests**: **`tests/`** workspace member (`ratgreet-tests`), importing **`libratgreet::…`**. The historical **`src/integration/`** tree is **misnamed test code** wired through `#[cfg(test)] mod integration` in `main.rs` — delete after migration.

**Migration (Phase 3)**:

1. Add root workspace `Cargo.toml` with `members = ["libratgreet", "ratgreet", "tests"]`.
2. Move production `src/**` (except entry glue) into **`libratgreet/src/`**; add `libratgreet/src/lib.rs` re-exports as needed.
3. Add **`ratgreet/src/main.rs`** thin binary depending on `libratgreet`.
4. Create **`tests/`** + move `src/integration/**` → `tests/src/**`; update `use` paths to `libratgreet::…`.
5. Delete **`src/integration/`** and **`mod integration`** from `main.rs`.

**Do not** add test-only trees under **`ratgreet/src/`** or **`libratgreet/src/`** after migration (only under **`tests/src/`**).

### 2.1 Toolchain and dependency policy

- **Rust edition**: `2024`.
- **Version requirements**: `x.y` or `x` in `Cargo.toml`; pin via committed `Cargo.lock`.
- **Dependency health**: widely adopted crates; avoid archived / inactive deps (~1 year rule from wau).
- **Replacements (overhaul)**:

| Remove / avoid                                             | Replacement / notes                                                                   |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| `nix`                                                      | `rustix` (`rustix::system::uname` for hostname)                                       |
| `getopts`                                                  | `clap` (minimal derive on binary only)                                                |
| `i18n-embed`, `i18n-embed-fl`, `rust-embed`, `unic-langid` | English strings module; delete `i18n.toml`                                            |
| `nsswrapper` feature + `contrib/fixtures/`                 | Remove feature from `Cargo.toml`; delete `contrib/`; rely on greetd-stub + unit tests |
| `build.rs` + `VERSION` env                                 | `CARGO_PKG_VERSION` in `--version`                                                    |
| `lazy_static`                                              | `std::sync::LazyLock` where still needed                                              |
| `chrono` `unstable-locales`                                | Plain `chrono` + fixed `en-US` formatting (no locale feature)                         |

- **Keep (evaluate versions at upgrade time)**: `greetd_ipc`, `ratatui`, `crossterm`, `tokio`, `uzers`, `utmp-rs`, `zeroize`, `tracing` ecosystem, `smart-default` (or derive `Default` manually over time).

### 2.2 Features and CI matrix

- **No optional Cargo features** after overhaul: drop `nsswrapper` and the `[features]` table (or leave `default = []` only with no extra flags).
- **CI** builds/tests the default crate only (no `--all-features` / `--no-default-features` matrix unless a new optional feature is added later).

---

## 3. Configuration files

Illustrative shapes: `examples/config.toml`, `examples/theme.toml`.

### 3.1 Config (`config.toml`)

Purpose: operator-facing behavior — everything that is not pure color styling.

Resolution order:

1. Built-in defaults (documented in `examples/config.toml`)
2. `/etc/ratgreet/config.toml` (packager) and/or `$XDG_CONFIG_HOME/ratgreet/config.toml`
3. `--config <path>` override

Suggested sections (names may adjust during implementation):

```toml
[logging]
level = "info"          # trace | debug | info | warn | error
file = "/tmp/ratgreet.log"  # optional file sink when debug enabled

[session]
cmd = "…"               # optional default command
env = ["KEY=VALUE"]
wayland_dirs = ["/usr/share/wayland-sessions"]
x11_dirs = ["/usr/share/xsessions"]
session_wrapper = "…"
xsession_wrapper = "startx /usr/bin/env"
no_xsession_wrapper = false

[secrets]
display = "masked"      # hidden | plain | masked (default; was --asterisks)
mask_char = "*"         # single character when display = "masked"

[keybindings]
command = 2
sessions = 3
power = 12

[power]
shutdown = "…"
reboot = "…"
no_setsid = false
```

### 3.2 Theme (`theme.toml`)

Purpose: layout, banner, clock (`[ui]`) and colors for `Themed::*` roles (`[colors]`). Replaces `--theme container=…;title=…` and former `[ui]` in config.

```toml
[ui]
width = 80
window_padding = 0
container_padding = 1
prompt_padding = 1
greet_align = "center"  # left | center | right
show_time = false
time_format = "%c"
issue = false           # mutually exclusive with greeting
greeting = "Welcome"

[colors]
container = "blue"
time = "white"
text = "white"
border = "cyan"
title = "cyan"
greet = "white"
prompt = "white"
input = "white"
action = "yellow"
button = "yellow"
```

Resolution: `--theme <path>` → else XDG/`/etc` search path → built-in default theme.

### 3.3 CLI (minimal)

Documented in `examples/cli.md`:

| Flag                   | Purpose                                          |
| ---------------------- | ------------------------------------------------ |
| `-h`, `--help`         | Usage                                            |
| `-v`, `--version`      | `CARGO_PKG_VERSION` (+ target triple optional)   |
| `--config PATH`        | Config file                                      |
| `--theme PATH`         | Theme file                                       |
| `-d`, `--debug [FILE]` | Enable tracing (file from arg or config default) |

**Removed from CLI** (config/theme only): session dirs, remember flags, padding, width, power commands, keybindings, `--issue`/`--greeting`, `--time`, user-menu bounds, `--theme` inline string, `--env`, wrappers, etc.

---

## 4. Quality gates

Whenever a phase/step is marked complete:

- `cargo fmt --check`
- `typos` (`.typos.toml`)
- `cargo deny check licenses` (`deny.toml`)
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo doc --workspace --no-deps` (`RUSTDOCFLAGS=-D warnings`)

### 4.1 Test discipline

- No inline `#[cfg(test)] mod tests { … }` inside implementation files — use sibling `tests.rs`.
- Greetd-stub integration tests live in the **`tests/`** member (`ratgreet-tests`), depending on **`libratgreet`**; use `greetd-stub` and `tempfile` only (no NSS wrapper, no `contrib/fixtures/`).
- Remove tests that only assert CLI flag parsing for deprecated flags; replace with config/theme parse tests under `libratgreet/src/config/`, `theme/`, `settings/`.

### 4.2 CI blueprint (ratgreet)

Workflows under `.github/workflows/` (names/paths **ratgreet**, not wau):

| Workflow         | Job                                                                                                   |
| ---------------- | ----------------------------------------------------------------------------------------------------- |
| `build.yml`      | `cargo build -p ratgreet --release` (workspace root; ship binary from `ratgreet/`)                   |
| `fmt-clippy.yml` | `cargo fmt --check`; `cargo clippy --workspace --all-targets`                                        |
| `test.yml`       | `cargo test --workspace` (`libratgreet` unit + `ratgreet-tests` integration)                          |
| `doc.yml`        | `cargo doc -p libratgreet -p ratgreet --no-deps`                                                      |
| `typos.yml`      | spelling                                                                                              |
| `deny.yml`       | license check                                                                                         |
| `deploy.yml`     | On `v*` tags: build `ratgreet` release binary, strip, tarball `ratgreet-$VERSION-x86_64-linux.tar.gz` |
| `dependabot.yml` | cargo + github-actions weekly                                                                         |

**Cleanup from wau copy-paste**: no `-p wau`, `-p libwau`, or codecov flags named `wau`/`libwau`.

---

## 5. Phased steps

### Phase 0 — CI + hygiene

- [x] Copy wau-style workflows into `.github/workflows/`
- [x] Rename paths/crates in workflows to **ratgreet** (single package)
- [x] Add `deny.toml`, `.typos.toml`, `dependabot.yml` tuned for this repo
- [x] Remove wau-only `examples/toc/`; add ratgreet `examples/*.toml` + `examples/cli.md`
- [x] Drop `build.rs`; version uses `CARGO_PKG_VERSION`
- [x] Remove `i18n.toml`, `contrib/locales/`, `src/ui/i18n.rs`, `i18n-embed*` deps; English strings in `src/ui/strings.rs`
- [x] Replace `nix` with `rustix` (uname); drop `lazy_static` / `chrono` locale features (build unblock)
- [x] Delete `contrib/` entirely (`fixtures/`, `man/`, screenshots, `git-version.sh`, any remaining locales); screenshots under `docs/images/`
- [x] Drop `nsswrapper` feature: remove `[features]` entry, `src/info.rs` nsswrapper tests, README nsswrapper instructions; simplify CI workflows (§4.2) to default-only builds

**Verify**: §4 gates on current tree; no `contrib/` paths referenced in code, README, or `.typos.toml`.

### Phase 1 — Module layout + config/theme schemas

- [x] Reorganize flat `src/*.rs` into directory modules per §2 (`greeter/`, `ipc/`, `info/`, …)
- [x] Add `src/config/`, `src/theme/`, `src/settings/` with parse/validate + path resolution
- [x] Implement `Settings` merge (cli > file > defaults)
- [x] `examples/config.toml` + `examples/theme.toml` match parser

**Verify**: unit tests in `src/config/tests.rs`, etc. (today at repo-root `src/`; after Phase 3, under `libratgreet/src/`).

### Phase 2 — Minimal CLI + migration

- [x] Replace `getopts` with `clap`; wire `--config`, `--theme`, `--debug`
- [x] Map old CLI flags → config keys in `CHANGELOG.md` migration table
- [x] Delete deprecated `Greeter::options()` / `getopts::Matches` surface

**Verify**: `ratgreet --help`; integration tests load config from temp files.

### Phase 3 — Workspace layout (`libratgreet` + `ratgreet` + `tests`), strings

- [x] **Workspace restructure (wau-style)**: root `Cargo.toml` with `members = ["libratgreet", "ratgreet", "tests"]`; split current `src/` into **`libratgreet/`** + thin **`ratgreet/`** binary; update CI, `deny.toml`, doc paths
- [x] **`libratgreet`**: move production modules + unit tests; `src/lib.rs` public API for binary and `ratgreet-tests`
- [x] **`ratgreet-tests`**: move **`src/integration/`** → **`tests/src/`**; `use libratgreet::…`; delete `mod integration` from binary `main.rs`
- [x] Replace `fl!()` / i18n with `libratgreet/src/ui/strings.rs`
- [x] Move any remaining inline tests in `libratgreet/src/ui/` to sibling `tests.rs`
- [x] Keep `libratgreet/src/ui/` free of config parsing; consume `Settings` / `Greeter` only

**Verify**: `cargo test --workspace` green; `cargo build -p ratgreet --release` produces the greeter binary; no `src/integration/` anywhere; `rg 'mod integration'` empty; `tests/` has no production modules.

### Phase 4 — Dependency upgrade

- [x] Edition 2024 in `Cargo.toml`
- [x] `nix` → `rustix` in `info` (hostname)
- [x] Bump `ratatui`, `crossterm`, `tokio`, `greetd_ipc`, etc. to current compatible versions
- [x] `cargo deny` + clippy clean under new deps

**Verify**: §4 on Linux CI.

### Phase 5 — Docs + release discipline

- [ ] README: config/theme paths, minimal CLI, greetd unit example (no `contrib/` screenshot paths — use `docs/` assets or hosted images)
- [ ] Remove stale wau references; keep `docs/WAU_RS_PLAN.md` as read-only reference
- [ ] Tag release when stable

**Verify**: packager can configure via `/etc/ratgreet/config.toml` only.

---

## 6. CLI → config migration (operator reference)

| Former CLI                                         | New location                       |
| -------------------------------------------------- | ---------------------------------- |
| `--cmd`, `--env`                                   | `[session]`                        |
| `--sessions`, `--xsessions`, wrappers              | `[session]`                        |
| `--width`, `*-padding`, `--greet-align`            | `theme.toml` `[ui]`                |
| `--issue`, `--greeting`, `--time`, `--time-format` | `theme.toml` `[ui]`                |
| `--remember*`                                      | removed (manual login every time)  |
| `--user-menu*`                                     | removed (manual username entry)    |
| `--asterisks*`                                     | `[secrets]`                        |
| `--theme`                                          | `theme.toml` / `--theme` path only |
| `--power-*`, `--kb-*`                              | `[power]`, `[keybindings]`         |
| `-d` / `--debug`                                   | `[logging]` + CLI `--debug`        |

---

## 7. Definition of done (overhaul v1)

- [ ] Workspace builds with Phase 0–4 complete (`libratgreet` + `ratgreet` + `ratgreet-tests`)
- [ ] greetd login, session pick, power menus work via **config.toml** + **theme.toml**
- [ ] No `build.rs`, no i18n crates, no `nix`, no `contrib/`, no `nsswrapper` feature
- [ ] CI green on push/PR (§4.2 workflows, ratgreet naming)
- [ ] README and examples document the new configuration model

---

## 8. Document maintenance

Update this plan when:

- config/theme schema changes
- major module layout changes
- CI workflow names or matrices change

When example shapes change, update `examples/*.toml` and `examples/cli.md` in the same PR.

### Revision history

| Date       | Change                                                                                                                                                     |
| ---------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 2026-05-22 | Initial ratgreet overhaul plan derived from `WAU_RS_PLAN.md`; CI rename, drop i18n/build.rs, config/theme-first, rustix migration, two-crate target layout |
| 2026-05-22 | Interim: single crate at repo root (Phase 1); Phase 3 splits into `libratgreet` + `ratgreet` + `tests`                                                     |
| 2026-05-22 | Phase 0 complete: CI hygiene, drop i18n/build.rs, English strings, rustix, quality gates green                                                             |
| 2026-05-22 | Plan: drop `nsswrapper` feature and remove `contrib/` entirely; CI default-only                                                                            |
| 2026-05-22 | Phase 0 complete: removed `contrib/`, `nsswrapper`, CI default-only matrix                                                                                 |
| 2026-05-22 | Phase 1 complete: directory modules, config/theme/settings parsers and merge tests                                                                         |
| 2026-05-22 | Plan: Phase 3 relocates misnamed `src/integration/` (test-only) to crate-root `tests/`                                                                     |
| 2026-05-22 | Plan: wau-style workspace — root `Cargo.toml`, `ratgreet/` package, `tests/` member crate for integration only                                               |
| 2026-05-22 | Plan: Phase 3 adds **`libratgreet`** + thin **`ratgreet`** binary + **`tests/`** so integration harness can link without `src/integration/`                |
| 2026-05-22 | Phase 2 complete: `clap` minimal CLI, `Settings` wired in `main`, removed `getopts` / `Greeter::options()`                                                 |
| 2026-05-22 | Phase 3 complete: workspace split `libratgreet` + `ratgreet` + `tests`; integration harness in `ratgreet-tests`; `test-harness` feature for stub runs   |
| 2026-05-22 | Phase 4 complete: edition 2024 workspace; rustix 1.x; ratatui 0.30 / crossterm 0.29 / toml 1.1 / thiserror 2; theme CSS hex colors; §4 gates green locally   |
| 2026-05-22 | Rename project **tuigreet → ratgreet** (`ratgreet`, `libratgreet`, `ratgreet-tests`; `/etc/ratgreet/` config paths)                                          |
