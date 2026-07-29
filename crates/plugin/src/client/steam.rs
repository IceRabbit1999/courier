use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Deserialize;
use shared::{HeroEntry, PersonaState};
use snafu::OptionExt;
use tracing::warn;

use crate::{
    client::{Client, Endpoint},
    error::UnresolvedProfileSnafu,
};

const BASE_URL: &str = "https://api.steampowered.com";

/// Map an app locale (see [`i18n::SUPPORTED_LOCALES`]) to a Steam `language` code.
/// The DOTA2 econ endpoints use ISO-style codes (`zh`), not the Steam store codes
/// (`schinese`).
fn steam_language(locale: &str) -> &'static str {
    match locale {
        "zh-CN" => "zh",
        _ => "english",
    }
}

/// Typed access to the Steam Web API, sharing the one [`Client`].
///
/// Docs: <https://developer.valvesoftware.com/wiki/Steam_Web_API>
pub struct Steam<'a> {
    client: &'a Client,
    key: String,
}

impl Client {
    pub fn steam(&self, key: impl Into<String>) -> Steam<'_> {
        Steam { client: self, key: key.into() }
    }
}

impl Steam<'_> {
    /// `ISteamUser/GetFriendList` — the public friends of a profile.
    ///
    /// `GET 'https://api.steampowered.com/ISteamUser/GetFriendList/v0001/?key=<KEY>&steamid=<ID>&relationship=friend'`
    /// ```json
    /// { "friendslist": { "friends": [
    ///   { "steamid": "76561198050940782", "relationship": "friend", "friend_since": 1516276673 }
    /// ] } }
    /// ```
    pub async fn get_friend_list(&self, steamid: &str) -> crate::Result<Vec<Friend>> {
        let response = self
            .client
            .execute(GetFriendList {
                key: self.key.clone(),
                steamid: steamid.to_owned(),
            })
            .await?;
        Ok(response.friends_list.friends)
    }

    /// `ISteamUser/GetPlayerSummaries` — profile summaries for up to 100 ids.
    ///
    /// `GET 'https://api.steampowered.com/ISteamUser/GetPlayerSummaries/v0002/?key=<KEY>&steamids=<ID,ID>&format=json'`
    /// ```json
    /// { "response": { "players": [ {
    ///   "steamid": "<STEAM_ID>", "communityvisibilitystate": 3, "profilestate": 1,
    ///   "personaname": "<PLAYER_NAME>", "profileurl": "https://steamcommunity.com/profiles/<STEAM_ID>/",
    ///   "avatar": "...", "avatarmedium": "...", "avatarfull": "...", "avatarhash": "...",
    ///   "lastlogoff": 1780155532, "personastate": 0, "primaryclanid": "103582791429521408",
    ///   "timecreated": 1491981282, "personastateflags": 0, "loccountrycode": "CN"
    /// } ] } }
    /// ```
    pub async fn get_player_summaries(&self, steamids: &[&str]) -> crate::Result<Vec<PlayerSummary>> {
        let response = self
            .client
            .execute(GetPlayerSummaries {
                key: self.key.clone(),
                steamids: steamids.join(","),
            })
            .await?;
        Ok(response.response.players)
    }

    /// The user's friends as watch-list entries: the friend list merged with the
    /// player summaries (which carry persona, avatar and online state). Summaries
    /// are fetched in chunks of 100 (the endpoint's per-call id limit).
    pub async fn get_friends(&self, steam_id: &str) -> crate::Result<Vec<shared::Friend>> {
        let friends = self.get_friend_list(steam_id).await?;
        if friends.is_empty() {
            return Ok(Vec::new());
        }

        let friend_since: HashMap<&str, i64> = friends.iter().map(|f| (f.steamid.as_str(), f.friend_since)).collect();
        let ids = friends.iter().map(|f| f.steamid.as_str()).collect::<Vec<_>>();
        let summaries = self.summaries_for(&ids).await?;
        Ok(merge_summaries(&friend_since, summaries))
    }

    /// Refresh only the dynamic fields (persona state, current game, last log-off)
    /// of an already-known set of friends by re-fetching their player summaries.
    /// Unlike [`Self::get_friends`] it does not re-fetch the friend list, so newly
    /// added friends won't appear; `friend_since` is preserved from `friends`.
    pub async fn refresh_statuses(&self, friends: &[shared::Friend]) -> crate::Result<Vec<shared::Friend>> {
        if friends.is_empty() {
            return Ok(Vec::new());
        }

        let friend_since = friends.iter().map(|f| (f.steam_id.as_str(), f.friend_since)).collect::<HashMap<&str, i64>>();
        let ids = friends.iter().map(|f| f.steam_id.as_str()).collect::<Vec<_>>();
        let summaries = self.summaries_for(&ids).await?;
        let mut merged = merge_summaries(&friend_since, summaries);

        // Recently played games are only worth fetching for friends who are online
        // (one request each); offline friends keep an empty list. A single failed
        // fetch is logged and skipped rather than failing the whole refresh.
        let _ = futures::future::join_all(merged.iter_mut().map(|f| async move {
            if f.persona_state.is_online() {
                let games = self
                    .get_recent_games(&f.steam_id)
                    .await
                    .inspect_err(|e| warn!(steanid=%f.steam_id, "Failed to fetch recent games: {e:?}"))?;
                f.recent_games = games;
            }
            Ok::<_, crate::Error>(())
        }))
        .await;
        Ok(merged)
    }

    /// Refresh only the dynamic fields (persona state, current game, last log-off)
    /// of an already-known set of follows by re-fetching their player summaries.
    /// `added_at` is preserved from `follows`.
    pub async fn refresh_follow_statuses(&self, follows: &[shared::Follow]) -> crate::Result<Vec<shared::Follow>> {
        if follows.is_empty() {
            return Ok(Vec::new());
        }

        let added_at = follows.iter().map(|f| (f.steam_id.as_str(), f.added_at)).collect::<HashMap<&str, i64>>();
        let ids = follows.iter().map(|f| f.steam_id.as_str()).collect::<Vec<_>>();
        let summaries = self.summaries_for(&ids).await?;
        let mut merged = merge_follow_summaries(&added_at, summaries);

        let _ = futures::future::join_all(merged.iter_mut().map(|f| async move {
            if f.persona_state.is_online() {
                let games = self
                    .get_recent_games(&f.steam_id)
                    .await
                    .inspect_err(|e| warn!(steamid=%f.steam_id, "Failed to fetch recent games: {e:?}"))?;
                f.recent_games = games;
            }
            Ok::<_, crate::Error>(())
        }))
        .await;
        Ok(merged)
    }

    /// `ISteamUser/ResolveVanityURL` — turn a vanity name (the `<name>` in
    /// `steamcommunity.com/id/<name>`) into a steamid64. Returns `None` if Steam
    /// has no match for it.
    ///
    /// `GET 'https://api.steampowered.com/ISteamUser/ResolveVanityURL/v0001/?key=<KEY>&vanityurl=<NAME>'`
    pub async fn resolve_vanity_url(&self, vanity: &str) -> crate::Result<Option<String>> {
        let response = self
            .client
            .execute(ResolveVanityUrl {
                key: self.key.clone(),
                vanity_url: vanity.to_owned(),
            })
            .await?;
        Ok(if response.response.success == 1 { response.response.steam_id } else { None })
    }

    /// Resolve a pasted Steam64 id or profile URL to a steamid64. Accepts a bare
    /// id, a `.../profiles/<id>` URL, a `.../id/<vanity>` URL, or a bare vanity
    /// name.
    async fn resolve_steam_id(&self, input: &str) -> crate::Result<String> {
        let trimmed = input.trim().trim_end_matches('/');

        if let Some(id) = trimmed.rsplit("/profiles/").next().filter(|s| *s != trimmed) {
            return Ok(id.to_owned());
        }

        let vanity = trimmed.rsplit("/id/").next().filter(|s| *s != trimmed).unwrap_or(trimmed);
        if !vanity.is_empty() && vanity.chars().all(|c| c.is_ascii_digit()) {
            return Ok(vanity.to_owned());
        }

        self.resolve_vanity_url(vanity).await?.context(UnresolvedProfileSnafu { input: input.to_owned() })
    }

    /// Turn a pasted Steam64 id or profile URL into a [`shared::Follow`] seed by
    /// resolving it, then fetching its player summary and recent games.
    pub async fn resolve_and_summarize(&self, input: &str) -> crate::Result<shared::Follow> {
        let steam_id = self.resolve_steam_id(input).await?;
        let summary = self
            .get_player_summaries(&[&steam_id])
            .await?
            .into_iter()
            .next()
            .context(UnresolvedProfileSnafu { input: input.to_owned() })?;
        let recent_games = self.get_recent_games(&steam_id).await.unwrap_or_default();

        Ok(shared::Follow {
            added_at: unix_now(),
            steam_id: summary.steam_id,
            persona_name: summary.persona_name,
            avatar: summary.avatar,
            profile_url: summary.profile_url,
            persona_state: PersonaState::from_i32(summary.persona_state),
            last_log_off: summary.last_log_off,
            game_extra_info: summary.game_extra_info,
            recent_games,
            tracked: false,
        })
    }

    /// `IPlayerService/GetRecentlyPlayedGames` — the games a single profile has played in the
    /// trailing two weeks. Returns an empty list for private profiles (the endpoint omits
    /// `games` rather than erroring).
    ///
    /// `GET 'https://api.steampowered.com/IPlayerService/GetRecentlyPlayedGames/v0001/?key=<KEY>&steamid=<ID>'`
    pub async fn get_recent_games(&self, steamid: &str) -> crate::Result<Vec<shared::RecentGame>> {
        let response = self
            .client
            .execute(GetRecentPlayedGameList {
                key: self.key.clone(),
                steamid: steamid.to_owned(),
            })
            .await?;
        Ok(response
            .response
            .games
            .into_iter()
            .map(|game| shared::RecentGame {
                app_id: game.appid,
                name: game.name,
                playtime_2weeks: game.playtime_2weeks,
                playtime_forever: game.playtime_forever,
                img_icon_url: game.img_icon_url,
            })
            .collect())
    }

    /// Fetch player summaries for `ids` in chunks of 100 (the endpoint's per-call limit).
    async fn summaries_for(&self, ids: &[&str]) -> crate::Result<Vec<PlayerSummary>> {
        let summaries = futures::future::join_all(ids.chunks(100).map(|chunk| async move {
            let summary = self
                .get_player_summaries(chunk)
                .await
                .inspect_err(|e| warn!(steam_ids = ?chunk, "Failed to fetch plaer summary: {e}"))?;
            Ok::<_, crate::Error>(summary)
        }))
        .await
        .into_iter()
        .filter_map(|r| r.ok())
        .flatten()
        .collect::<Vec<_>>();

        Ok(summaries)
    }

    /// `IEconDOTA2_570/GetHeroes` — the full hero list for one locale. Heroes (unlike
    /// items) come from Steam directly, since its localized `language` param works.
    pub async fn get_heroes(&self, locale: &str) -> crate::Result<Vec<HeroEntry>> {
        let response = self
            .client
            .execute(GetHeroList {
                key: self.key.clone(),
                language: steam_language(locale).to_owned(),
            })
            .await?;
        Ok(response
            .result
            .heroes
            .into_iter()
            .map(|hero| HeroEntry {
                id: hero.id,
                internal_name: hero.name,
                display_name: hero.localized_name,
            })
            .collect())
    }
}

/// Merge player summaries with their `friend_since` timestamps into watch-list entries.
fn merge_summaries(friend_since: &HashMap<&str, i64>, summaries: Vec<PlayerSummary>) -> Vec<shared::Friend> {
    summaries
        .into_iter()
        .map(|summary| shared::Friend {
            friend_since: friend_since.get(summary.steam_id.as_str()).copied().unwrap_or_default(),
            persona_state: PersonaState::from_i32(summary.persona_state),
            steam_id: summary.steam_id,
            persona_name: summary.persona_name,
            avatar: summary.avatar,
            profile_url: summary.profile_url,
            last_log_off: summary.last_log_off,
            game_extra_info: summary.game_extra_info,
            recent_games: Vec::new(),
        })
        .collect()
}

/// Merge player summaries with their `added_at` timestamps into follow entries.
fn merge_follow_summaries(added_at: &HashMap<&str, i64>, summaries: Vec<PlayerSummary>) -> Vec<shared::Follow> {
    summaries
        .into_iter()
        .map(|summary| shared::Follow {
            added_at: added_at.get(summary.steam_id.as_str()).copied().unwrap_or_default(),
            persona_state: PersonaState::from_i32(summary.persona_state),
            steam_id: summary.steam_id,
            persona_name: summary.persona_name,
            avatar: summary.avatar,
            profile_url: summary.profile_url,
            last_log_off: summary.last_log_off,
            game_extra_info: summary.game_extra_info,
            recent_games: Vec::new(),
            tracked: false,
        })
        .collect()
}

struct GetFriendList {
    key: String,
    steamid: String,
}

impl Endpoint for GetFriendList {
    type Response = FriendListResponse;

    fn url(&self) -> String {
        format!("{BASE_URL}/ISteamUser/GetFriendList/v0001/")
    }

    fn query(&self) -> Vec<(&'static str, String)> {
        vec![("key", self.key.clone()), ("steamid", self.steamid.clone()), ("relationship", "friend".to_owned())]
    }
}

struct GetPlayerSummaries {
    key: String,
    steamids: String,
}

impl Endpoint for GetPlayerSummaries {
    type Response = PlayerSummariesResponse;

    fn url(&self) -> String {
        format!("{BASE_URL}/ISteamUser/GetPlayerSummaries/v0002/")
    }

    fn query(&self) -> Vec<(&'static str, String)> {
        vec![("key", self.key.clone()), ("steamids", self.steamids.clone()), ("format", "json".to_owned())]
    }
}

#[derive(Debug, Deserialize)]
struct FriendListResponse {
    #[serde(rename = "friendslist")]
    friends_list: FriendList,
}

#[derive(Debug, Deserialize)]
struct FriendList {
    friends: Vec<Friend>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Friend {
    pub steamid: String,
    pub relationship: String,
    pub friend_since: i64,
}

#[derive(Debug, Deserialize)]
struct PlayerSummariesResponse {
    response: PlayerSummaries,
}

#[derive(Debug, Deserialize)]
struct PlayerSummaries {
    players: Vec<PlayerSummary>,
}

/// Fields beyond the always-public block are absent for private profiles, hence `Option`.
#[derive(Debug, Clone, Deserialize)]
pub struct PlayerSummary {
    #[serde(rename = "steamid")]
    pub steam_id: String,
    #[serde(rename = "personaname")]
    pub persona_name: String,
    #[serde(rename = "profileurl")]
    pub profile_url: String,
    pub avatar: String,
    // pub avatarmedium: String,
    // pub avatarfull: String,
    // pub avatarhash: String,
    #[serde(rename = "personastate")]
    pub persona_state: i32,
    // pub communityvisibilitystate: i32,
    // pub profilestate: Option<i32>,
    #[serde(rename = "lastlogoff")]
    pub last_log_off: Option<i64>,
    // pub primaryclanid: Option<String>,
    // pub timecreated: Option<i64>,
    // pub personastateflags: Option<i64>,
    // pub loccountrycode: Option<String>,
    #[serde(rename = "gameid")]
    pub game_id: Option<String>,
    #[serde(rename = "gameextrainfo")]
    pub game_extra_info: Option<String>,
}

struct ResolveVanityUrl {
    key: String,
    vanity_url: String,
}

impl Endpoint for ResolveVanityUrl {
    type Response = ResolveVanityUrlResponse;

    fn url(&self) -> String {
        format!("{BASE_URL}/ISteamUser/ResolveVanityURL/v0001/")
    }

    fn query(&self) -> Vec<(&'static str, String)> {
        vec![("key", self.key.clone()), ("vanityurl", self.vanity_url.clone())]
    }
}

#[derive(Debug, Deserialize)]
struct ResolveVanityUrlResponse {
    response: ResolveVanityUrlResult,
}

/// `steamid` is absent when `success != 1` (no match).
#[derive(Debug, Deserialize)]
struct ResolveVanityUrlResult {
    success: i32,
    #[serde(rename = "steamid")]
    steam_id: Option<String>,
}

fn unix_now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or_default()
}

struct GetHeroList {
    key: String,
    language: String,
}

#[derive(Debug, Deserialize)]
struct HeroListResponse {
    result: HeroList,
}

#[derive(Debug, Deserialize)]
struct HeroList {
    heroes: Vec<Hero>,
}

#[derive(Debug, Deserialize)]
struct Hero {
    name: String,
    id: i32,
    localized_name: String,
}

impl Endpoint for GetHeroList {
    type Response = HeroListResponse;

    fn url(&self) -> String {
        format!("{BASE_URL}/IEconDOTA2_570/GetHeroes/v1/")
    }

    fn query(&self) -> Vec<(&'static str, String)> {
        vec![("key", self.key.clone()), ("language", self.language.clone())]
    }
}

struct GetRecentPlayedGameList {
    key: String,
    steamid: String,
}

impl Endpoint for GetRecentPlayedGameList {
    type Response = RecentlyPlayedGamesResponse;

    fn url(&self) -> String {
        format!("{BASE_URL}/IPlayerService/GetRecentlyPlayedGames/v0001/")
    }

    fn query(&self) -> Vec<(&'static str, String)> {
        vec![("key", self.key.clone()), ("steamid", self.steamid.clone()), ("format", "json".to_owned())]
    }
}

#[derive(Debug, Deserialize)]
struct RecentlyPlayedGamesResponse {
    response: RecentlyPlayedGames,
}

/// A private profile yields `{ "response": {} }` (no `games`), so every field
/// defaults.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RecentlyPlayedGames {
    games: Vec<RecentGame>,
}

#[derive(Debug, Deserialize)]
struct RecentGame {
    appid: i64,
    name: String,
    playtime_2weeks: i64,
    playtime_forever: i64,
    #[serde(default)]
    img_icon_url: String,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test environment")]
mod tests {

    #[tokio::test]
    async fn test_get_friend_list() {
        dotenvy::dotenv().ok();
        let key = std::env::var("STEAM_API_KEY").unwrap();
        let steamid = std::env::var("STEAMID").unwrap();
        let client = crate::Client::new(None).unwrap();
        let steam = client.steam(key);
        let friend_list = steam.get_friend_list(&steamid).await.unwrap();
        println!("{:?}", friend_list);
        let steamids = friend_list.iter().map(|f| f.steamid.as_str()).collect::<Vec<&str>>();
        println!("{}", steamids.len());
        let player_summaries = steam.get_player_summaries(&steamids).await.unwrap();
        println!("{:#?}", player_summaries);
    }
}
