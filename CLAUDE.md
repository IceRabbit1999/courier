# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

Courier is a Rust desktop application (egui/eframe GUI) for tracking Dota 2 game data.

## Commands

The toolchain requires **nightly** (the crates use `#![feature(error_generic_member_access)]`).

```sh
just run                              # cargo +nightly run --bin courier --all-features
just fmt                              # cargo +nightly fmt --all  — run after every edit
```

Feature flags (defined on the `courier` bin, forwarded to `ui`):
- `cjk` — bundles a CJK font; `ui/build.rs` downloads NotoSansSC into `crates/ui/assets/fonts/` at build time (needs network on first build).
- `inspect` — enables the dev-only `InspectPanel` (system stats via `sysinfo`).

## Workspace-wide style rules

`Cargo.toml` sets these as **deny** clippy lints across all crates:
- `unwrap_used`, `expect_used` — never use `.unwrap()`/`.expect()`. Where genuinely unavoidable, add `#[allow(clippy::unwrap_used, reason = "...")]` with a justification (see existing call sites in `configs/src/lib.rs`).
- `allow_attributes_without_reason` — every `#[allow]` must carry a `reason = "..."`.
- Only comments when necessary. For example, a `// Settings` for a field `setting` is totally unnecessary and ugly.
- Prefer `to_owned()`, for example, than `to_string()` when trying to convert a `&str` to a `String`. Which is to say, `to_string()` should express exactly the "to_string" action, for example, convert an u32 to `String`
- Prefer `info!()` than `tracing::info!()`, but with one exception: ambiguity exists when declaring types, for example, `foo: tokio::sync::Mutex<T>` is better than `foo: Mutex<T>`. The complete namespace makes the intention clear.
- Prefer turbofish syntax than manual type inference. For example, `let bar = foo.try_deserialize::<T>()` is better than `let bar: T = foo.try_deserialize()`. 
- Carefully on visibility control for all fields, structs, functions, modules, etc. Use `pub` only when you have a good and enough reason.
- Always run `cargo +nightly fmt --all` to format the code after editing.
- No need to run any tests unless you are specifically asked to do so.

## Architecture

Cargo workspace, edition 2024. Crates under `crates/`:

- **`app`** (binary `courier`) — thin entry point (`main.rs`). Creates the tokio multi-thread runtime, initializes `tracing`, and hands a `runtime::Handle` to `ui::App` via `eframe::run_native`. The app owns the runtime; UI code never touches async directly.
- **`ui`** — all egui rendering and app state. The bulk of the code lives here.
- **`configs`** — global config + persistence.
- **`i18n`** — Fluent-based localization.
- **`shared`** — domain models (`models.rs`) and shared error types.
- **`storage`** — SQLite via `sqlx`. Owns the connection pool and migrations. See below.
- **`plugin`** — currently a placeholder/scaffold (default cargo template; a `plugin-core` crate was recently removed). Treat as WIP.

### UI state machine (`crates/ui/src/app.rs`)

`App` is the single `eframe::App`. Its `state: AppState` is a two-variant enum:
- `Setup(SetupScreen)` — shown on first launch (`configs::is_first_launch()`), collects app path + language, then transitions to `Main`.
- `Main(MainState)` — the normal app: a `Route` enum (Dashboard/Friends/Matches/Heroes/Items/Settings) selects which screen renders in the central panel, alongside the menu bar, sidebar, toasts, and exit modal.

Each screen is a struct in `crates/ui/src/screens/` implementing the `Screen` trait (`fn show(&mut self, ui)`). Screens own their own state. Screens that need to affect global app state return an action enum from `show()` instead of mutating globals directly — e.g. `SettingScreen::show` returns `Option<SettingsAction>` (`ThemeChanged`/`StoragePathChanged`/`Reset`) which `App::ui` then applies. Follow this action-return pattern when a screen needs to reach outside itself.

### Async bridge (`crates/ui/src/async_bridge.rs`)

UI code never awaits. `AsyncBridge` (cloneable) wraps the tokio handle + an `mpsc::UnboundedSender<TaskResult>`. Call `bridge.spawn(future)` to run background work; the future's `TaskResult` is sent back over the channel and `ctx.request_repaint()` wakes the UI. `App::ui` drains the receiver each frame (`self.rx.try_recv()`) and routes results through `handle_task_result`. **To add a background operation:** add a variant to the `TaskResult` enum, spawn via the bridge, and handle the variant in `handle_task_result`. (Both are currently stubs awaiting real network tasks.) Toasts use the same channel pattern via `ToastSender`/`ToastEvent`.

### Config (`crates/configs/src/lib.rs`)

Two layers, both global `LazyLock<RwLock<…>>` singletons:
- **`Bootstrap`** (`~/Courier/bootstrap.toml`) — stores the foundational paths: `app_path` (where `config.toml` lives; `is_first_launch()` is true when unset) and `storage_path` (where the SQLite DB lives; defaults to `~/.courier/data`, independent of `app_path`). Each path has a `set_*` and a `migrate_*_path` (move existing contents to a new dir, then persist).
- **`AppConfig`** (`<app_path>/config.toml`) — the real settings tree (`general`, `appearance`, `tracking`, `games.dota2`, `notification`, `secrets`). Built via the `config` crate, layering the TOML file over `COURIER_`-prefixed env vars. Every field is `#[serde(default)]` so partial files load.

`secrets` (`SecretsConfig`) holds BYOK API credentials (`steam_web_api_key`, `stratz_api_token`, `opendota_api_key`) — all `Option<String>`, no defaults shipped. This is the home for any new secret; don't scatter keys into per-game sections.

Access pattern: `configs::read()` for a read guard, `configs::update(|cfg| …)` to mutate the in-memory copy, `configs::save()` to persist. Mutation and persistence are separate steps — `update` does not write to disk.

### Storage (`crates/storage/src/lib.rs`)

`Storage` wraps a `sqlx::SqlitePool` opened (create-if-missing) at `configs::storage_path()/courier.db`, running embedded migrations from `crates/storage/migrations/` via `sqlx::migrate!`. Opened in `main.rs` with `runtime.block_on(...)` and owned by `ui::App`; future background tasks get the pool from there. Use the runtime query API (`sqlx::query(...)`), not the compile-time-checked `query!` macros, so there's no `DATABASE_URL` build dependency. **New tables go in a new numbered migration file** — never edit an applied one. Changing `storage_path` at runtime closes the pool, calls `configs::migrate_storage_path`, then reopens (`App::migrate_storage`); the pool must be closed before the DB file can move.

### i18n (`crates/i18n/src/lib.rs`)

Fluent (`.ftl`) bundles in `crates/i18n/locales/` (`en`, `zh-CN`), compiled in via `include_str!` in `loader.rs`. Global `LazyLock<RwLock<I18nState>>`. Call `i18n::message("key")` anywhere to look up a string; missing keys fall back to the default locale, then to the raw key. Adding a UI string means adding the key to **every** `.ftl` file. `init`/`set_locale` honor `language = "auto"` (system detection) vs. an explicit locale.

### Theme (`crates/ui/src/theme/`)

`CourierTheme` (owned by `App`) holds the mode (Light/Dark/Custom) and applies styling to the egui context each frame. Colors are exposed through a **global** `static COLORS: RwLock<ColorPalette>` in `theme/color.rs` — call `colors()` to read the active palette and `set_colors()` to swap it. Helper modules `spacing`, `radius`, `font_size`, `animation`, `sidebar` expose layout constants (several read from `AppConfig::appearance`).

## Guidelines

### 1. Think Before Coding

**Don't assume. Don't hide confusion. Surface tradeoffs.**

Before implementing:
- State your assumptions explicitly. If uncertain, ask.
- If multiple interpretations exist, present them - don't pick silently.
- If a simpler approach exists, say so. Push back when warranted.
- If something is unclear, stop. Name what's confusing. Ask.

### 2. Simplicity First

**Minimum code that solves the problem. Nothing speculative. Unless you are specifically asked to do so.**

- No features beyond what was asked.
- No abstractions for single-use code.
- No "flexibility" or "configurability" that wasn't requested.
- No error handling for impossible scenarios.
- If you write 200 lines and it could be 50, rewrite it.

Ask yourself: "Would a senior engineer say this is overcomplicated?" If yes, simplify.

### 3. Surgical Changes

**Touch only what you must. Clean up only your own mess.**

When editing existing code:
- Don't "improve" adjacent code, comments, or formatting.
- Don't refactor things that aren't broken.
- Match existing style, even if you'd do it differently.
- If you notice unrelated dead code, mention it - don't delete it.

When your changes create orphans:
- Remove imports/variables/functions that YOUR changes made unused.
- Don't remove pre-existing dead code unless asked.

The test: Every changed line should trace directly to the user's request.

### 4. Goal-Driven Execution

**Define success criteria. Loop until verified.**

Transform tasks into verifiable goals:
- "Add validation" → "Write tests for invalid inputs, then make them pass"
- "Fix the bug" → "Write a test that reproduces it, then make it pass"
- "Refactor X" → "Ensure tests pass before and after"

For multi-step tasks, state a brief plan:
```
1. [Step] → verify: [check]
2. [Step] → verify: [check]
3. [Step] → verify: [check]
```

## Notes

- `queued.md` tracks small deferred TODOs the maintainer wants to revisit.
- `config.toml` at the repo root is a sample/reference config, not the live one (the live file lives under the user's chosen `app_path`).

