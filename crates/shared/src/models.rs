use std::{collections::HashMap, time::Duration};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameType {
    Dota2,
    Cs2,
    LeagueOfLegends,
}

impl GameType {
    pub fn display_name(&self) -> &'static str {
        match self {
            GameType::Dota2 => "Dota 2",
            GameType::Cs2 => "Counter-Strike 2",
            GameType::LeagueOfLegends => "League of Legends",
        }
    }

    pub fn icon_name(&self) -> &'static str {
        match self {
            GameType::Dota2 => "dota2",
            GameType::Cs2 => "cs2",
            GameType::LeagueOfLegends => "lol",
        }
    }
}

/// Match outcome
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchOutcome {
    Victory,
    Defeat,
    Draw,
    Unknown,
}

/// Team side
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Team {
    Radiant,
    Dire,
    Unknown,
}

/// A game match (game-agnostic)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Match {
    pub id: String,
    pub game: GameType,
    pub start_time: DateTime<Utc>,
    #[serde(with = "duration_serde")]
    pub duration: Duration,
    pub outcome: MatchOutcome,
    pub mode: String,
    pub lobby_type: Option<String>,
    pub players: Vec<MatchPlayer>,
    /// Game-specific extra data
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// A player in a match
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchPlayer {
    pub player_id: String,
    pub display_name: String,
    pub team: Team,
    pub character_id: Option<u32>,
    pub character_name: Option<String>,
    pub stats: PlayerStats,
    pub items: Vec<u32>,
    /// Whether this is the tracked player
    pub is_tracked: bool,
}

/// Player statistics in a match
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlayerStats {
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
    pub level: Option<u32>,
    pub gold: Option<u32>,
    pub last_hits: Option<u32>,
    pub denies: Option<u32>,
    pub gpm: Option<u32>,
    pub xpm: Option<u32>,
    /// Game-specific custom stats
    #[serde(default)]
    pub custom: HashMap<String, serde_json::Value>,
}

/// Player summary for search results and watch lists
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerSummary {
    pub id: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub game: GameType,
    pub rank: Option<String>,
    pub last_match: Option<DateTime<Utc>>,
}

/// Player profile with detailed information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerProfile {
    pub id: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub game: GameType,
    pub rank: Option<String>,
    pub wins: u32,
    pub losses: u32,
    pub last_match: Option<DateTime<Utc>>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Hero/Champion/Agent representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub id: u32,
    pub name: String,
    pub localized_name: String,
    pub game: GameType,
    pub primary_attribute: Option<String>,
    pub attack_type: Option<String>,
    pub roles: Vec<String>,
    pub image_url: Option<String>,
}

/// Item representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: u32,
    pub name: String,
    pub localized_name: String,
    pub game: GameType,
    pub cost: Option<u32>,
    pub description: Option<String>,
    pub image_url: Option<String>,
}

/// Notification to be sent to user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub title: String,
    pub body: String,
    pub icon: Option<String>,
    pub match_id: Option<String>,
    pub player_id: Option<String>,
}

/// Duration serialization helper
mod duration_serde {
    use std::time::Duration;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}
