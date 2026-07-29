use super::friend::{PersonaState, RecentGame};

/// A player the user has chosen to track, independent of the Steam friend graph
/// (a pro player, a stranger, or the user themselves).
#[derive(Debug, Clone)]
pub struct Follow {
    pub steam_id: String,
    pub added_at: i64,
    pub persona_name: String,
    pub avatar: String,
    pub profile_url: String,
    pub persona_state: PersonaState,
    pub last_log_off: Option<i64>,
    pub game_extra_info: Option<String>,
    pub recent_games: Vec<RecentGame>,
    /// Whether this follow is enrolled in background match tracking. Only a subset
    /// of follows are tracked; the rest are watch-list-only.
    pub tracked: bool,
}

impl Follow {
    /// Whether the player is currently in a game (any title).
    pub fn in_game(&self) -> bool {
        self.game_extra_info.is_some()
    }
}
