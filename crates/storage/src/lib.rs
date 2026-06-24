#![feature(error_generic_member_access)]

use std::{
    collections::HashMap,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use shared::{Friend, HeroEntry, ItemEntry, PersonaState, RecentGame};
use snafu::ResultExt;
use sqlx::{
    Row, SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use tracing::info;

pub mod error;
pub use error::*;

const DB_FILE: &str = "courier.db";

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// A handle to the on-disk SQLite database living under the configured storage
/// directory. Cloning is cheap (the inner [`SqlitePool`] is reference-counted);
/// background tasks clone it to read and write user data off the UI thread.
#[derive(Clone)]
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

    /// Upsert the hero list for one `locale`: the base rows (id + internal name)
    /// plus the localized display names. Called once per locale during a sync.
    pub async fn upsert_heroes(&self, locale: &str, heroes: &[HeroEntry]) -> Result<()> {
        let mut tx = self.pool.begin().await.context(QuerySnafu)?;
        for hero in heroes {
            sqlx::query!(
                "INSERT INTO heroes (id, internal_name) VALUES (?, ?) ON CONFLICT(id) DO UPDATE SET internal_name = excluded.internal_name",
                hero.id,
                hero.internal_name,
            )
            .execute(&mut *tx)
            .await
            .context(QuerySnafu)?;
            sqlx::query!(
                "INSERT INTO hero_names (hero_id, locale, display_name) VALUES (?, ?, ?) ON CONFLICT(hero_id, locale) DO UPDATE SET display_name = excluded.display_name",
                hero.id,
                locale,
                hero.display_name,
            )
            .execute(&mut *tx)
            .await
            .context(QuerySnafu)?;
        }
        tx.commit().await.context(QuerySnafu)?;
        self.mark_synced("heroes").await
    }

    /// Upsert the item list for one `locale`. See [`Self::upsert_heroes`].
    pub async fn upsert_items(&self, locale: &str, items: &[ItemEntry]) -> Result<()> {
        let mut tx = self.pool.begin().await.context(QuerySnafu)?;
        for item in items {
            sqlx::query!(
                "INSERT INTO items (id, short_name) VALUES (?, ?) ON CONFLICT(id) DO UPDATE SET short_name = excluded.short_name",
                item.id,
                item.short_name,
            )
            .execute(&mut *tx)
            .await
            .context(QuerySnafu)?;
            sqlx::query!(
                "INSERT INTO item_names (item_id, locale, display_name) VALUES (?, ?, ?) ON CONFLICT(item_id, locale) DO UPDATE SET display_name = excluded.display_name",
                item.id,
                locale,
                item.display_name,
            )
            .execute(&mut *tx)
            .await
            .context(QuerySnafu)?;
        }
        tx.commit().await.context(QuerySnafu)?;
        self.mark_synced("items").await
    }

    pub async fn hero_count(&self) -> Result<i64> {
        let count = sqlx::query_scalar!(r#"SELECT COUNT(*) AS "count: i64" FROM heroes"#)
            .fetch_one(&self.pool)
            .await
            .context(QuerySnafu)?;
        Ok(count)
    }

    pub async fn item_count(&self) -> Result<i64> {
        let count = sqlx::query_scalar!(r#"SELECT COUNT(*) AS "count: i64" FROM items"#)
            .fetch_one(&self.pool)
            .await
            .context(QuerySnafu)?;
        Ok(count)
    }

    /// Replace the stored friend list with `friends`, stamping each row with the
    /// current sync time. Stale rows (no longer friends) are removed.
    pub async fn replace_friends(&self, friends: &[Friend]) -> Result<()> {
        let now = unix_now();
        let mut tx = self.pool.begin().await.context(QuerySnafu)?;
        sqlx::query!("DELETE FROM friends").execute(&mut *tx).await.context(QuerySnafu)?;
        sqlx::query!("DELETE FROM friend_recent_games").execute(&mut *tx).await.context(QuerySnafu)?;
        for friend in friends {
            sqlx::query!(
                "INSERT INTO friends (steam_id, friend_since, persona_name, avatar, profile_url, persona_state, last_log_off, game_extra_info, updated_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                friend.steam_id,
                friend.friend_since,
                friend.persona_name,
                friend.avatar,
                friend.profile_url,
                friend.persona_state as i32,
                friend.last_log_off,
                friend.game_extra_info,
                now,
            )
            .execute(&mut *tx)
            .await
            .context(QuerySnafu)?;

            for game in &friend.recent_games {
                sqlx::query(
                    "INSERT INTO friend_recent_games (steam_id, app_id, name, playtime_2weeks, playtime_forever, img_icon_url) \
                     VALUES (?, ?, ?, ?, ?, ?)",
                )
                .bind(&friend.steam_id)
                .bind(game.app_id)
                .bind(&game.name)
                .bind(game.playtime_2weeks)
                .bind(game.playtime_forever)
                .bind(&game.img_icon_url)
                .execute(&mut *tx)
                .await
                .context(QuerySnafu)?;
            }
        }
        tx.commit().await.context(QuerySnafu)?;
        self.mark_synced("friends").await
    }

    /// Recently played games for every friend, keyed by steam id and ordered by
    /// two-week playtime (longest first).
    async fn recent_games_by_friend(&self) -> Result<HashMap<String, Vec<RecentGame>>> {
        let rows = sqlx::query(
            "SELECT steam_id, app_id, name, playtime_2weeks, playtime_forever, img_icon_url \
             FROM friend_recent_games ORDER BY playtime_2weeks DESC",
        )
        .fetch_all(&self.pool)
        .await
        .context(QuerySnafu)?;

        let mut by_friend: HashMap<String, Vec<RecentGame>> = HashMap::new();
        for row in rows {
            let steam_id = row.try_get::<String, _>("steam_id").context(QuerySnafu)?;
            by_friend.entry(steam_id).or_default().push(RecentGame {
                app_id: row.try_get("app_id").context(QuerySnafu)?,
                name: row.try_get("name").context(QuerySnafu)?,
                playtime_2weeks: row.try_get("playtime_2weeks").context(QuerySnafu)?,
                playtime_forever: row.try_get("playtime_forever").context(QuerySnafu)?,
                img_icon_url: row.try_get("img_icon_url").context(QuerySnafu)?,
            });
        }
        Ok(by_friend)
    }

    /// The stored friend list, online friends first then alphabetical.
    pub async fn list_friends(&self) -> Result<Vec<Friend>> {
        let rows = sqlx::query!(
            r#"SELECT steam_id AS "steam_id!", friend_since, persona_name, avatar, profile_url, persona_state, last_log_off, game_extra_info
               FROM friends ORDER BY (persona_state != 0) DESC, persona_name COLLATE NOCASE ASC"#,
        )
        .fetch_all(&self.pool)
        .await
        .context(QuerySnafu)?;

        let mut recent_games = self.recent_games_by_friend().await?;

        Ok(rows
            .into_iter()
            .map(|row| Friend {
                recent_games: recent_games.remove(&row.steam_id).unwrap_or_default(),
                steam_id: row.steam_id,
                friend_since: row.friend_since,
                persona_name: row.persona_name,
                avatar: row.avatar,
                profile_url: row.profile_url,
                persona_state: PersonaState::from_i32(row.persona_state as i32),
                last_log_off: row.last_log_off,
                game_extra_info: row.game_extra_info,
            })
            .collect())
    }

    /// Last successful sync time (unix seconds) for `key`, if any.
    pub async fn last_synced(&self, key: &str) -> Result<Option<i64>> {
        sqlx::query_scalar!("SELECT synced_at FROM sync_meta WHERE key = ?", key)
            .fetch_optional(&self.pool)
            .await
            .context(QuerySnafu)
    }

    async fn mark_synced(&self, key: &str) -> Result<()> {
        let now = unix_now();
        sqlx::query!(
            "INSERT INTO sync_meta (key, synced_at) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET synced_at = excluded.synced_at",
            key,
            now,
        )
        .execute(&self.pool)
        .await
        .context(QuerySnafu)?;
        Ok(())
    }
}

fn unix_now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or_default()
}
