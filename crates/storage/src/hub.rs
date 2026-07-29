//! The hub service's SQLite store: pending link handshakes, linked
//! subscribers, and the per-subscriber tracking state uploaded via `/sync`.
//! Lives in its own database file (courier-hub.db) but shares the crate's
//! migrator, so its `hub_`-prefixed tables are part of the single schema and
//! every query gets the compile-time `query!` check.

use std::{path::Path, sync::Arc};

use parking_lot::Mutex;
use shared::hub::SyncRequest;
use snafu::{OptionExt, ResultExt};
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use tracing::{info, warn};
use ulid::Generator;

use crate::{ConnectSnafu, CreateDirSnafu, MigrateSnafu, QuerySnafu, Result, UnexpectedNoneSnafu, token, unix_now};

const DB_FILE: &str = "courier-hub.db";

/// A fresh handshake: what `/link/new` hands back to the desktop client. The
/// client keeps `secret` and starts polling with `token`; both are plaintext
/// here and nowhere else.
pub struct PendingLink {
    pub token: String,
    pub secret: String,
}

/// A linked subscriber: the chat to push to and the locale to render in.
pub struct Subscriber {
    pub subscriber_id: String,
    pub chat_id: i64,
    pub locale: String,
}

/// One synced account plus its push watermark, for the hub-side tracker.
pub struct TrackedAccount {
    pub steam_id: String,
    pub persona_name: String,
    pub last_match_id: Option<i64>,
}

/// A handle to the hub's on-disk database. Cloning is cheap (the inner
/// [`SqlitePool`] is reference-counted); the HTTP, dispatcher, and tracker sides
/// each hold a clone.
#[derive(Clone)]
pub struct HubStore {
    pool: SqlitePool,
    ids: Arc<Mutex<Generator>>,
}

impl HubStore {
    /// Open (creating if missing) the hub database under `storage_dir`,
    /// applying any pending migrations.
    pub async fn open(storage_dir: impl AsRef<Path>) -> Result<Self> {
        let dir = storage_dir.as_ref();
        std::fs::create_dir_all(dir).context(CreateDirSnafu)?;

        let options = SqliteConnectOptions::new().filename(dir.join(DB_FILE)).create_if_missing(true);
        let pool = SqlitePoolOptions::new().connect_with(options).await.context(ConnectSnafu)?;
        crate::MIGRATOR.run(&pool).await.context(MigrateSnafu)?;

        info!("Hub store opened at {}", dir.display());
        Ok(Self {
            pool,
            ids: Arc::new(Mutex::new(Generator::new())),
        })
    }

    /// Next subscriber id. The generator only fails when a millisecond's random
    /// component saturates; a plain ULID is still unique, so fall back rather
    /// than failing the link.
    fn next_id(&self) -> String {
        let mut ids = self.ids.lock();
        ids.generate()
            .unwrap_or_else(|e| {
                warn!("monotonic ULID overflow, falling back to a random one: {e}");
                ulid::Ulid::generate()
            })
            .to_string()
    }

    /// Start a handshake: mint the single-use link token *and* the secret that
    /// will authenticate the subscriber once the chat binds, storing only their
    /// digests. Minting the secret up front is what lets the whole flow keep
    /// hashes at rest — `/link/status` never has to hand back a credential it
    /// would otherwise need to store in the clear.
    pub async fn create_pending(&self) -> Result<PendingLink> {
        let link_token = token::mint();
        let secret = token::mint();
        let now = unix_now();
        sqlx::query!(
            "INSERT INTO hub_pending_links (token_hash, secret_hash, created_at, subscriber_id) VALUES (?, ?, ?, NULL)",
            link_token.hash,
            secret.hash,
            now,
        )
        .execute(&self.pool)
        .await
        .context(QuerySnafu)?;
        Ok(PendingLink {
            token: link_token.plaintext,
            secret: secret.plaintext,
        })
    }

    /// Bind a chat to a pending token: promote the pending row's secret into a
    /// real subscriber and mark the token linked. Returns the new subscriber id,
    /// or `None` if the token is unknown or was already used.
    pub async fn bind(&self, token: &str, chat_id: i64) -> Result<Option<String>> {
        let token_hash = token::hash(token);
        let mut tx = self.pool.begin().await.context(QuerySnafu)?;

        let existing = sqlx::query!("SELECT subscriber_id, secret_hash FROM hub_pending_links WHERE token_hash = ?", token_hash)
            .fetch_optional(&mut *tx)
            .await
            .context(QuerySnafu)?;
        let Some(row) = existing else {
            return Ok(None);
        };
        if row.subscriber_id.is_some() {
            return Ok(None);
        }

        let subscriber_id = self.next_id();
        let now = unix_now();
        sqlx::query!(
            "INSERT INTO hub_subscribers (subscriber_id, secret_hash, chat_id, created_at) VALUES (?, ?, ?, ?)",
            subscriber_id,
            row.secret_hash,
            chat_id,
            now,
        )
        .execute(&mut *tx)
        .await
        .context(QuerySnafu)?;
        sqlx::query!("UPDATE hub_pending_links SET subscriber_id = ? WHERE token_hash = ?", subscriber_id, token_hash)
            .execute(&mut *tx)
            .await
            .context(QuerySnafu)?;

        tx.commit().await.context(QuerySnafu)?;
        Ok(Some(subscriber_id))
    }

    /// Poll a handshake: once the token is bound, return the subscriber id. The
    /// client already holds the matching secret from `/link/new`.
    pub async fn status(&self, token: &str) -> Result<Option<String>> {
        let token_hash = token::hash(token);
        sqlx::query_scalar!(
            r#"SELECT s.subscriber_id AS "subscriber_id!"
               FROM hub_pending_links p JOIN hub_subscribers s ON s.subscriber_id = p.subscriber_id
               WHERE p.token_hash = ?"#,
            token_hash,
        )
        .fetch_optional(&self.pool)
        .await
        .context(QuerySnafu)
    }

    /// Resolve the Telegram chat a bearer secret authorizes pushing to.
    pub async fn chat_for_secret(&self, secret: &str) -> Result<i64> {
        let secret_hash = token::hash(secret);
        sqlx::query_scalar!("SELECT chat_id FROM hub_subscribers WHERE secret_hash = ?", secret_hash)
            .fetch_one(&self.pool)
            .await
            .context(QuerySnafu)
    }

    pub async fn delete_subscriber(&self, secret: &str) -> Result<()> {
        let secret_hash = token::hash(secret);
        let mut tx = self.pool.begin().await.context(QuerySnafu)?;
        let subscriber_id = sqlx::query_scalar!("SELECT subscriber_id FROM hub_subscribers WHERE secret_hash = ?", secret_hash)
            .fetch_optional(&mut *tx)
            .await
            .context(QuerySnafu)?
            .context(UnexpectedNoneSnafu { expected: "subscriber_id" })?;
        sqlx::query!("DELETE FROM hub_tracked_accounts WHERE subscriber_id = ?", subscriber_id)
            .execute(&mut *tx)
            .await
            .context(QuerySnafu)?;
        sqlx::query!("DELETE FROM hub_subscribers WHERE subscriber_id = ?", subscriber_id)
            .execute(&mut *tx)
            .await
            .context(QuerySnafu)?;
        tx.commit().await.context(QuerySnafu)?;
        Ok(())
    }

    /// Apply a `/sync` snapshot: replace the subscriber's tracked set (keeping
    /// the watermark of accounts that stay, so a re-sync never re-announces old
    /// matches), update the offline flag and locale, and refresh the hero names
    /// for that locale.
    pub async fn sync(&self, secret: &str, req: &SyncRequest) -> Result<()> {
        let secret_hash = token::hash(secret);
        let mut tx = self.pool.begin().await.context(QuerySnafu)?;

        let subscriber_id = sqlx::query_scalar!("SELECT subscriber_id FROM hub_subscribers WHERE secret_hash = ?", secret_hash)
            .fetch_one(&mut *tx)
            .await
            .context(QuerySnafu)?
            .context(UnexpectedNoneSnafu { expected: "subscriber_id" })?;

        let offline_mode = i64::from(req.offline_mode);
        sqlx::query!(
            "UPDATE hub_subscribers SET offline_mode = ?, locale = ? WHERE subscriber_id = ?",
            offline_mode,
            req.locale,
            subscriber_id,
        )
        .execute(&mut *tx)
        .await
        .context(QuerySnafu)?;

        let existing = sqlx::query_scalar!("SELECT steam_id FROM hub_tracked_accounts WHERE subscriber_id = ?", subscriber_id)
            .fetch_all(&mut *tx)
            .await
            .context(QuerySnafu)?;
        let dropped = existing.into_iter().filter(|steam_id| !req.accounts.iter().any(|a| &a.steam_id == steam_id));
        for steam_id in dropped {
            sqlx::query!("DELETE FROM hub_tracked_accounts WHERE subscriber_id = ? AND steam_id = ?", subscriber_id, steam_id,)
                .execute(&mut *tx)
                .await
                .context(QuerySnafu)?;
        }
        for account in &req.accounts {
            sqlx::query!(
                r#"INSERT INTO hub_tracked_accounts (subscriber_id, steam_id, persona_name, last_match_id) VALUES (?, ?, ?, NULL)
                   ON CONFLICT (subscriber_id, steam_id) DO UPDATE SET persona_name = excluded.persona_name"#,
                subscriber_id,
                account.steam_id,
                account.persona_name,
            )
            .execute(&mut *tx)
            .await
            .context(QuerySnafu)?;
        }

        for (hero_id, name) in &req.hero_names {
            sqlx::query!(
                "INSERT INTO hub_hero_names (locale, hero_id, name) VALUES (?, ?, ?) ON CONFLICT (locale, hero_id) DO UPDATE SET name = excluded.name",
                req.locale,
                hero_id,
                name,
            )
            .execute(&mut *tx)
            .await
            .context(QuerySnafu)?;
        }

        for (item_id, name) in &req.item_names {
            sqlx::query!(
                "INSERT INTO hub_item_names (locale, item_id, name) VALUES (?, ?, ?) ON CONFLICT (locale, item_id) DO UPDATE SET name = excluded.name",
                req.locale,
                item_id,
                name,
            )
            .execute(&mut *tx)
            .await
            .context(QuerySnafu)?;
        }

        tx.commit().await.context(QuerySnafu)?;
        Ok(())
    }

    pub async fn offline_subscribers(&self) -> Result<Vec<Subscriber>> {
        sqlx::query_as!(
            Subscriber,
            r#"SELECT subscriber_id AS "subscriber_id!", chat_id, locale FROM hub_subscribers WHERE offline_mode = 1"#,
        )
        .fetch_all(&self.pool)
        .await
        .context(QuerySnafu)
    }

    /// Resolve the subscriber bound to a Telegram chat, for updates that arrive
    /// from the bot rather than from an authenticated desktop client.
    pub async fn subscriber_by_chat(&self, chat_id: i64) -> Result<Option<Subscriber>> {
        sqlx::query_as!(
            Subscriber,
            r#"SELECT subscriber_id AS "subscriber_id!", chat_id, locale FROM hub_subscribers WHERE chat_id = ?"#,
            chat_id,
        )
        .fetch_optional(&self.pool)
        .await
        .context(QuerySnafu)
    }

    /// Resolve the subscriber a bearer secret authorizes, so a `/sync` can be
    /// followed by an immediate tracker pass for that subscriber alone.
    pub async fn subscriber_by_secret(&self, secret: &str) -> Result<Option<Subscriber>> {
        let secret_hash = token::hash(secret);
        sqlx::query_as!(
            Subscriber,
            r#"SELECT subscriber_id AS "subscriber_id!", chat_id, locale FROM hub_subscribers WHERE secret_hash = ?"#,
            secret_hash,
        )
        .fetch_optional(&self.pool)
        .await
        .context(QuerySnafu)
    }

    pub async fn tracked_accounts(&self, subscriber_id: &str) -> Result<Vec<TrackedAccount>> {
        sqlx::query_as!(
            TrackedAccount,
            "SELECT steam_id, persona_name, last_match_id FROM hub_tracked_accounts WHERE subscriber_id = ?",
            subscriber_id,
        )
        .fetch_all(&self.pool)
        .await
        .context(QuerySnafu)
    }

    pub async fn set_account_watermark(&self, subscriber_id: &str, steam_id: &str, match_id: i64) -> Result<()> {
        sqlx::query!(
            "UPDATE hub_tracked_accounts SET last_match_id = ? WHERE subscriber_id = ? AND steam_id = ?",
            match_id,
            subscriber_id,
            steam_id,
        )
        .execute(&self.pool)
        .await
        .context(QuerySnafu)?;
        Ok(())
    }

    pub async fn hero_names(&self, locale: &str) -> Result<std::collections::HashMap<i32, String>> {
        let rows = sqlx::query!("SELECT hero_id, name FROM hub_hero_names WHERE locale = ?", locale)
            .fetch_all(&self.pool)
            .await
            .context(QuerySnafu)?;
        Ok(rows.into_iter().map(|row| (row.hero_id as i32, row.name)).collect())
    }

    pub async fn item_names(&self, locale: &str) -> Result<std::collections::HashMap<i32, String>> {
        let rows = sqlx::query!("SELECT item_id, name FROM hub_item_names WHERE locale = ?", locale)
            .fetch_all(&self.pool)
            .await
            .context(QuerySnafu)?;
        Ok(rows.into_iter().map(|row| (row.item_id as i32, row.name)).collect())
    }
}
