use std::collections::HashMap;

use serde::Deserialize;
use shared::{HeroEntry, ItemEntry, MatchDetail, MatchPlayer, MatchSummary};
use snafu::OptionExt;

use crate::{
    client::{Client, Endpoint},
    error::InvalidSteamIdSnafu,
};

const BASE_URL: &str = "https://api.opendota.com/api";

/// The offset between a Steam64 id and the Steam32 `account_id` OpenDota expects.
/// `account_id = steam64 - STEAM64_BASE`.
const STEAM64_BASE: u64 = 76_561_197_960_265_728;

/// Convert a Steam64 id (as stored on [`shared::Friend`]) to the Steam32
/// `account_id` OpenDota keys players by.
fn to_account_id(steam_id: &str) -> crate::Result<u32> {
    steam_id
        .parse::<u64>()
        .ok()
        .and_then(|id| id.checked_sub(STEAM64_BASE))
        .and_then(|id| u32::try_from(id).ok())
        .context(InvalidSteamIdSnafu { steam_id: steam_id.to_owned() })
}

/// Typed access to the OpenDota REST API, sharing the one [`Client`].
///
/// Docs: <https://docs.opendota.com/>
pub struct OpenDota<'a> {
    client: &'a Client,
    api_key: Option<String>,
}

impl Client {
    pub fn opendota(&self, api_key: Option<String>) -> OpenDota<'_> {
        OpenDota { client: self, api_key }
    }
}

impl OpenDota<'_> {
    /// `/players/{account_id}/recentMatches` — the player's recent matches as
    /// compact summaries, truncated to `limit` (newest first).
    pub async fn recent_matches(&self, steam_id: &str, limit: usize) -> crate::Result<Vec<MatchSummary>> {
        let account_id = to_account_id(steam_id)?;
        let mut matches = self
            .client
            .execute(GetRecentMatches {
                account_id,
                api_key: self.api_key.clone(),
            })
            .await?;
        matches.truncate(limit);
        Ok(matches.into_iter().map(Into::into).collect())
    }

    /// `/matches/{match_id}` — the full detail for a single match.
    pub async fn match_detail(&self, match_id: i64) -> crate::Result<MatchDetail> {
        let raw = self
            .client
            .execute(GetMatchDetail {
                match_id,
                api_key: self.api_key.clone(),
            })
            .await?;
        Ok(raw.into())
    }

    /// `/constants/heroes` — the full hero list as id + internal/localized names.
    /// OpenDota's constants are English-only (the `localized_name` ignores any
    /// language), so callers store these under the `en` locale; localized names
    /// still come from Steam. OpenDota's value here is completeness, not l10n.
    pub async fn get_heroes(&self) -> crate::Result<Vec<HeroEntry>> {
        let raw = self.client.execute(GetConstHeroes).await?;
        Ok(raw
            .into_values()
            .map(|hero| HeroEntry {
                id: hero.id,
                internal_name: hero.name,
                display_name: hero.localized_name,
            })
            .collect())
    }

    /// `/constants/items` — the full item list, keyed by short name. More complete
    /// than Stratz (covers every neutral artifact and the `enhancement_*` active
    /// enchantments), but English-only; stored under the `en` locale. Items without
    /// a display name (recipes, internal placeholders) fall back to the short name.
    pub async fn get_items(&self) -> crate::Result<Vec<ItemEntry>> {
        let raw = self.client.execute(GetConstItems).await?;
        Ok(raw
            .into_iter()
            .filter_map(|(short_name, item)| {
                let id = item.id?;
                let display_name = item.dname.unwrap_or_else(|| short_name.clone());
                Some(ItemEntry { id, short_name, display_name })
            })
            .collect())
    }
}

struct GetRecentMatches {
    account_id: u32,
    api_key: Option<String>,
}

impl Endpoint for GetRecentMatches {
    type Response = Vec<RecentMatchRaw>;

    fn url(&self) -> String {
        format!("{BASE_URL}/players/{}/recentMatches", self.account_id)
    }

    fn query(&self) -> Vec<(&'static str, String)> {
        api_key_query(&self.api_key)
    }
}

struct GetMatchDetail {
    match_id: i64,
    api_key: Option<String>,
}

impl Endpoint for GetMatchDetail {
    type Response = MatchDetailRaw;

    fn url(&self) -> String {
        format!("{BASE_URL}/matches/{}", self.match_id)
    }

    fn query(&self) -> Vec<(&'static str, String)> {
        api_key_query(&self.api_key)
    }
}

fn api_key_query(api_key: &Option<String>) -> Vec<(&'static str, String)> {
    api_key.iter().map(|key| ("api_key", key.clone())).collect()
}

/// The constants endpoints serve static JSON files (no key required), each a map
/// keyed differently: heroes by id, items by short name.
struct GetConstHeroes;

impl Endpoint for GetConstHeroes {
    type Response = HashMap<String, HeroConstRaw>;

    fn url(&self) -> String {
        format!("{BASE_URL}/constants/heroes")
    }
}

struct GetConstItems;

impl Endpoint for GetConstItems {
    type Response = HashMap<String, ItemConstRaw>;

    fn url(&self) -> String {
        format!("{BASE_URL}/constants/items")
    }
}

#[derive(Debug, Deserialize)]
struct HeroConstRaw {
    id: i32,
    name: String,
    localized_name: String,
}

/// `id` is absent for a handful of internal entries; `dname` for recipes and
/// placeholders. Both are tolerated by [`OpenDota::get_items`].
#[derive(Debug, Deserialize)]
struct ItemConstRaw {
    id: Option<i32>,
    dname: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RecentMatchRaw {
    match_id: i64,
    hero_id: i32,
    player_slot: i32,
    radiant_win: bool,
    start_time: i64,
    duration: i32,
    game_mode: i32,
    lobby_type: i32,
    kills: i32,
    deaths: i32,
    assists: i32,
    gold_per_min: i32,
    xp_per_min: i32,
    last_hits: i32,
    hero_damage: i32,
    tower_damage: i32,
    hero_healing: i32,
}

impl From<RecentMatchRaw> for MatchSummary {
    fn from(raw: RecentMatchRaw) -> Self {
        Self {
            match_id: raw.match_id,
            hero_id: raw.hero_id,
            player_slot: raw.player_slot,
            radiant_win: raw.radiant_win,
            start_time: raw.start_time,
            duration: raw.duration,
            game_mode: raw.game_mode,
            lobby_type: raw.lobby_type,
            kills: raw.kills,
            deaths: raw.deaths,
            assists: raw.assists,
            gold_per_min: raw.gold_per_min,
            xp_per_min: raw.xp_per_min,
            last_hits: raw.last_hits,
            hero_damage: raw.hero_damage,
            tower_damage: raw.tower_damage,
            hero_healing: raw.hero_healing,
        }
    }
}

/// Only the fields we use are declared; the full `/matches` object carries many
/// more, which `serde` silently ignores.
#[derive(Debug, Deserialize)]
struct MatchDetailRaw {
    match_id: i64,
    radiant_win: bool,
    duration: i32,
    start_time: i64,
    game_mode: i32,
    lobby_type: i32,
    radiant_score: i32,
    dire_score: i32,
    first_blood_time: i32,
    players: Vec<MatchPlayerRaw>,
}

impl From<MatchDetailRaw> for MatchDetail {
    fn from(raw: MatchDetailRaw) -> Self {
        Self {
            match_id: raw.match_id,
            radiant_win: raw.radiant_win,
            duration: raw.duration,
            start_time: raw.start_time,
            game_mode: raw.game_mode,
            lobby_type: raw.lobby_type,
            radiant_score: raw.radiant_score,
            dire_score: raw.dire_score,
            first_blood_time: raw.first_blood_time,
            players: raw.players.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct MatchPlayerRaw {
    account_id: Option<i64>,
    player_slot: i32,
    hero_id: i32,
    personaname: Option<String>,
    kills: i32,
    deaths: i32,
    assists: i32,
    last_hits: i32,
    denies: i32,
    gold_per_min: i32,
    xp_per_min: i32,
    level: i32,
    net_worth: i32,
    hero_damage: i32,
    tower_damage: i32,
    hero_healing: i32,
    item_0: i32,
    item_1: i32,
    item_2: i32,
    item_3: i32,
    item_4: i32,
    item_5: i32,
    backpack_0: i32,
    backpack_1: i32,
    backpack_2: i32,
    item_neutral: i32,
    #[serde(default)]
    item_neutral2: i32,
    #[serde(default)]
    aghanims_scepter: i32,
    #[serde(default)]
    aghanims_shard: i32,
}

impl From<MatchPlayerRaw> for MatchPlayer {
    fn from(raw: MatchPlayerRaw) -> Self {
        Self {
            account_id: raw.account_id,
            player_slot: raw.player_slot,
            hero_id: raw.hero_id,
            personaname: raw.personaname,
            kills: raw.kills,
            deaths: raw.deaths,
            assists: raw.assists,
            last_hits: raw.last_hits,
            denies: raw.denies,
            gold_per_min: raw.gold_per_min,
            xp_per_min: raw.xp_per_min,
            level: raw.level,
            net_worth: raw.net_worth,
            hero_damage: raw.hero_damage,
            tower_damage: raw.tower_damage,
            hero_healing: raw.hero_healing,
            items: [raw.item_0, raw.item_1, raw.item_2, raw.item_3, raw.item_4, raw.item_5],
            backpack: [raw.backpack_0, raw.backpack_1, raw.backpack_2],
            item_neutral: raw.item_neutral,
            item_neutral2: raw.item_neutral2,
            aghanims_scepter: raw.aghanims_scepter > 0,
            aghanims_shard: raw.aghanims_shard > 0,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test environment")]
mod tests {
    #[tokio::test]
    async fn test_recent_matches() {
        dotenvy::dotenv().ok();
        let steam_id = std::env::var("STEAMID").unwrap();
        let client = crate::Client::new(None).unwrap();
        let matches = client.opendota(None).recent_matches(&steam_id, 1).await.unwrap();
        println!("{matches:#?}");
        let detail = client.opendota(None).match_detail(matches[0].match_id).await.unwrap();
        println!("{detail:#?}");
    }
}
