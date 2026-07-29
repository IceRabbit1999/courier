//! Wire types shared between the desktop client (`plugin`) and the hub service
//! (`bot`). Keeping them here gives the HTTP contract a single source of truth.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::model::matches::{MatchDetail, format_duration, format_relative, game_mode_label};

/// An already-localized push payload, in plain text: a one-line `title` and a
/// multi-line `body`. The renderer owns i18n and the hero name map; channels
/// only deliver it, and are free to add their own markup around the two fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub title: String,
    pub body: String,
}

impl Notification {
    /// Render the tracked player's match as a rich, single-player card in
    /// `locale`, from the full [`MatchDetail`] fetched from OpenDota. `player_slot`
    /// (carried over from the recent-matches summary) picks the tracked player's
    /// row out of the ten. `hero_names` and `item_names` map ids to the localized
    /// names synced from the desktop app, so the card can name the hero and the
    /// final inventory. Both the desktop tracker and the hub's offline tracker go
    /// through here, so the two can never show a different card for the same match.
    pub fn match_card(locale: &str, persona_name: &str, detail: &MatchDetail, player_slot: i32, hero_names: &HashMap<i32, String>, item_names: &HashMap<i32, String>) -> Self {
        let msg = |key: &str| i18n::message_in(locale, key);

        // The side, and therefore the result, is known from the slot alone, so the
        // headline holds up even in the impossible case that the row is missing.
        let won = (player_slot < 128) == detail.radiant_win;
        let side = msg(if player_slot < 128 { "matches-radiant" } else { "matches-dire" });
        let title = format!(
            "{} {persona_name} · {}",
            if won { "🏆" } else { "💀" },
            msg(if won { "notify-match-win" } else { "notify-match-loss" })
        );
        let footer = format!("{side} · {} : {} · {}", detail.radiant_score, detail.dire_score, format_relative(locale, detail.start_time));

        let Some(me) = detail.players.iter().find(|p| p.player_slot == player_slot) else {
            return Self { title, body: footer };
        };

        let hero = hero_names.get(&me.hero_id).cloned().unwrap_or_else(|| format!("#{}", me.hero_id));
        let net_worth = if me.net_worth >= 1000 {
            format!("{:.1}k", me.net_worth as f32 / 1000.0)
        } else {
            me.net_worth.to_string()
        };

        let mut lines = vec![
            format!("{hero} · {} · {}", game_mode_label(locale, detail.game_mode), format_duration(detail.duration)),
            format!("⚔️ {} / {} / {}  ·  💰 {net_worth}", me.kills, me.deaths, me.assists),
            format!(
                "{} {}  ·  {} {}  ·  {} {}",
                msg("stats-gpm"),
                me.gold_per_min,
                msg("stats-xpm"),
                me.xp_per_min,
                msg("stats-level"),
                me.level
            ),
            format!("{} {}  ·  {} {}", msg("stats-last-hits"), me.last_hits, msg("stats-denies"), me.denies),
        ];
        if let Some(mmr) = me.computed_mmr {
            lines.push(format!("📈 ~{} {}", mmr.round() as i64, msg("notify-match-mmr")));
        }
        let upgrades = [
            me.aghanims_scepter.then(|| msg("matches-aghanim-scepter")),
            me.aghanims_shard.then(|| msg("matches-aghanim-shard")),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
        if !upgrades.is_empty() {
            lines.push(format!("✨ {}", upgrades.join(" · ")));
        }
        lines.push(format!("🎒 {}: {}", msg("match-items"), me.item_line(item_names)));
        lines.push(footer);

        Self { title, body: lines.join("\n") }
    }
}

/// Reply to `POST /link/new`: the single-use linking `token`, the bearer
/// `secret` that will authenticate this install once the chat binds, and the
/// Telegram deep link the user opens. The hub keeps only digests of the first
/// two, so this response is the one and only time it can hand them over.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkNew {
    pub token: String,
    pub secret: String,
    pub deep_link: String,
}

/// Reply to `GET /link/status`. Once `linked` is true, `subscriber_id` is
/// populated and the client pairs it with the secret it already holds,
/// discarding the token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkStatus {
    pub linked: bool,
    pub subscriber_id: Option<String>,
}

/// Body of `POST /notify`. The subscriber is identified by the bearer secret, so
/// only the payload travels here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotifyRequest {
    pub notification: Notification,
}

/// One follow the hub should track on the subscriber's behalf when offline
/// mode is on. `steam_id` is the Steam64 id, as everywhere else in the app.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedAccount {
    pub steam_id: String,
    pub persona_name: String,
}

/// Body of `POST /sync`: a full snapshot of what the hub may know about this
/// subscriber. The tracked set is *replaced*, not merged, so the app stays the
/// source of truth; `hero_names` and `item_names` carry the app's localized
/// names for the hub to render offline-mode pushes with.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequest {
    pub accounts: Vec<TrackedAccount>,
    pub locale: String,
    pub hero_names: HashMap<i32, String>,
    #[serde(default)]
    pub item_names: HashMap<i32, String>,
    pub offline_mode: bool,
}

/// Success reply for side-effecting endpoints (`/notify`, `/unlink`). A failure
/// arrives as a non-2xx status (which the client surfaces as an error) rather
/// than `ok: false`, but the flag is checked defensively.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ack {
    #[serde(default)]
    pub ok: bool,
}
