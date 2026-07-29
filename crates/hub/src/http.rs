//! The hub's HTTP API: device linking (`/link/new`, `/link/status`), push
//! (`/notify`, `/unlink`), and the tracking snapshot upload (`/sync`). Desktop
//! clients speak to these with the shared `shared::hub` DTOs.

use axum::{
    Json, Router,
    extract::{Query, State},
    http::{HeaderMap, header::AUTHORIZATION},
    routing::{get, post},
};
use serde::Deserialize;
use shared::hub::{Ack, LinkNew, LinkStatus, NotifyRequest, SyncRequest};
use snafu::{OptionExt, ResultExt, ensure};
use tracing::{error, info, instrument};

use crate::{
    AppState,
    error::{AuthSnafu, StorageSnafu, TelegramSnafu},
    telegram, tracker,
};

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/link/new", post(link_new))
        .route("/link/status", get(link_status))
        .route("/notify", post(notify))
        .route("/sync", post(sync))
        .route("/unlink", post(unlink))
        .with_state(state)
}

#[instrument(skip_all)]
async fn link_new(State(state): State<AppState>, headers: HeaderMap) -> crate::Result<Json<LinkNew>> {
    // Invite-only gate: an unset code leaves the hub open (self-hosters), a set
    // one demands a matching bearer before any token is minted.
    if let Some(code) = state.config.server.access_code.as_deref() {
        ensure!(bearer(&headers).as_deref() == Some(code), AuthSnafu);
    }
    let pending = state.store.create_pending().await.context(StorageSnafu)?;
    info!("Minted a link token, waiting for a chat to bind");
    let deep_link = format!("https://t.me/{}?start={}", state.bot_username, pending.token);
    Ok(Json(LinkNew {
        token: pending.token,
        secret: pending.secret,
        deep_link,
    }))
}

#[derive(Deserialize)]
struct TokenQuery {
    token: String,
}

#[instrument(skip_all)]
async fn link_status(State(state): State<AppState>, Query(q): Query<TokenQuery>) -> crate::Result<Json<LinkStatus>> {
    let subscriber_id = state.store.status(&q.token).await.context(StorageSnafu)?;
    Ok(Json(LinkStatus {
        linked: subscriber_id.is_some(),
        subscriber_id,
    }))
}

#[instrument(skip_all)]
async fn notify(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<NotifyRequest>) -> crate::Result<Json<Ack>> {
    let secret = bearer(&headers).context(AuthSnafu)?;
    let chat_id = state.store.chat_for_secret(&secret).await.context(StorageSnafu)?;
    telegram::push(&state.bot, &state.limits, chat_id, &req.notification).await.context(TelegramSnafu)?;
    info!("Relayed a client-rendered notification to chat {chat_id}");
    Ok(Json(Ack { ok: true }))
}

#[instrument(skip_all, fields(accounts = req.accounts.len(), locale = %req.locale, offline_mode = req.offline_mode))]
async fn sync(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<SyncRequest>) -> crate::Result<Json<Ack>> {
    let secret = bearer(&headers).context(AuthSnafu)?;

    state.store.sync(&secret, &req).await.context(StorageSnafu)?;
    info!("Stored the tracking snapshot");

    // Don't make the client wait on OpenDota: acknowledge the snapshot, then
    // push the latest match right away rather than at the next poll tick.
    if let Some(subscriber) = state.store.subscriber_by_secret(&secret).await.context(StorageSnafu)? {
        tokio::spawn(async move {
            let _ = tracker::cycle_subscriber(&state, &subscriber, tracker::Mode::Latest)
                .await
                .inspect(|pushed| info!("Post-sync pass for {} pushed {pushed} match(es)", subscriber.subscriber_id))
                .inspect_err(|e| error!("sync: immediate pass failed for {}: {e}", subscriber.subscriber_id));
        });
    }

    Ok(Json(Ack { ok: true }))
}

#[instrument(skip_all)]
async fn unlink(State(state): State<AppState>, headers: HeaderMap) -> crate::Result<Json<Ack>> {
    let secret = bearer(&headers).context(AuthSnafu)?;
    state.store.delete_subscriber(&secret).await.context(StorageSnafu)?;
    info!("Subscriber unlinked and forgotten");
    Ok(Json(Ack { ok: true }))
}

fn bearer(headers: &HeaderMap) -> Option<String> {
    headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::to_owned)
}
