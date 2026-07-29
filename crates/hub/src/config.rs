//! The hub's own configuration, deliberately independent of the desktop app's
//! `configs` crate: a server has no bootstrap step and no home directory to
//! discover paths from. Everything is resolved against the directory holding the
//! binary, so unpacking a release next to a `config/` directory is a complete
//! deployment.
//!
//! Layering, lowest to highest: struct defaults, `config/config.toml` (override
//! the whole path with `COURIER_HUB_CONFIG`), then `COURIER_HUB__`-prefixed
//! environment variables (`COURIER_HUB__TELEGRAM__TOKEN`, …). The env layer is
//! what makes a hosted deployment work without a config file at all: ship
//! secrets as environment variables and leave the file to the self-hosters.

use std::path::{Path, PathBuf};

use config::{Config, Environment};
use serde::{Deserialize, Serialize};
use snafu::ResultExt;

use crate::error::{ConfigSnafu, Result};

const CONFIG_PATH_ENV: &str = "COURIER_HUB_CONFIG";
const DEFAULT_CONFIG_PATH: &str = "config/config.toml";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HubConfig {
    pub server: ServerConfig,
    pub telegram: TelegramConfig,
    pub tracking: TrackingConfig,
    pub network: NetworkConfig,
    pub secrets: SecretsConfig,
}

impl HubConfig {
    pub fn load() -> Result<Self> {
        let root = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| PathBuf::from("."));
        let config_file = std::env::var(CONFIG_PATH_ENV).map_or_else(|_| root.join(DEFAULT_CONFIG_PATH), PathBuf::from);

        let mut config = Config::builder()
            .add_source(config::File::from(config_file).required(false))
            .add_source(Environment::with_prefix("HUB").separator("_").try_parsing(true))
            .build()
            .context(ConfigSnafu)?
            .try_deserialize::<Self>()
            .context(ConfigSnafu)?;

        // `join` with an absolute path replaces, so an absolute `data_path`
        // (a mounted volume, `/var/lib/courier-hub`) passes through untouched.
        config.server.data_path = root.join(&config.server.data_path);
        config.server.log_path = root.join(&config.server.log_path);
        Ok(config)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    /// HTTP bind address for the hub's API.
    pub bind: String,
    /// Directory holding the hub's SQLite database, relative to the binary.
    pub data_path: PathBuf,
    /// Directory the daily-rotated log files go into, relative to the binary.
    pub log_path: PathBuf,
    /// Early-access gate: when set, `/link/new` requires a matching invite code
    /// before minting a token, keeping the official hub invite-only while its
    /// resources are limited. `None` leaves the hub open, which is what a
    /// self-hoster running their own box wants.
    pub access_code: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind: "0.0.0.0:6969".to_owned(),
            data_path: PathBuf::from("data"),
            log_path: PathBuf::from("logs"),
            access_code: None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TelegramConfig {
    /// The bot token from @BotFather. The hub refuses to start without it.
    pub token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TrackingConfig {
    /// Offline-mode poll interval in minutes.
    pub poll_interval_mins: u32,
    /// How many recent matches to pull per tracked account each poll, so several
    /// games finished between polls are all caught.
    pub fetch_limit: usize,
}

impl Default for TrackingConfig {
    fn default() -> Self {
        Self {
            poll_interval_mins: 10,
            fetch_limit: 5,
        }
    }
}

/// `proxy` is an optional proxy URL (e.g. `http://127.0.0.1:7890`) applied to
/// every outbound request; `None` means a direct connection.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct NetworkConfig {
    pub proxy: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SecretsConfig {
    /// The key the hub polls OpenDota with in offline mode. Optional but
    /// strongly recommended; the keyless quota won't carry many subscribers.
    pub opendota_api_key: Option<String>,
}
