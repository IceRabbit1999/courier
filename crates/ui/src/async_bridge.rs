use std::future::Future;

use tracing::error;

use crate::components::toast::ToastSender;

/// Results flowing from background tasks back to the UI thread.
///
/// Each variant represents one completed (or failed) async operation.
/// Add new variants here as more background tasks are introduced.
pub enum TaskResult {
    // Placeholder: real variants will be added when actual async tasks are implemented.
    // e.g.:
    // MatchesLoaded(Vec<shared::Match>),
    // MatchesFailed(String),
    // PlayerLoaded(shared::PlayerProfile),
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

    /// Spawn an async task whose output is a [`TaskResult`].
    ///
    /// The result is automatically sent to the UI-side receiver,
    /// and a repaint is requested so the UI picks it up promptly.
    pub fn spawn<F>(&self, future: F)
    where
        F: Future<Output = TaskResult> + Send + 'static,
    {
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        self.runtime.spawn(async move {
            let result = future.await;
            if let Err(e) = tx.send(result) {
                error!("Failed to send task result to UI: {e}");
            }
            ctx.request_repaint();
        });
    }
}
