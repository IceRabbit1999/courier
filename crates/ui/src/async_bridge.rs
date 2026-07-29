use std::future::Future;

use tracing::error;

use crate::components::toast::ToastSender;

/// Results flowing from background tasks back to the UI thread.
///
/// Each variant represents one completed async operation; every failure, whatever
/// its source, arrives as [`TaskResult::TaskFailed`]. Add new variants here as
/// more background tasks are introduced.
#[derive(Debug)]
pub enum TaskResult {
    /// The friend list, loaded from the DB on startup or refreshed from Steam.
    FriendsLoaded(Vec<shared::Friend>),
    /// The follow list, loaded from the DB or updated by an add/remove/refresh.
    FollowsLoaded(Vec<shared::Follow>),
    /// Static reference data (heroes/items) was synced; carries the row counts.
    StaticDataSynced { heroes: i64, items: i64 },
    /// A friend's match summaries, loaded from the DB or fetched from OpenDota.
    MatchSummariesLoaded { steam_id: String, matches: Vec<shared::MatchSummary> },
    /// Full detail for one match, loaded from the DB or fetched from OpenDota.
    MatchDetailLoaded(Box<shared::MatchDetail>),
    /// Localized hero/item name maps (plus CDN icon slugs) for rendering matches.
    MatchNamesLoaded {
        heroes: std::collections::HashMap<i32, String>,
        items: std::collections::HashMap<i32, String>,
        hero_slugs: std::collections::HashMap<i32, String>,
        item_slugs: std::collections::HashMap<i32, String>,
    },
    /// Background tracking found new matches for a tracked follow.
    NewMatchesTracked { steam_id: String, count: usize },
    /// A Telegram link handshake began: open `deep_link` and poll `token`.
    TelegramLinkStarted { token: String, secret: String, deep_link: String },
    /// The Telegram link completed; carries the subscriber credential to persist.
    TelegramLinked { subscriber_id: String, secret: String },
    /// A Telegram test push was delivered.
    TelegramTestSent,
    /// The tracking snapshot was uploaded to the hub; carries how many
    /// tracked accounts it covered.
    TelegramSynced { accounts: usize },
    /// The Telegram subscriber was unlinked.
    TelegramUnlinked,
    /// Any background task failed; carries the user-facing error message.
    TaskFailed(String),
}

/// A cloneable handle for spawning async tasks from UI code.
///
/// Wraps a tokio runtime handle and an mpsc sender so that any callsite
/// can fire off background work without touching async directly.
/// The spawned future's result is sent back through the channel,
/// and `ctx.request_repaint()` is called to wake the UI.
#[derive(Clone)]
pub struct AsyncBridge {
    runtime: tokio::runtime::Handle,
    tx: tokio::sync::mpsc::UnboundedSender<TaskResult>,
    ctx: egui::Context,
    toasts: ToastSender,
}

impl AsyncBridge {
    pub fn new(runtime: tokio::runtime::Handle, tx: tokio::sync::mpsc::UnboundedSender<TaskResult>, ctx: egui::Context, toasts: ToastSender) -> Self {
        Self { runtime, tx, ctx, toasts }
    }

    /// Returns a [`ToastSender`] for reporting notifications from async tasks.
    pub fn toasts(&self) -> &ToastSender {
        &self.toasts
    }

    /// Push a [`TaskResult`] straight to the UI receiver and request a repaint.
    /// Used by long-lived tasks (e.g. the tracker) that outlive a single
    /// [`Self::spawn`] and report results as they occur rather than on completion.
    pub fn send(&self, result: TaskResult) {
        if let Err(e) = self.tx.send(result) {
            error!("Failed to send task result to UI: {e}");
        }
        self.ctx.request_repaint();
    }

    /// Spawn an async task.
    ///
    /// On `Ok` the [`TaskResult`] is forwarded as-is; on `Err` it is collapsed
    /// into [`TaskResult::TaskFailed`]. Either way the result is sent to the
    /// UI-side receiver and a repaint is requested so the UI picks it up promptly.
    pub fn spawn<F>(&self, future: F)
    where
        F: Future<Output = Result<TaskResult, snafu::Whatever>> + Send + 'static,
    {
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        self.runtime.spawn(async move {
            let result = future.await.unwrap_or_else(|e| TaskResult::TaskFailed(e.to_string()));
            if let Err(e) = tx.send(result) {
                error!("Failed to send task result to UI: {e}");
            }
            ctx.request_repaint();
        });
    }
}
