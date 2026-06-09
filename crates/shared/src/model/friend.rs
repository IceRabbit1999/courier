/// A friend as shown in the watch list: the Steam friend-list entry merged with
/// the player summary (persona, avatar, online state).
#[derive(Debug, Clone)]
pub struct Friend {
    pub steam_id: String,
    pub friend_since: i64,
    pub persona_name: String,
    pub avatar: String,
    pub profile_url: String,
    pub persona_state: PersonaState,
    pub last_log_off: Option<i64>,
    pub game_extra_info: Option<String>,
}

impl Friend {
    /// Whether the friend is currently in a game (any title).
    pub fn in_game(&self) -> bool {
        self.game_extra_info.is_some()
    }
}

/// Steam `personastate`. See <https://developer.valvesoftware.com/wiki/Steam_Web_API#GetPlayerSummaries_(v0002)>.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersonaState {
    Offline = 0,
    Online = 1,
    Busy = 2,
    Away = 3,
    Snooze = 4,
    LookingToTrade = 5,
    LookingToPlay = 6,
}

impl PersonaState {
    pub fn from_i32(value: i32) -> Self {
        match value {
            1 => Self::Online,
            2 => Self::Busy,
            3 => Self::Away,
            4 => Self::Snooze,
            5 => Self::LookingToTrade,
            6 => Self::LookingToPlay,
            _ => Self::Offline,
        }
    }

    pub fn is_online(self) -> bool {
        !matches!(self, Self::Offline)
    }
}
