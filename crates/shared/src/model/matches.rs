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
}

impl MatchPlayer {
    pub fn is_radiant(&self) -> bool {
        self.player_slot < 128
    }
}
