#![feature(error_generic_member_access)]

use std::{
    collections::HashMap,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use shared::{Follow, Friend, HeroEntry, ItemEntry, MatchDetail, MatchPlayer, MatchSummary, PersonaState, RecentGame};
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
                r#"INSERT INTO friends (steam_id, friend_since, persona_name, avatar, profile_url, persona_state, last_log_off, game_extra_info, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
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
                sqlx::query!(
                    r#"INSERT INTO friend_recent_games (steam_id, app_id, name, playtime_2weeks, playtime_forever, img_icon_url)
                     VALUES (?, ?, ?, ?, ?, ?)"#,
                    friend.steam_id,
                    game.app_id,
                    game.name,
                    game.playtime_2weeks,
                    game.playtime_forever,
                    game.img_icon_url,
                )
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
        let rows = sqlx::query!(
            r#"SELECT steam_id AS "steam_id!", app_id, name, playtime_2weeks, playtime_forever, img_icon_url
             FROM friend_recent_games ORDER BY playtime_2weeks DESC"#,
        )
        .fetch_all(&self.pool)
        .await
        .context(QuerySnafu)?;

        let mut by_friend: HashMap<String, Vec<RecentGame>> = HashMap::new();
        for row in rows {
            by_friend.entry(row.steam_id).or_default().push(RecentGame {
                app_id: row.app_id,
                name: row.name,
                playtime_2weeks: row.playtime_2weeks,
                playtime_forever: row.playtime_forever,
                img_icon_url: row.img_icon_url,
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

    /// Recently played games for every followed player, keyed by steam id and
    /// ordered by two-week playtime (longest first).
    async fn recent_games_by_follow(&self) -> Result<HashMap<String, Vec<RecentGame>>> {
        let rows = sqlx::query!(
            r#"SELECT steam_id AS "steam_id!", app_id, name, playtime_2weeks, playtime_forever, img_icon_url
             FROM follow_recent_games ORDER BY playtime_2weeks DESC"#,
        )
        .fetch_all(&self.pool)
        .await
        .context(QuerySnafu)?;

        let mut by_follow: HashMap<String, Vec<RecentGame>> = HashMap::new();
        for row in rows {
            by_follow.entry(row.steam_id).or_default().push(RecentGame {
                app_id: row.app_id,
                name: row.name,
                playtime_2weeks: row.playtime_2weeks,
                playtime_forever: row.playtime_forever,
                img_icon_url: row.img_icon_url,
            });
        }
        Ok(by_follow)
    }

    /// The stored follow list, online players first then alphabetical.
    pub async fn list_follows(&self) -> Result<Vec<Follow>> {
        let rows = sqlx::query!(
            r#"SELECT steam_id AS "steam_id!", added_at, persona_name, avatar, profile_url, persona_state, last_log_off, game_extra_info
               FROM follows ORDER BY (persona_state != 0) DESC, persona_name COLLATE NOCASE ASC"#,
        )
        .fetch_all(&self.pool)
        .await
        .context(QuerySnafu)?;

        let mut recent_games = self.recent_games_by_follow().await?;

        Ok(rows
            .into_iter()
            .map(|row| Follow {
                recent_games: recent_games.remove(&row.steam_id).unwrap_or_default(),
                steam_id: row.steam_id,
                added_at: row.added_at,
                persona_name: row.persona_name,
                avatar: row.avatar,
                profile_url: row.profile_url,
                persona_state: PersonaState::from_i32(row.persona_state as i32),
                last_log_off: row.last_log_off,
                game_extra_info: row.game_extra_info,
            })
            .collect())
    }

    /// Upsert `follows` rows: inserts new players, and for existing ones updates
    /// their presence/status fields without touching `added_at`. Each row's
    /// recent games are replaced wholesale.
    pub async fn upsert_follows(&self, follows: &[Follow]) -> Result<()> {
        let now = unix_now();
        let mut tx = self.pool.begin().await.context(QuerySnafu)?;
        for follow in follows {
            sqlx::query!(
                r#"INSERT INTO follows (steam_id, added_at, persona_name, avatar, profile_url, persona_state, last_log_off, game_extra_info, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                 ON CONFLICT(steam_id) DO UPDATE SET
                    persona_name = excluded.persona_name, avatar = excluded.avatar, profile_url = excluded.profile_url,
                    persona_state = excluded.persona_state, last_log_off = excluded.last_log_off,
                    game_extra_info = excluded.game_extra_info, updated_at = excluded.updated_at"#,
                follow.steam_id,
                follow.added_at,
                follow.persona_name,
                follow.avatar,
                follow.profile_url,
                follow.persona_state as i32,
                follow.last_log_off,
                follow.game_extra_info,
                now,
            )
            .execute(&mut *tx)
            .await
            .context(QuerySnafu)?;

            sqlx::query!("DELETE FROM follow_recent_games WHERE steam_id = ?", follow.steam_id)
                .execute(&mut *tx)
                .await
                .context(QuerySnafu)?;
            for game in &follow.recent_games {
                sqlx::query!(
                    r#"INSERT INTO follow_recent_games (steam_id, app_id, name, playtime_2weeks, playtime_forever, img_icon_url)
                     VALUES (?, ?, ?, ?, ?, ?)"#,
                    follow.steam_id,
                    game.app_id,
                    game.name,
                    game.playtime_2weeks,
                    game.playtime_forever,
                    game.img_icon_url,
                )
                .execute(&mut *tx)
                .await
                .context(QuerySnafu)?;
            }
        }
        tx.commit().await.context(QuerySnafu)?;
        Ok(())
    }

    /// Stop following one player.
    pub async fn remove_follow(&self, steam_id: &str) -> Result<()> {
        let mut tx = self.pool.begin().await.context(QuerySnafu)?;
        sqlx::query!("DELETE FROM follow_recent_games WHERE steam_id = ?", steam_id)
            .execute(&mut *tx)
            .await
            .context(QuerySnafu)?;
        sqlx::query!("DELETE FROM follows WHERE steam_id = ?", steam_id)
            .execute(&mut *tx)
            .await
            .context(QuerySnafu)?;
        tx.commit().await.context(QuerySnafu)?;
        Ok(())
    }

    /// Copy every current friend into `follows` (a no-op for ones already followed).
    pub async fn add_all_friends_to_follows(&self) -> Result<()> {
        let now = unix_now();
        let mut tx = self.pool.begin().await.context(QuerySnafu)?;
        sqlx::query!(
            r#"INSERT OR IGNORE INTO follows (steam_id, added_at, persona_name, avatar, profile_url, persona_state, last_log_off, game_extra_info, updated_at)
             SELECT steam_id, ?, persona_name, avatar, profile_url, persona_state, last_log_off, game_extra_info, updated_at FROM friends"#,
            now,
        )
        .execute(&mut *tx)
        .await
        .context(QuerySnafu)?;
        sqlx::query!(
            r#"INSERT OR IGNORE INTO follow_recent_games (steam_id, app_id, name, playtime_2weeks, playtime_forever, img_icon_url)
             SELECT steam_id, app_id, name, playtime_2weeks, playtime_forever, img_icon_url FROM friend_recent_games"#,
        )
        .execute(&mut *tx)
        .await
        .context(QuerySnafu)?;
        tx.commit().await.context(QuerySnafu)?;
        Ok(())
    }

    /// Copy specific friends (by `steam_id`) into `follows` (a no-op for ones
    /// already followed or no longer a friend).
    pub async fn add_friends_to_follows(&self, steam_ids: &[String]) -> Result<()> {
        let now = unix_now();
        let mut tx = self.pool.begin().await.context(QuerySnafu)?;
        for steam_id in steam_ids {
            sqlx::query!(
                r#"INSERT OR IGNORE INTO follows (steam_id, added_at, persona_name, avatar, profile_url, persona_state, last_log_off, game_extra_info, updated_at)
                 SELECT steam_id, ?, persona_name, avatar, profile_url, persona_state, last_log_off, game_extra_info, updated_at FROM friends WHERE steam_id = ?"#,
                now,
                steam_id,
            )
            .execute(&mut *tx)
            .await
            .context(QuerySnafu)?;
            sqlx::query!(
                r#"INSERT OR IGNORE INTO follow_recent_games (steam_id, app_id, name, playtime_2weeks, playtime_forever, img_icon_url)
                 SELECT steam_id, app_id, name, playtime_2weeks, playtime_forever, img_icon_url FROM friend_recent_games WHERE steam_id = ?"#,
                steam_id,
            )
            .execute(&mut *tx)
            .await
            .context(QuerySnafu)?;
        }
        tx.commit().await.context(QuerySnafu)?;
        Ok(())
    }

    /// Remove every current friend from `follows` (players followed independently
    /// of any friendship are untouched).
    pub async fn remove_all_friends_from_follows(&self) -> Result<()> {
        let mut tx = self.pool.begin().await.context(QuerySnafu)?;
        sqlx::query!("DELETE FROM follow_recent_games WHERE steam_id IN (SELECT steam_id FROM friends)")
            .execute(&mut *tx)
            .await
            .context(QuerySnafu)?;
        sqlx::query!("DELETE FROM follows WHERE steam_id IN (SELECT steam_id FROM friends)")
            .execute(&mut *tx)
            .await
            .context(QuerySnafu)?;
        tx.commit().await.context(QuerySnafu)?;
        Ok(())
    }

    /// Replace the stored match summaries for one friend with `summaries`.
    /// Summaries belong to a specific friend (the player whose history they came
    /// from), so a re-fetch wholesale replaces that friend's rows only.
    pub async fn replace_match_summaries(&self, steam_id: &str, summaries: &[MatchSummary]) -> Result<()> {
        let mut tx = self.pool.begin().await.context(QuerySnafu)?;
        sqlx::query!("DELETE FROM match_summaries WHERE steam_id = ?", steam_id)
            .execute(&mut *tx)
            .await
            .context(QuerySnafu)?;
        for m in summaries {
            sqlx::query!(
                r#"INSERT INTO match_summaries
                 (steam_id, match_id, hero_id, player_slot, radiant_win, start_time, duration, game_mode, lobby_type,
                  kills, deaths, assists, gold_per_min, xp_per_min, last_hits, hero_damage, tower_damage, hero_healing)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
                steam_id,
                m.match_id,
                m.hero_id,
                m.player_slot,
                m.radiant_win,
                m.start_time,
                m.duration,
                m.game_mode,
                m.lobby_type,
                m.kills,
                m.deaths,
                m.assists,
                m.gold_per_min,
                m.xp_per_min,
                m.last_hits,
                m.hero_damage,
                m.tower_damage,
                m.hero_healing,
            )
            .execute(&mut *tx)
            .await
            .context(QuerySnafu)?;
        }
        tx.commit().await.context(QuerySnafu)?;
        self.mark_synced("matches").await
    }

    /// One friend's stored match summaries, newest first.
    pub async fn list_match_summaries(&self, steam_id: &str) -> Result<Vec<MatchSummary>> {
        let rows = sqlx::query!(
            r#"SELECT match_id, hero_id, player_slot, radiant_win, start_time, duration, game_mode, lobby_type,
                    kills, deaths, assists, gold_per_min, xp_per_min, last_hits, hero_damage, tower_damage, hero_healing
             FROM match_summaries WHERE steam_id = ? ORDER BY start_time DESC"#,
            steam_id,
        )
        .fetch_all(&self.pool)
        .await
        .context(QuerySnafu)?;

        Ok(rows
            .into_iter()
            .map(|row| MatchSummary {
                match_id: row.match_id,
                hero_id: row.hero_id as i32,
                player_slot: row.player_slot as i32,
                radiant_win: row.radiant_win != 0,
                start_time: row.start_time,
                duration: row.duration as i32,
                game_mode: row.game_mode as i32,
                lobby_type: row.lobby_type as i32,
                kills: row.kills as i32,
                deaths: row.deaths as i32,
                assists: row.assists as i32,
                gold_per_min: row.gold_per_min as i32,
                xp_per_min: row.xp_per_min as i32,
                last_hits: row.last_hits as i32,
                hero_damage: row.hero_damage as i32,
                tower_damage: row.tower_damage as i32,
                hero_healing: row.hero_healing as i32,
            })
            .collect())
    }

    /// Upsert the full detail for one match: the header row plus its players
    /// (replaced wholesale).
    pub async fn upsert_match_detail(&self, detail: &MatchDetail) -> Result<()> {
        let mut tx = self.pool.begin().await.context(QuerySnafu)?;
        sqlx::query!(
            r#"INSERT INTO match_details
             (match_id, radiant_win, duration, start_time, game_mode, lobby_type, radiant_score, dire_score, first_blood_time)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(match_id) DO UPDATE SET
                radiant_win = excluded.radiant_win, duration = excluded.duration, start_time = excluded.start_time,
                game_mode = excluded.game_mode, lobby_type = excluded.lobby_type, radiant_score = excluded.radiant_score,
                dire_score = excluded.dire_score, first_blood_time = excluded.first_blood_time"#,
            detail.match_id,
            detail.radiant_win,
            detail.duration,
            detail.start_time,
            detail.game_mode,
            detail.lobby_type,
            detail.radiant_score,
            detail.dire_score,
            detail.first_blood_time,
        )
        .execute(&mut *tx)
        .await
        .context(QuerySnafu)?;

        sqlx::query!("DELETE FROM match_players WHERE match_id = ?", detail.match_id)
            .execute(&mut *tx)
            .await
            .context(QuerySnafu)?;
        for p in &detail.players {
            let (item_0, item_1, item_2, item_3, item_4, item_5) = (p.items[0], p.items[1], p.items[2], p.items[3], p.items[4], p.items[5]);
            let (backpack_0, backpack_1, backpack_2) = (p.backpack[0], p.backpack[1], p.backpack[2]);
            sqlx::query!(
                r#"INSERT INTO match_players
                 (match_id, player_slot, account_id, hero_id, personaname, kills, deaths, assists, last_hits, denies,
                  gold_per_min, xp_per_min, level, net_worth, hero_damage, tower_damage, hero_healing,
                  item_0, item_1, item_2, item_3, item_4, item_5, backpack_0, backpack_1, backpack_2, item_neutral, item_neutral2,
                  aghanims_scepter, aghanims_shard)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
                detail.match_id,
                p.player_slot,
                p.account_id,
                p.hero_id,
                p.personaname,
                p.kills,
                p.deaths,
                p.assists,
                p.last_hits,
                p.denies,
                p.gold_per_min,
                p.xp_per_min,
                p.level,
                p.net_worth,
                p.hero_damage,
                p.tower_damage,
                p.hero_healing,
                item_0,
                item_1,
                item_2,
                item_3,
                item_4,
                item_5,
                backpack_0,
                backpack_1,
                backpack_2,
                p.item_neutral,
                p.item_neutral2,
                p.aghanims_scepter,
                p.aghanims_shard,
            )
            .execute(&mut *tx)
            .await
            .context(QuerySnafu)?;
        }
        tx.commit().await.context(QuerySnafu)?;
        Ok(())
    }

    /// The stored detail for one match, if it has been fetched. Players are
    /// ordered by slot (Radiant 0..=127 first, then Dire).
    pub async fn get_match_detail(&self, match_id: i64) -> Result<Option<MatchDetail>> {
        let Some(head) = sqlx::query!(
            r#"SELECT radiant_win, duration, start_time, game_mode, lobby_type, radiant_score, dire_score, first_blood_time
             FROM match_details WHERE match_id = ?"#,
            match_id,
        )
        .fetch_optional(&self.pool)
        .await
        .context(QuerySnafu)?
        else {
            return Ok(None);
        };

        let player_rows = sqlx::query!(
            r#"SELECT player_slot, account_id, hero_id, personaname, kills, deaths, assists, last_hits, denies,
                    gold_per_min, xp_per_min, level, net_worth, hero_damage, tower_damage, hero_healing,
                    item_0, item_1, item_2, item_3, item_4, item_5, backpack_0, backpack_1, backpack_2, item_neutral, item_neutral2,
                    aghanims_scepter, aghanims_shard
             FROM match_players WHERE match_id = ? ORDER BY player_slot ASC"#,
            match_id,
        )
        .fetch_all(&self.pool)
        .await
        .context(QuerySnafu)?;

        let players = player_rows
            .into_iter()
            .map(|row| MatchPlayer {
                player_slot: row.player_slot as i32,
                account_id: row.account_id,
                hero_id: row.hero_id as i32,
                personaname: row.personaname,
                kills: row.kills as i32,
                deaths: row.deaths as i32,
                assists: row.assists as i32,
                last_hits: row.last_hits as i32,
                denies: row.denies as i32,
                gold_per_min: row.gold_per_min as i32,
                xp_per_min: row.xp_per_min as i32,
                level: row.level as i32,
                net_worth: row.net_worth as i32,
                hero_damage: row.hero_damage as i32,
                tower_damage: row.tower_damage as i32,
                hero_healing: row.hero_healing as i32,
                items: [
                    row.item_0 as i32,
                    row.item_1 as i32,
                    row.item_2 as i32,
                    row.item_3 as i32,
                    row.item_4 as i32,
                    row.item_5 as i32,
                ],
                backpack: [row.backpack_0 as i32, row.backpack_1 as i32, row.backpack_2 as i32],
                item_neutral: row.item_neutral as i32,
                item_neutral2: row.item_neutral2 as i32,
                aghanims_scepter: row.aghanims_scepter != 0,
                aghanims_shard: row.aghanims_shard != 0,
            })
            .collect::<Vec<_>>();

        Ok(Some(MatchDetail {
            match_id,
            radiant_win: head.radiant_win != 0,
            duration: head.duration as i32,
            start_time: head.start_time,
            game_mode: head.game_mode as i32,
            lobby_type: head.lobby_type as i32,
            radiant_score: head.radiant_score as i32,
            dire_score: head.dire_score as i32,
            first_blood_time: head.first_blood_time as i32,
            players,
        }))
    }

    /// Localized hero names keyed by hero id, falling back to the internal name
    /// when a locale has no translation. Used to render match heroes as text.
    pub async fn hero_names(&self, locale: &str) -> Result<HashMap<i32, String>> {
        let rows = sqlx::query!(
            r#"SELECT h.id AS "id!", COALESCE(hn.display_name, h.internal_name) AS "name!"
             FROM heroes h LEFT JOIN hero_names hn ON hn.hero_id = h.id AND hn.locale = ?"#,
            locale,
        )
        .fetch_all(&self.pool)
        .await
        .context(QuerySnafu)?;

        let mut map = HashMap::with_capacity(rows.len());
        for row in rows {
            map.insert(row.id as i32, row.name);
        }
        Ok(map)
    }

    /// Hero internal names keyed by hero id (e.g. `npc_dota_hero_antimage`). Used
    /// to build CDN icon URLs; locale-independent.
    pub async fn hero_slugs(&self) -> Result<HashMap<i32, String>> {
        let rows = sqlx::query!("SELECT id, internal_name FROM heroes").fetch_all(&self.pool).await.context(QuerySnafu)?;
        let mut map = HashMap::with_capacity(rows.len());
        for row in rows {
            map.insert(row.id as i32, row.internal_name);
        }
        Ok(map)
    }

    /// Item short names keyed by item id (e.g. `blink`). Used to build CDN icon
    /// URLs; locale-independent.
    pub async fn item_slugs(&self) -> Result<HashMap<i32, String>> {
        let rows = sqlx::query!("SELECT id, short_name FROM items").fetch_all(&self.pool).await.context(QuerySnafu)?;
        let mut map = HashMap::with_capacity(rows.len());
        for row in rows {
            map.insert(row.id as i32, row.short_name);
        }
        Ok(map)
    }

    /// Localized item names keyed by item id, falling back to the short name.
    pub async fn item_names(&self, locale: &str) -> Result<HashMap<i32, String>> {
        let rows = sqlx::query!(
            r#"SELECT i.id AS "id!", COALESCE(inm.display_name, i.short_name) AS "name!"
             FROM items i LEFT JOIN item_names inm ON inm.item_id = i.id AND inm.locale = ?"#,
            locale,
        )
        .fetch_all(&self.pool)
        .await
        .context(QuerySnafu)?;

        let mut map = HashMap::with_capacity(rows.len());
        for row in rows {
            map.insert(row.id as i32, row.name);
        }
        Ok(map)
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
