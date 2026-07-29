use std::collections::HashMap;

use serde::Deserialize;

/// A friend's match as it appears in the list view, sourced from OpenDota's
/// `/players/{account_id}/recentMatches` endpoint. The `player_slot` encodes the
/// team: slots `0..=127` are Radiant, `128..` are Dire.
#[derive(Debug, Clone)]
pub struct MatchSummary {
    pub match_id: i64,
    pub hero_id: i32,
    pub player_slot: i32,
    pub radiant_win: bool,
    pub start_time: i64,
    pub duration: i32,
    pub game_mode: i32,
    pub lobby_type: i32,
    pub kills: i32,
    pub deaths: i32,
    pub assists: i32,
    pub gold_per_min: i32,
    pub xp_per_min: i32,
    pub last_hits: i32,
    /// Kept for the planned AI match-critique feature; unused by the UI for now.
    pub hero_damage: i32,
    pub tower_damage: i32,
    pub hero_healing: i32,
}

impl MatchSummary {
    pub fn is_radiant(&self) -> bool {
        self.player_slot < 128
    }

    pub fn won(&self) -> bool {
        self.is_radiant() == self.radiant_win
    }
}

/// `M:SS` from a duration in seconds.
pub fn format_duration(seconds: i32) -> String {
    let seconds = seconds.max(0);
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

/// A coarse "N minutes/hours/days ago" from a unix timestamp, in `locale`.
pub fn format_relative(locale: &str, start_time: i64) -> String {
    let elapsed = (chrono::Utc::now().timestamp() - start_time).max(0);
    let (value, unit) = if elapsed < 3600 {
        (elapsed / 60, "matches-ago-minutes")
    } else if elapsed < 86_400 {
        (elapsed / 3600, "matches-ago-hours")
    } else {
        (elapsed / 86_400, "matches-ago-days")
    };
    format!("{value}{}", i18n::message_in(locale, unit))
}

/// Names for the most common OpenDota `game_mode` ids; others show as `#id`.
pub fn game_mode_label(locale: &str, mode: i32) -> String {
    let key = match mode {
        1 => "match-mode-all-pick",
        2 => "match-mode-captains-mode",
        3 => "match-mode-random-draft",
        4 => "match-mode-single-draft",
        5 => "match-mode-all-random",
        16 => "match-mode-captains-draft",
        22 => "match-mode-ranked-all-pick",
        23 => "match-mode-turbo",
        _ => return format!("#{mode}"),
    };
    i18n::message_in(locale, key)
}

/// Full detail for a single match (`/matches/{match_id}`), reduced to the subset
/// we render or expect to feed the AI critique feature. Holds all ten players.
#[derive(Debug, Clone)]
pub struct MatchDetail {
    pub match_id: i64,
    pub radiant_win: bool,
    pub duration: i32,
    pub start_time: i64,
    pub game_mode: i32,
    pub lobby_type: i32,
    pub radiant_score: i32,
    pub dire_score: i32,
    /// Kept for the planned AI match-critique feature.
    pub first_blood_time: i32,
    pub players: Vec<MatchPlayer>,
    /// The match's teamfights, in order. Fed to the AI critique feature as
    /// context, not shown in the UI or the Telegram card. Only populated on a
    /// fresh OpenDota fetch — the desktop DB does not persist it.
    pub teamfights: Vec<Teamfight>,
    /// All-chat and chat-wheel messages, likewise AI-only context that the DB
    /// does not persist.
    pub chat: Vec<ChatMessage>,
}

/// One teamfight window. `players` is fixed at ten entries, index-aligned with
/// [`MatchDetail::players`] ordered by ascending `player_slot`, which is how the
/// AI maps a fight's per-player deltas back to a hero.
#[derive(Debug, Clone, Deserialize)]
pub struct Teamfight {
    pub start: i32,
    pub end: i32,
    pub last_death: i32,
    pub deaths: i32,
    #[serde(default)]
    pub players: Vec<TeamfightPlayer>,
}

/// One player's contribution to a single [`Teamfight`].
#[derive(Debug, Clone, Deserialize)]
pub struct TeamfightPlayer {
    #[serde(default)]
    pub deaths: i32,
    #[serde(default)]
    pub damage: i32,
    #[serde(default)]
    pub healing: i32,
    #[serde(default)]
    pub gold_delta: i32,
    #[serde(default)]
    pub xp_delta: i32,
}

/// One chat event. `kind` is `"chat"` for typed all-chat or `"chatwheel"` for a
/// wheel phrase; `key` is the message text or the wheel phrase id accordingly.
#[derive(Debug, Clone, Deserialize)]
pub struct ChatMessage {
    pub time: i32,
    #[serde(rename = "type")]
    pub kind: String,
    pub key: String,
    #[serde(default)]
    pub player_slot: Option<i32>,
}

/// One entry of a player's timed [`MatchPlayer::purchase_log`]: a negative
/// `time` is a pre-horn purchase. `key` is the item's short name (e.g. `blink`).
#[derive(Debug, Clone, Deserialize)]
pub struct PurchaseEvent {
    pub time: i32,
    pub key: String,
}

/// One player within a [`MatchDetail`]. Items are the final inventory state
/// (`item_0..item_5`), backpack and the neutral item; `0` means an empty slot.
#[derive(Debug, Clone)]
pub struct MatchPlayer {
    /// `None` for players with an anonymous/private profile.
    pub account_id: Option<i64>,
    pub player_slot: i32,
    pub hero_id: i32,
    pub personaname: Option<String>,
    pub kills: i32,
    pub deaths: i32,
    pub assists: i32,
    pub last_hits: i32,
    pub denies: i32,
    pub gold_per_min: i32,
    pub xp_per_min: i32,
    pub level: i32,
    pub net_worth: i32,
    /// Kept for the planned AI match-critique feature.
    pub hero_damage: i32,
    pub tower_damage: i32,
    pub hero_healing: i32,
    pub items: [i32; 6],
    pub backpack: [i32; 3],
    pub item_neutral: i32,
    /// The neutral item's active enchantment (`item_neutral2`); `0` when none.
    pub item_neutral2: i32,
    /// Whether the player had Aghanim's Scepter / Shard by the end of the match.
    pub aghanims_scepter: bool,
    pub aghanims_shard: bool,
    /// OpenDota's post-game estimate of the player's MMR, and their ranked medal
    /// (`rank_tier`: tens digit = medal 1–8, units = stars). Both are `None` for
    /// an anonymous profile OpenDota could not resolve.
    pub computed_mmr: Option<f64>,
    pub rank_tier: Option<i32>,
    /// The hero facet the player picked (0 when unknown).
    pub hero_variant: i32,
    /// Parsed lane assignment (`lane_role`: 1 safe, 2 mid, 3 off, 4 jungle) and
    /// the share of teamfights the player took part in (0.0–1.0). `None` when the
    /// replay was not parsed.
    pub lane_role: Option<i32>,
    pub teamfight_participation: Option<f64>,
    /// Vision and jungle-economy counts, the shorthand for how a support played.
    pub observers_placed: i32,
    pub sentries_placed: i32,
    pub camps_stacked: i32,
    pub creeps_stacked: i32,
    /// Every item the player bought, as short-name → count, and the same in timed
    /// order. Build-order context for the AI critique; not shown in the UI.
    pub purchase: HashMap<String, i32>,
    pub purchase_log: Vec<PurchaseEvent>,
}

impl MatchPlayer {
    pub fn is_radiant(&self) -> bool {
        self.player_slot < 128
    }

    /// The player's final inventory as short, human-readable names via
    /// `item_names` (id → localized name), neutral items included, empty slots
    /// dropped. Mirrors the desktop match view's text fallback so the Telegram
    /// card and the app agree on one item list.
    pub fn item_line(&self, item_names: &HashMap<i32, String>) -> String {
        let names = self
            .items
            .iter()
            .chain(std::iter::once(&self.item_neutral))
            .chain(std::iter::once(&self.item_neutral2))
            .filter(|&&id| id != 0)
            .map(|id| item_names.get(id).cloned().unwrap_or_else(|| format!("#{id}")))
            .collect::<Vec<_>>();
        if names.is_empty() { "—".to_owned() } else { names.join(" · ") }
    }
}
