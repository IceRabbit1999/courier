//! Background match tracking. A single long-lived task polls each *tracked*
//! follow for new matches and pushes them out through the configured notification
//! [`plugin::channel::Channel`]s. It lives off the UI thread entirely; results
//! surface back through the [`AsyncBridge`].

use std::time::Duration;

use plugin::channel::{Channel, OfficialHub};
use shared::hub::Notification;
use tokio::sync::mpsc;
use tracing::warn;

use crate::async_bridge::{AsyncBridge, TaskResult};

/// A small pause between per-follow OpenDota calls to stay friendly to rate limits.
const PER_FOLLOW_DELAY: Duration = Duration::from_millis(400);

/// Fallback interval if the configured value is nonsensical (0).
const MIN_INTERVAL: Duration = Duration::from_secs(60);

enum Control {
    Wake,
}

/// Handle to the running tracker task. Dropping every clone lets the loop exit on
/// its next control await.
#[derive(Clone)]
pub struct Tracker {
    control_tx: mpsc::UnboundedSender<Control>,
}

impl Tracker {
    pub fn spawn(runtime: &tokio::runtime::Handle, storage: storage::Storage, bridge: AsyncBridge) -> Self {
        let (control_tx, control_rx) = mpsc::unbounded_channel();
        runtime.spawn(run_loop(storage, bridge, control_rx));
        Self { control_tx }
    }

    /// Ask the tracker to run a cycle now (e.g. right after a follow is tracked)
    /// instead of waiting out the interval.
    pub fn wake(&self) {
        let _ = self.control_tx.send(Control::Wake);
    }
}

async fn run_loop(storage: storage::Storage, bridge: AsyncBridge, mut control_rx: mpsc::UnboundedReceiver<Control>) {
    loop {
        run_cycle(&storage, &bridge).await;

        let interval = poll_interval();
        tokio::select! {
            _ = tokio::time::sleep(interval) => {}
            control = control_rx.recv() => {
                match control {
                    Some(Control::Wake) => {}
                    None => break,
                }
            }
        }
    }
}

fn poll_interval() -> Duration {
    let mins = configs::read().tracking.poll_interval_mins;
    if mins == 0 { MIN_INTERVAL } else { Duration::from_secs(u64::from(mins) * 60) }
}

async fn run_cycle(storage: &storage::Storage, bridge: &AsyncBridge) {
    let (background, fetch_limit, opendota_key, proxy, telegram, secret) = {
        let config = configs::read();
        (
            config.tracking.background_tracking,
            config.tracking.fetch_limit,
            config.secrets.opendota_api_key.clone(),
            config.network.proxy.clone(),
            config.notification.telegram.clone(),
            config.secrets.telegram_subscriber_secret.clone(),
        )
    };

    if !background {
        return;
    }

    let Ok(tracked) = storage.list_tracked_follows().await.inspect_err(|e| warn!("tracker: failed to list tracked follows: {e}")) else {
        return;
    };
    if tracked.is_empty() {
        return;
    }

    let Ok(client) = plugin::Client::new(proxy.as_deref()).inspect_err(|e| warn!("tracker: failed to build http client: {e}")) else {
        return;
    };

    let channels = build_channels(&client, &telegram, secret);
    let hero_names = storage.hero_names(&i18n::current_locale()).await.unwrap_or_default();
    let item_names = storage.item_names(&i18n::current_locale()).await.unwrap_or_default();

    for follow in tracked {
        if let Err(e) = process_follow(storage, &client, opendota_key.clone(), fetch_limit, &channels, &hero_names, &item_names, bridge, &follow).await {
            warn!("tracker: {} failed: {e}", follow.steam_id);
        }
        tokio::time::sleep(PER_FOLLOW_DELAY).await;
    }
}

/// Assemble the enabled push channels from config. Currently just the Telegram
/// hub; more channels append here. With offline mode on, the hub's own tracker
/// owns Telegram delivery, so the channel is skipped here — exactly one side
/// pushes.
fn build_channels(client: &plugin::Client, telegram: &configs::TelegramConfig, secret: Option<String>) -> Vec<Box<dyn Channel>> {
    let mut channels: Vec<Box<dyn Channel>> = Vec::new();
    if telegram.enabled
        && telegram.notify_new_match
        && !telegram.offline_mode
        && let Some(secret) = secret
    {
        channels.push(Box::new(OfficialHub::new(client.clone(), telegram.hub_base_url.clone(), secret)));
    }
    channels
}

#[allow(clippy::too_many_arguments, reason = "one call site; splitting into a context struct buys nothing")]
async fn process_follow(
    storage: &storage::Storage,
    client: &plugin::Client,
    opendota_key: Option<String>,
    fetch_limit: usize,
    channels: &[Box<dyn Channel>],
    hero_names: &std::collections::HashMap<i32, String>,
    item_names: &std::collections::HashMap<i32, String>,
    bridge: &AsyncBridge,
    follow: &storage::TrackedFollow,
) -> Result<(), snafu::Whatever> {
    use snafu::ResultExt;

    let matches = client
        .opendota(opendota_key.clone())
        .recent_matches(&follow.steam_id, fetch_limit)
        .await
        .whatever_context("failed to fetch recent matches")?;

    let Some(newest_id) = matches.iter().map(|m| m.match_id).max() else {
        return Ok(());
    };

    // First observation: adopt the newest match as the watermark without pushing,
    // so tracking a player never blasts their pre-existing last game.
    let Some(watermark) = follow.last_tracked_match_id else {
        storage
            .set_tracked_watermark(&follow.steam_id, newest_id)
            .await
            .whatever_context("failed to prime watermark")?;
        return Ok(());
    };

    let mut fresh = matches.iter().filter(|m| m.match_id > watermark).collect::<Vec<_>>();
    if fresh.is_empty() {
        return Ok(());
    }
    fresh.sort_by_key(|m| m.match_id);

    storage
        .replace_match_summaries(&follow.steam_id, &matches)
        .await
        .whatever_context("failed to store match summaries")?;

    let locale = i18n::current_locale();
    for summary in &fresh {
        // The rich card needs the full match; a detail fetch that fails is skipped
        // so it can't block the newer matches or the watermark advance below.
        let Ok(detail) = client
            .opendota(opendota_key.clone())
            .match_detail(summary.match_id)
            .await
            .inspect_err(|e| warn!("tracker: match {} detail failed: {e}", summary.match_id))
        else {
            continue;
        };
        let notification = Notification::match_card(&locale, &follow.persona_name, &detail, summary.player_slot, hero_names, item_names);
        for channel in channels {
            if let Err(e) = channel.deliver(&notification).await {
                warn!("tracker: channel {} failed: {e}", channel.id());
            }
        }
    }

    storage
        .set_tracked_watermark(&follow.steam_id, newest_id)
        .await
        .whatever_context("failed to advance watermark")?;

    bridge.send(TaskResult::NewMatchesTracked {
        steam_id: follow.steam_id.clone(),
        count: fresh.len(),
    });
    Ok(())
}
