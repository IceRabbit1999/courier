#![feature(error_generic_member_access)]

mod config;
mod error;
mod http;
mod rate_limit;
mod telegram;
mod tracker;

use std::{path::Path, sync::Arc};

use snafu::{OptionExt, ResultExt};
use storage::HubStore;
use teloxide::prelude::*;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

pub use crate::error::Result;
use crate::{
    config::HubConfig,
    error::{MissingTokenSnafu, PluginSnafu, StorageSnafu, TelegramSnafu},
    rate_limit::RateLimit,
};

/// Everything the request handlers, the dispatcher and the tracker share. The
/// config is read once at startup and never reloaded, so the outbound HTTP
/// client is built once alongside it rather than per call site.
#[derive(Clone)]
pub struct AppState {
    pub store: HubStore,
    pub bot: Bot,
    pub bot_username: String,
    pub limits: Arc<RateLimit>,
    pub config: Arc<HubConfig>,
    pub client: plugin::Client,
}

fn init_tracing(log_dir: &Path) -> Result<WorkerGuard> {
    std::fs::create_dir_all(log_dir).with_whatever_context(|_| format!("Failed to create the log directory '{}'", log_dir.display()))?;
    let (file_writer, guard) = tracing_appender::non_blocking(tracing_appender::rolling::daily(log_dir, "courier-hub.log"));

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().pretty().with_thread_ids(true).with_target(true))
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_thread_ids(true)
                .with_target(true)
                .with_writer(file_writer),
        )
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,axum::rejection=trace")))
        .init();

    Ok(guard)
}

#[tokio::main]
#[snafu::report]
async fn main() -> Result<()> {
    let config = HubConfig::load()?;
    let token = config.telegram.token.clone().context(MissingTokenSnafu)?;

    let _guard = init_tracing(&config.server.log_path)?;
    info!("Courier hub v{} starting, logs at {}", env!("CARGO_PKG_VERSION"), config.server.log_path.display());

    info!("Opening the hub database at {}", config.server.data_path.display());
    let store = HubStore::open(&config.server.data_path).await.context(StorageSnafu)?;
    let client = plugin::Client::new(config.network.proxy.as_deref()).context(PluginSnafu {
        purpose: "Failed to build the HTTP client",
    })?;
    let bot = Bot::new(token);
    let me = bot.get_me().await.context(TelegramSnafu)?;
    let bot_username = me.user.username.clone().unwrap_or_default();
    info!("Authenticated with Telegram as @{bot_username}");

    let bind = config.server.bind.clone();
    let state = AppState {
        store,
        bot: bot.clone(),
        bot_username,
        limits: Arc::new(RateLimit::new()),
        config: Arc::new(config),
        client,
    };

    let shutdown = CancellationToken::new();
    tokio::spawn({
        let shutdown = shutdown.clone();
        async move {
            match tokio::signal::ctrl_c().await {
                Ok(()) => info!("Shutdown signal received, draining"),
                Err(e) => error!("failed to listen for shutdown signal: {e}"),
            }
            shutdown.cancel();
        }
    });

    let listener = TcpListener::bind(&bind).await.with_whatever_context(|_| format!("Failed to bind '{bind}'"))?;
    info!("HTTP API listening on {bind}");
    let server = tokio::spawn({
        let state = state.clone();
        let shutdown = shutdown.clone();
        async move {
            if let Err(e) = axum::serve(listener, http::router(state)).with_graceful_shutdown(shutdown.cancelled_owned()).await {
                error!("http server exited: {e}");
            };
        }
    });

    let offline_tracker = tokio::spawn(tracker::run(state.clone(), shutdown.clone()));

    telegram::run(bot, state, shutdown.clone()).await;
    // The dispatcher returning means Telegram is done; make sure the others hear
    // about it even when the exit didn't start from a signal.
    shutdown.cancel();
    let _ = tokio::join!(server, offline_tracker);
    info!("Hub stopped");
    Ok(())
}
