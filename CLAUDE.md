# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

Courier is a Rust desktop application (egui/eframe GUI) for tracking Dota 2 game data.

## Commands

The toolchain requires **nightly** (the crates use `#![feature(error_generic_member_access)]`).

```sh
just app                              # cargo +nightly run --bin courier --all-features
just hub                              # run the courier-hub server
just sqlx                             # cargo sqlx prepare --workspace — refresh the offline cache
just release <app|hub> <platform>     # cross/cargo release build
cargo +nightly fmt --all              # run after every edit
```

Feature flags (defined on the `courier` bin, forwarded to `ui`):
- `cjk` — bundles a CJK font; `ui/build.rs` downloads NotoSansSC into `crates/ui/assets/fonts/` at build time (needs network on first build).
- `inspect` — enables the dev-only `InspectPanel` (system stats via `sysinfo`).

## Workspace-wide style rules

**These are not preferences. Each one is a rule with a mechanical check — before you say you are
done, run the self-review at the end of this section against your own diff.**

### S1 — SQL is always compile-time checked
Always use `sqlx::query!` / `query_as!` / `query_scalar!`. Never runtime `sqlx::query()` /
`query_scalar()`, in any crate, in any case — unless the SQL genuinely cannot be known at compile
time. After adding or changing queries/schema: apply migrations to the dev DB
(`cargo sqlx migrate run --source crates/storage/migrations`, `DATABASE_URL` comes from `.env`),
then `just sqlx` to refresh the offline `.sqlx` cache.

### S2 — No `.unwrap()` / `.expect()`
`unwrap_used` and `expect_used` are **deny** in the workspace `Cargo.toml`. Where genuinely
unavoidable, add `#[allow(clippy::unwrap_used, reason = "...")]` with a real justification (see
`configs/src/lib.rs`, `hub/src/rate_limit.rs`). `allow_attributes_without_reason` is also deny —
every `#[allow]` carries a `reason = "..."`.

### S3 — `let … else`, not a `match` that unwraps
If a `match`/`if let` on `Result`/`Option` exists **only** to unwrap the happy path and
log-then-return on the other, it is not a `match`. Use `let … else`. To keep the error in the log,
chain `.inspect_err(…)` first — that is the whole reason the pattern works. Furthermore, consider
whether it would be more appropriate to use `?` to propagate errors/none-case.

```rust
// ❌
let tracked = match storage.list_tracked_follows().await {
    Ok(tracked) => tracked,
    Err(e) => { warn!("failed: {e}"); return; }
};
// ✅
let Ok(tracked) = storage.list_tracked_follows().await.inspect_err(|e| warn!("failed: {e}")) else {
    return;
};
```

A `match` stays only when **every arm does real, different work** (e.g. `Ok(Some)` / `Ok(None)` /
`Err` each sending a different reply). Two arms that both just log at different levels are better as
`.inspect(…).inspect_err(…)`; a fallback value is `.unwrap_or_else(…)`, not a `match`.

### S4 — Iterator chains, not `for` loops
Reach for `map` / `filter` / `filter_map` / `collect` / `fold` / `extend` / `retain` / `partition`
first. A `for` loop needs a *must* reason — an `.await` in the body, sequential `egui` painting, or
a genuine simultaneous partition-and-group.

```rust
// ❌                                        // ✅
let mut map = HashMap::new();                rows.into_iter().map(|r| (r.id as i32, r.name)).collect()
for row in rows { map.insert(row.id, row.name); }

// ❌ filter-by-continue                     // ✅ filter in the chain
for m in &self.matches {                     let visible = self.matches.iter().filter(|m| …);
    if !keep { continue; }                   for m in visible { … }
```

Even when the body must stay a `for` (it awaits), hoist the *filtering* into the iterator rather
than nesting an `if` inside.

### S5 — Comments earn their place
A comment is for the **why** the code cannot state: a rate-limit rationale, a protocol quirk, an
ordering constraint. Never restate the next line. `// Toasts overlay` above `main.toasts.show(&ctx)`
and `// Dota 2 sub-section` above a Dota 2 label are pure noise — delete them.

### S6 — A function needs a reason to exist
Write a function only when it is **reused (2+ call sites)** or the block is **genuinely long**. A
single-use three-line wrapper is worse than the three lines inlined at its one call site: it costs a
name, a signature, and a jump. When inlining one, carry its doc comment down as a `//` note if the
rationale still matters.

### S7 — One top-level `Error` per crate
Each crate has one `Error` enum (`crates/*/src/error.rs`, or inline in `lib.rs`/`main.rs` for small
crates) plus `pub type Result<T, E = Error>`. That enum is *the* error type of the crate. Introduce
a nested error type only for a genuinely large sub-module, and only when it earns the indirection.
Do not add two variants that wrap the same source type for different call sites — give the one
variant a `purpose: String` context field instead.

A named variant earns its place when code **matches on it** (e.g. `hub::Error::Auth` → 401) or its
`display` carries context worth surfacing. For a **trivial, one-off failure nobody matches on**
(binding a socket, creating a dir, parsing a locale), don't mint a variant — reach for the enum's
catch-all `Whatever` variant via `.whatever_context("…")` / `.with_whatever_context(|_| …)` /
`whatever!(…)`:

```rust
#[snafu(whatever, display("{message}"))]
Whatever {
    message: String,
    #[snafu(source(from(Box<dyn std::error::Error + Send + Sync>, Some)))]
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
},
```

`app`, `i18n`, and `hub` carry this variant today. Add it to an enum the first time you'd otherwise
write a throwaway variant — not pre-emptively (an unused `Whatever` is just dead code). A crate whose
every variant is load-bearing (`plugin`, `storage`, `configs`) does not have one. UI-side background
tasks are a related case: they use bare `snafu::Whatever` via `whatever_context`, collapsed by the
`AsyncBridge` (see below).

### S8 — Smaller conventions
- `to_owned()` for `&str → String`. `to_string()` only when it really is a *to-string* conversion
  (`u32 → String`, `Display`).
- `info!()`, not `tracing::info!()` — import the macro. The exception is type positions, where the
  full path disambiguates: `foo: tokio::sync::Mutex<T>` beats `foo: Mutex<T>`.
- Turbofish over an annotation on the binding: `foo.try_deserialize::<T>()`, not
  `let bar: T = foo.try_deserialize()`.
- Visibility is deliberate for every field, struct, fn and module. `pub` needs a reason;
  `pub(crate)` or private is the default.
- Run `cargo +nightly fmt --all` after editing. No need to run tests unless asked.

### Self-review before finishing
Re-read your own diff and grep it for each of these:

| Check | Look for |
|---|---|
| S3 | `match` arms that are `Ok(x) => x` + log-and-return |
| S4 | `for ` — can it be a chain? does the body really `.await` or paint? |
| S5 | `//` comments that paraphrase the line under them |
| S6 | new `fn`s with exactly one call site |
| S7 | new error enums, or two variants wrapping the same `source` type |
| S8 | `tracing::` prefixes, `.to_string()` on a `&str`, stray `pub` |

## Architecture

Cargo workspace, edition 2024. Crates under `crates/`:

- **`app`** (binary `courier`) — thin entry point (`main.rs`). Creates the tokio multi-thread runtime, initializes `tracing`, and hands a `runtime::Handle` to `ui::App` via `eframe::run_native`. The app owns the runtime; UI code never touches async directly.
- **`ui`** — all egui rendering and app state. The bulk of the code lives here.
- **`configs`** — global config + persistence.
- **`i18n`** — Fluent-based localization.
- **`shared`** — domain models (`model/`: `friend`, `hero`, `item`, `matches`) and shared error types.
- **`storage`** — SQLite via `sqlx`. Owns the connection pool and migrations. See below.
- **`plugin`** — the network layer: one shared `reqwest`-based `Client` plus typed API clients (Steam, Stratz, OpenDota). See below.
- **`hub`** (binary `courier-hub`) — the hosted Telegram hub: axum HTTP API (`/link/*`, `/notify`, `/sync`, `/unlink`), teloxide dispatcher, and the offline-mode tracker. **Config is entirely its own** (`crates/hub/src/config.rs`) — it never touches `configs`, since a server has no bootstrap step or home directory: defaults, layered under `config/config.toml` relative to the binary (`COURIER_HUB_CONFIG` overrides the path), layered under `COURIER_HUB__SECTION__FIELD` env vars, which is what lets the hosted deployment ship secrets without a config file while self-hosters use the file (`crates/hub/config.example.toml`). Loaded once in `main` into `AppState.config` (with `AppState.client`, the one proxy-aware `plugin::Client`) — don't reach for a config global. Persists via `storage::HubStore`; errors are `snafu`-based (`crates/hub/src/error.rs`). `subscriber_id` is a monotonic ULID (shared `ulid::Generator` on `HubStore`, so ids sort by creation even within one millisecond). The link token and the subscriber secret are 256-bit CSPRNG credentials minted by `storage::token` and persisted **only as SHA-256 digests** — `/link/new` is the single moment either plaintext is disclosed, which is why the secret is minted at handshake start rather than at bind. Never add a code path that stores or returns a plaintext credential. Outbound quotas live in `crates/hub/src/rate_limit.rs` — one `governor` limiter per upstream, shared through `AppState.limits`; add a limiter there rather than sprinkling sleeps at call sites. Every long-running task (axum, teloxide dispatcher, offline tracker) shuts down off the single `tokio_util` `CancellationToken` created in `main`.

### UI state machine (`crates/ui/src/app.rs`)

`App` is the single `eframe::App`. Its `state: AppState` is a two-variant enum:
- `Setup(SetupScreen)` — shown on first launch (`configs::is_first_launch()`), collects app path + language, then transitions to `Main`.
- `Main(MainState)` — the normal app: a `Route` enum (Dashboard/Friends/Matches/Heroes/Items/Settings) selects which screen renders in the central panel, alongside the menu bar, sidebar, toasts, and exit modal.

Each screen is a struct in `crates/ui/src/screens/` implementing the `Screen` trait (`fn show(&mut self, ui)`). Screens own their own state. Screens that need to affect global app state return an action enum from `show()` instead of mutating globals directly — e.g. `SettingScreen::show` returns `Option<SettingsAction>` (`ThemeChanged`/`StoragePathChanged`/`Reset`) which `App::ui` then applies. Follow this action-return pattern when a screen needs to reach outside itself.

### Async bridge (`crates/ui/src/async_bridge.rs`)

UI code never awaits. `AsyncBridge` (cloneable) wraps the tokio handle + an `mpsc::UnboundedSender<TaskResult>`. Call `bridge.spawn(future)` to run background work; the future's `TaskResult` is sent back over the channel and `ctx.request_repaint()` wakes the UI. `App::ui` drains the receiver each frame (`self.rx.try_recv()`) and routes results through `handle_task_result`. Spawned futures return `TaskOutcome = Result<TaskResult, snafu::Whatever>`: attach context to `plugin`/`storage` errors with `whatever_context` then `?`, and the bridge collapses any `Err` into `TaskResult::TaskFailed(msg)`. **To add a background operation:** add a `TaskResult` variant, spawn via the bridge returning that variant, and handle it in `handle_task_result`. Toasts use the same channel pattern via `ToastSender`/`ToastEvent`.

### Config (`crates/configs/src/lib.rs`)

Two layers, both global `LazyLock<RwLock<…>>` singletons:
- **`Bootstrap`** (`~/Courier/bootstrap.toml`) — stores the foundational paths: `app_path` (where `config.toml` lives; `is_first_launch()` is true when unset) and `storage_path` (where the SQLite DB lives; defaults to `~/.courier/data`, independent of `app_path`). Each path has a `set_*` and a `migrate_*_path` (move existing contents to a new dir, then persist).
- **`AppConfig`** (`<app_path>/config.toml`) — the real settings tree (`version`, `general`, `appearance`, `tracking`, `games.dota2`, `friends`, `matches`, `network`, `notification`, `secrets`). It holds **no hub-server settings** — the hub is a separate deployment with its own config file; the app only stores `notification.telegram.hub_base_url`, which points at the official hub or a self-hosted one, and behaves identically either way. Built via the `config` crate, layering the TOML file over `COURIER_`-prefixed env vars. Every field is `#[serde(default)]` so partial files load. `network.proxy` (optional URL) is what `plugin::Client::new` uses for all outbound requests.

`secrets` (`SecretsConfig`) holds BYOK API credentials (`steam_web_api_key`, `stratz_api_token`, `opendota_api_key`) — all `Option<String>`, no defaults shipped. This is the home for any new secret; don't scatter keys into per-game sections.

Access pattern: `configs::read()` for a read guard, `configs::update(|cfg| …)` to mutate the in-memory copy, `configs::save()` to persist. Mutation and persistence are separate steps — `update` does not write to disk.

### Storage (`crates/storage/src/lib.rs`)

`Storage` wraps a `sqlx::SqlitePool` opened (create-if-missing) at `configs::storage_path()/courier.db`, running embedded migrations from `crates/storage/migrations/` via `sqlx::migrate!`. Opened in `main.rs` with `runtime.block_on(...)` and owned by `ui::App`; future background tasks get the pool from there. Use the compile-time-checked `query!` macros **always** (see the style rules above). **New tables go in a new numbered migration file** — never edit an applied one (migrations are incremental, `0001`–`0009`). Also maintain a single migration file mirroring the entire/all current db schema. Changing `storage_path` at runtime closes the pool, calls `configs::migrate_storage_path`, then reopens (`App::migrate_storage`); the pool must be closed before the DB file can move.

The hub's tables (`hub_*`-prefixed, migration `0009`) live in the same migration history: `storage::hub::HubStore` opens the hub's own `courier-hub.db` with the same migrator (each database simply leaves the other side's tables empty), which is what gives hub queries the same compile-time `query!` checks against the single dev schema.

### Plugin / network layer (`crates/plugin/src/`)

One shared `Client` (a single `reqwest::Client`, built proxy-aware from `NetworkConfig::proxy`) backs every API call. Two request paths:
- **REST** — implement the `Endpoint` trait (`url`, optional `query`/`method`/`body`, associated `Response: DeserializeOwned`) and call `client.execute(endpoint)`.
- **GraphQL** — `client.graphql::<Q>(url, token, vars)` runs a `graphql_client`-generated query through the same client; GraphQL replies are HTTP 200 even on failure, so the `errors` array is surfaced as `Error::GraphQl`.

Typed clients are obtained from the `Client` (`client.steam(key)`, `client.stratz(token)`, `client.opendota(api_key)`); each is a thin borrow holding its credential. OpenDota keys players by Steam32 `account_id` (`steam64 - 76561197960265728`). Errors are `snafu`-based (`crates/plugin/src/error.rs`).

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

