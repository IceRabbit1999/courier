#![feature(error_generic_member_access)]

use std::{
    path::{Path, PathBuf},
    sync::LazyLock,
};

use config::{Config, Environment};
use directories::BaseDirs;
use parking_lot::{RwLock, RwLockReadGuard};
use serde::{Deserialize, Serialize};
use snafu::{OptionExt, ResultExt};
use tracing::{info, warn};

pub mod error;
pub use error::*;

fn bootstrap_dir() -> PathBuf {
    #[allow(clippy::expect_used, reason = "app cannot function without a home directory")]
    BaseDirs::new().map(|p| p.home_dir().join("Courier")).expect("Failed to get project dirs")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bootstrap {
    app_path: Option<PathBuf>,
    #[serde(skip)]
    bootstrap_file: PathBuf,
}

impl Bootstrap {
    const BOOTSTRAP_FILE: &str = "bootstrap.toml";
    const APP_CONFIG_FILE: &str = "config.toml";

    pub fn app_path(&self) -> PathBuf {
        // todo: verify safety: check all usages of this function, ensure it is always called after app_path
        // is initialized
        #[allow(clippy::unwrap_used, reason = "only called after app_path is set during setup")]
        self.app_path.clone().unwrap()
    }

    fn new() -> Self {
        let bootstrap_dir = bootstrap_dir();
        let bootstrap_file = bootstrap_dir.join(Self::BOOTSTRAP_FILE);

        let app_path = std::fs::read_to_string(&bootstrap_file)
            .ok()
            .and_then(|s| toml::from_str::<Bootstrap>(&s).ok())
            .map(|bc| bc.app_path)
            .unwrap_or_default();
        info!("Bootstrap initialized with app_path: {:?}", app_path);
        Self { app_path, bootstrap_file }
    }

    fn save(&self) -> Result<()> {
        let bootstrap_path = &self.bootstrap_file;
        let content = toml::to_string_pretty(self).context(TomlSerializeSnafu)?;
        #[allow(clippy::unwrap_used, reason = "bootstrap_file is constructed via .join() so parent always exists")]
        std::fs::create_dir_all(bootstrap_path.parent().unwrap()).context(ConfigSaveSnafu)?;
        std::fs::write(bootstrap_path, content).context(ConfigSaveSnafu)?;
        info!("Saved bootstrap: {:?}", self);
        Ok(())
    }

    fn config_file(&self) -> Option<PathBuf> {
        self.app_path.as_ref().map(|path| path.join(Self::APP_CONFIG_FILE))
    }
}

static BOOTSTRAP: LazyLock<RwLock<Bootstrap>> = LazyLock::new(|| RwLock::new(Bootstrap::new()));

static CONFIG: LazyLock<RwLock<AppConfig>> = LazyLock::new(|| {
    let bootstrap = BOOTSTRAP.read();
    RwLock::new(
        AppConfig::load(&bootstrap)
            .inspect_err(|e| warn!("Failed to load config: {e:?}, starting with default config"))
            .unwrap_or_default(),
    )
});

pub fn is_first_launch() -> bool {
    BOOTSTRAP.read().app_path.is_none()
}

pub fn bootstrap() -> RwLockReadGuard<'static, Bootstrap> {
    BOOTSTRAP.read()
}

pub fn default_app_path() -> PathBuf {
    directories::BaseDirs::new().map(|b| b.home_dir().join(".courier")).unwrap_or_else(|| PathBuf::from("."))
}

pub fn set_app_path(path: PathBuf) -> Result<()> {
    let mut bootstrap = BOOTSTRAP.write();
    bootstrap.app_path = Some(path);
    bootstrap.save()
}

pub fn migrate_app_path(new_path: PathBuf) -> Result<()> {
    let mut bootstrap = BOOTSTRAP.write();
    let old_path = bootstrap.app_path.clone().context(AppPathSnafu)?;

    if old_path == new_path {
        return Ok(());
    }

    if old_path.exists() {
        std::fs::create_dir_all(&new_path).context(MigrationSnafu)?;
        for entry in std::fs::read_dir(&old_path).context(MigrationSnafu)? {
            let entry = entry.context(MigrationSnafu)?;
            let dest = new_path.join(entry.file_name());
            copy_path(&entry.path(), &dest)?;
        }
    }

    bootstrap.app_path = Some(new_path);
    bootstrap.save()?;

    let _ = std::fs::remove_dir_all(&old_path);

    Ok(())
}

fn copy_path(src: &Path, dst: &Path) -> Result<()> {
    if src.is_dir() {
        copy_dir_recursive(src, dst)?;
    } else {
        std::fs::copy(src, dst).context(MigrationSnafu)?;
    }
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst).context(MigrationSnafu)?;
    for entry in std::fs::read_dir(src).context(MigrationSnafu)? {
        let entry = entry.context(MigrationSnafu)?;
        copy_path(&entry.path(), &dst.join(entry.file_name()))?;
    }
    Ok(())
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub version: u32,
    pub general: GeneralConfig,
    pub appearance: AppearanceConfig,
    pub tracking: TrackingConfig,
    pub games: GamesConfig,
    pub notification: NotificationsConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub language: String,
    pub theme: ThemePreference,
    pub check_updates: bool,
    pub update_channel: UpdateChannel,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            language: "auto".to_owned(),
            theme: ThemePreference::System,
            check_updates: true,
            update_channel: UpdateChannel::Stable,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemePreference {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum UpdateChannel {
    #[default]
    Stable,
    Beta,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrackingConfig {
    pub poll_interval_mins: u32,
    pub background_tracking: bool,
}

impl Default for TrackingConfig {
    fn default() -> Self {
        Self {
            poll_interval_mins: 5,
            background_tracking: true,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GamesConfig {
    pub dota2: Dota2Config,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dota2Config {
    pub enabled: bool,
    pub steam_id: Option<String>,
    pub opendota_api_key: Option<String>,
}

impl Default for Dota2Config {
    fn default() -> Self {
        Self {
            enabled: true,
            steam_id: None,
            opendota_api_key: None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NotificationsConfig {
    pub desktop: DesktopNotificationConfig,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DesktopNotificationConfig {
    pub enabled: bool,
    pub sound: bool,
    pub notify_new_match: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppearanceConfig {
    pub sidebar_width: f32,
    pub sidebar_collapsed_width: f32,
    pub sidebar_item_height: f32,
    pub sidebar_icon_size: f32,
    pub animation_sidebar_speed: f32,
    pub animation_toast_speed: f32,
    pub animation_hover_speed: f32,
    pub toast_duration_secs: u64,
    pub toast_max_width: f32,
    pub exit_overlay_opacity: u8,
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            sidebar_width: 220.0,
            sidebar_collapsed_width: 64.0,
            sidebar_item_height: 44.0,
            sidebar_icon_size: 20.0,
            animation_sidebar_speed: 4.0,
            animation_toast_speed: 10.0,
            animation_hover_speed: 12.0,
            toast_duration_secs: 5,
            toast_max_width: 320.0,
            exit_overlay_opacity: 153,
        }
    }
}

impl AppConfig {
    fn load(bootstrap: &Bootstrap) -> Result<Self> {
        let config_path = bootstrap.config_file().context(AppPathSnafu)?;

        let builder = Config::builder()
            .set_default("version", 0)
            .context(ConfigInitSnafu)?
            .add_source(config::File::from(config_path).required(false))
            .add_source(Environment::with_prefix("COURIER").separator("_").try_parsing(true));

        let config = builder.build().context(ConfigInitSnafu)?.try_deserialize::<AppConfig>().context(ConfigInitSnafu)?;
        Ok(config)
    }

    fn save(&self, bootstrap: &Bootstrap) -> Result<()> {
        let config_path = bootstrap.config_file().context(AppPathSnafu)?;
        let toml_string = toml::to_string_pretty(self).context(TomlSerializeSnafu)?;
        std::fs::write(config_path, toml_string).context(ConfigSaveSnafu)?;
        Ok(())
    }
}

pub fn read() -> RwLockReadGuard<'static, AppConfig> {
    CONFIG.read()
}

pub fn save() -> Result<()> {
    let bootstrap = BOOTSTRAP.read();
    let config = CONFIG.read();
    config.save(&bootstrap)
}

pub fn update<F>(f: F)
where
    F: FnOnce(&mut AppConfig),
{
    let mut config = CONFIG.write();
    f(&mut config);
}
