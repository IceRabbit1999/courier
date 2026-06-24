use std::future::Future;

use tracing::error;

use crate::components::toast::ToastSender;

/// What a spawned task hands back: a successful [`TaskResult`] or a
/// [`snafu::Whatever`] error. Spawn closures attach context to any `plugin` /
/// `storage` error with [`snafu::ResultExt::whatever_context`] (then `?`), and
/// the bridge collapses every error into a single [`TaskResult::TaskFailed`].
pub type TaskOutcome = Result<TaskResult, snafu::Whatever>;

/// Results flowing from background tasks back to the UI thread.
///
/// Each variant represents one completed async operation; every failure, whatever
/// its source, arrives as [`TaskResult::TaskFailed`]. Add new variants here as
/// more background tasks are introduced.
#[derive(Debug)]
pub enum TaskResult {
    /// The friend list, loaded from the DB on startup or refreshed from Steam.
    FriendsLoaded(Vec<shared::Friend>),
    /// Static reference data (heroes/items) was synced; carries the row counts.
    StaticDataSynced { heroes: i64, items: i64 },
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

    /// Spawn an async task whose output is a [`TaskOutcome`].
    ///
    /// On `Ok` the [`TaskResult`] is forwarded as-is; on `Err` it is collapsed
    /// into [`TaskResult::TaskFailed`]. Either way the result is sent to the
    /// UI-side receiver and a repaint is requested so the UI picks it up promptly.
    pub fn spawn<F>(&self, future: F)
    where
        F: Future<Output = TaskOutcome> + Send + 'static,
    {
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        self.runtime.spawn(async move {
            let result = match future.await {
                Ok(result) => result,
                Err(e) => TaskResult::TaskFailed(e.to_string()),
            };
            if let Err(e) = tx.send(result) {
                error!("Failed to send task result to UI: {e}");
            }
            ctx.request_repaint();
        });
    }
}
