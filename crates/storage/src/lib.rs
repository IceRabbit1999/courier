#![feature(error_generic_member_access)]

use std::path::Path;

use snafu::ResultExt;
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use tracing::info;

pub mod error;
pub use error::*;

const DB_FILE: &str = "courier.db";

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// A handle to the on-disk SQLite database living under the configured storage
/// directory. Cloning the [`pool`](Self::pool) is cheap; background tasks fetch
/// it from here to read and write user data.
pub struct Storage {
    pool: SqlitePool,
}

impl Storage {
    /// Open (creating if missing) the database under `storage_dir`, applying any
    /// pending migrations.
    pub async fn open(storage_dir: impl AsRef<Path>) -> Result<Self> {
        let dir = storage_dir.as_ref();
        std::fs::create_dir_all(dir).context(CreateDirSnafu)?;

        let options = SqliteConnectOptions::new().filename(dir.join(DB_FILE)).create_if_missing(true);
        let pool = SqlitePoolOptions::new().connect_with(options).await.context(ConnectSnafu)?;
        MIGRATOR.run(&pool).await.context(MigrateSnafu)?;

        info!("Storage opened at {}", dir.display());
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Close the pool, releasing the database file. Required before the storage
    /// directory can be moved (see `configs::migrate_storage_path`).
    pub async fn close(&self) {
        self.pool.close().await;
    }
}
