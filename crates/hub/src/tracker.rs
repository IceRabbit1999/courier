use std::{collections::HashMap, time::Duration};

use shared::hub::Notification;
use snafu::ResultExt;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, instrument, warn};

use crate::{
    AppState,
    error::{PluginSnafu, StorageSnafu, TelegramSnafu},
    telegram,
};

const MIN_INTERVAL: Duration = Duration::from_secs(60);

fn poll_interval(state: &AppState) -> Duration {
    let minutes = state.config.tracking.poll_interval_mins;
    if minutes == 0 { MIN_INTERVAL } else { Duration::from_secs(u64::from(minutes) * 60) }
}

#[instrument(skip_all)]
pub async fn run(state: AppState, shutdown: CancellationToken) {
    let interval = poll_interval(&state);
    info!("Offline tracker started, polling every {interval:?}");
    loop {
        tokio::select! {
            biased;
            () = shutdown.cancelled() => break,
            result = run_cycle(&state, &shutdown) => {
                let _ = result.inspect_err(|e| error!("cycle failed: {e}"));
            }
        }
        tokio::select! {
            () = shutdown.cancelled() => break,
            () = tokio::time::sleep(interval) => {}
        }
    }
    info!("Offline tracker stopped");
}

/// What a pass should push. The background loop only ever announces matches the
/// subscriber hasn't seen; a pass the user asked for ([`Mode::Latest`]) always
/// produces a message, since a button that silently does nothing reads as broken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Everything newer than the stored watermark.
    New,
    /// The newest match per account, watermark or not.
    Latest,
}

#[instrument(skip_all)]
async fn run_cycle(state: &AppState, shutdown: &CancellationToken) -> crate::Result<()> {
    let subscribers = state.store.offline_subscribers().await.context(StorageSnafu)?;
    info!("Cycle over {} offline subscriber(s)", subscribers.len());

    for subscriber in subscribers {
        if shutdown.is_cancelled() {
            return Ok(());
        }
        if let Err(e) = cycle_subscriber(state, &subscriber, Mode::New).await {
            warn!("tracker: subscriber {} failed: {e}", subscriber.subscriber_id);
        }
    }
    Ok(())
}

/// Run one pass over a single subscriber's accounts, returning how many matches
/// were pushed. Shared by the interval loop, the `/sync` handler, and the bot's
/// fetch button so the three can never drift apart.
#[instrument(skip_all, fields(subscriber = %subscriber.subscriber_id, ?mode))]
pub async fn cycle_subscriber(state: &AppState, subscriber: &storage::hub::Subscriber, mode: Mode) -> crate::Result<usize> {
    let hero_names = state.store.hero_names(&subscriber.locale).await.unwrap_or_default();
    let item_names = state.store.item_names(&subscriber.locale).await.unwrap_or_default();
    let accounts = state.store.tracked_accounts(&subscriber.subscriber_id).await.context(StorageSnafu)?;

    let mut pushed = 0;
    for account in accounts {
        match process_account(state, subscriber, &account, &hero_names, &item_names, mode).await {
            Ok(count) => pushed += count,
            Err(e) => warn!("tracker: {} failed: {e}", account.steam_id),
        }
    }
    if pushed > 0 {
        info!("Pushed {pushed} match(es)");
    }
    Ok(pushed)
}

#[instrument(skip_all, fields(steam_id = %account.steam_id, persona = %account.persona_name))]
async fn process_account(
    state: &AppState,
    subscriber: &storage::hub::Subscriber,
    account: &storage::hub::TrackedAccount,
    hero_names: &HashMap<i32, String>,
    item_names: &HashMap<i32, String>,
    mode: Mode,
) -> crate::Result<usize> {
    state.limits.opendota().await;
    let matches = state
        .client
        .opendota(state.config.secrets.opendota_api_key.clone())
        .recent_matches(&account.steam_id, state.config.tracking.fetch_limit)
        .await
        .context(PluginSnafu {
            purpose: "failed to fetch recent matches",
        })?;

    let Some(newest_id) = matches.iter().map(|m| m.match_id).max() else {
        return Ok(0);
    };

    let fresh = match mode {
        Mode::Latest => matches.iter().filter(|m| m.match_id == newest_id).collect::<Vec<_>>(),
        Mode::New => {
            // No watermark yet means this account was only just synced: adopt the
            // current newest as the baseline instead of announcing history.
            let Some(watermark) = account.last_match_id else {
                info!("First sight of this account, adopting match {newest_id} as the baseline");
                state
                    .store
                    .set_account_watermark(&subscriber.subscriber_id, &account.steam_id, newest_id)
                    .await
                    .context(StorageSnafu)?;
                return Ok(0);
            };
            let mut fresh = matches.iter().filter(|m| m.match_id > watermark).collect::<Vec<_>>();
            fresh.sort_by_key(|m| m.match_id);
            fresh
        }
    };

    if fresh.is_empty() {
        return Ok(0);
    }

    info!("{} new match(es) to announce, newest is {newest_id}", fresh.len());
    // Each card now needs the full match, not just the summary, so the tracked
    // player's final items and end-game stats can be shown. Rendered in the
    // subscriber's synced locale; the app's own tracker sends the same card. A
    // detail fetch that fails (an unavailable match) is skipped rather than
    // blocking the newer matches behind it — the watermark still advances past it.
    let mut pushed = 0;
    for summary in fresh {
        state.limits.opendota().await;
        let Ok(detail) = state
            .client
            .opendota(state.config.secrets.opendota_api_key.clone())
            .match_detail(summary.match_id)
            .await
            .inspect_err(|e| warn!("tracker: match {} detail failed: {e}", summary.match_id))
        else {
            continue;
        };
        let notification = Notification::match_card(&subscriber.locale, &account.persona_name, &detail, summary.player_slot, hero_names, item_names);
        telegram::push(&state.bot, &state.limits, subscriber.chat_id, &notification).await.context(TelegramSnafu)?;
        pushed += 1;
    }

    state
        .store
        .set_account_watermark(&subscriber.subscriber_id, &account.steam_id, newest_id)
        .await
        .context(StorageSnafu)?;
    Ok(pushed)
}
