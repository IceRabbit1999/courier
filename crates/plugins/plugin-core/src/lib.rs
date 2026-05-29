use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use shared::{Match, MatchPlayer, Notification, PlayerProfile, PlayerSummary, Result};

#[async_trait]
pub trait GameSource: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &str;

    fn icon(&self) -> Option<&str> {
        None
    }

    async fn search_players(&self, query: &str) -> Result<Vec<PlayerSummary>>;

    async fn fetch_player(&self, player_id: &str) -> Result<PlayerProfile>;

    async fn fetch_matches(&self, player_id: &str, limit: u32) -> Result<Vec<Match>>;

    async fn fetch_match_detail(&self, match_id: &str) -> Result<Match>;
}

/// A notification output plugin
#[async_trait]
pub trait Notifier: Send + Sync {
    /// Unique identifier for this notifier
    fn id(&self) -> &'static str;

    /// Display name for this notifier
    fn name(&self) -> &str;

    /// Whether this notifier is enabled
    fn is_enabled(&self) -> bool;

    /// Send a notification
    async fn send(&self, notification: &Notification) -> Result<()>;
}

/// Plugin registry managing all available plugins
pub struct PluginRegistry {
    game_sources: HashMap<&'static str, Arc<dyn GameSource>>,
    notifiers: HashMap<&'static str, Arc<dyn Notifier>>,
}

impl PluginRegistry {
    /// Create a new empty plugin registry
    pub fn new() -> Self {
        Self {
            game_sources: HashMap::new(),
            notifiers: HashMap::new(),
        }
    }

    /// Register a game source plugin
    pub fn register_source<S: GameSource + 'static>(&mut self, source: S) {
        self.game_sources.insert(source.id(), Arc::new(source));
    }

    /// Register a notifier plugin
    pub fn register_notifier<N: Notifier + 'static>(&mut self, notifier: N) {
        self.notifiers.insert(notifier.id(), Arc::new(notifier));
    }

    /// Get a game source by ID
    pub fn get_source(&self, id: &str) -> Option<Arc<dyn GameSource>> {
        self.game_sources.get(id).cloned()
    }

    /// Get a notifier by ID
    pub fn get_notifier(&self, id: &str) -> Option<Arc<dyn Notifier>> {
        self.notifiers.get(id).cloned()
    }

    /// Get all registered game sources
    pub fn sources(&self) -> impl Iterator<Item = &Arc<dyn GameSource>> {
        self.game_sources.values()
    }

    /// Get all registered notifiers
    pub fn notifiers(&self) -> impl Iterator<Item = &Arc<dyn Notifier>> {
        self.notifiers.values()
    }

    /// Get all source IDs
    pub fn source_ids(&self) -> Vec<&'static str> {
        self.game_sources.keys().copied().collect()
    }

    /// Get all notifier IDs
    pub fn notifier_ids(&self) -> Vec<&'static str> {
        self.notifiers.keys().copied().collect()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper trait for formatting player stats in a KDA string
pub trait KdaDisplay {
    fn kda_string(&self) -> String;
    fn kda_ratio(&self) -> f32;
}

impl KdaDisplay for MatchPlayer {
    fn kda_string(&self) -> String {
        format!("{}/{}/{}", self.stats.kills, self.stats.deaths, self.stats.assists)
    }

    fn kda_ratio(&self) -> f32 {
        let deaths = self.stats.deaths.max(1) as f32;
        (self.stats.kills as f32 + self.stats.assists as f32) / deaths
    }
}
