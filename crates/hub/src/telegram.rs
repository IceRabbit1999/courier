use std::{fmt, str::FromStr};

use shared::hub::Notification;
use teloxide::{
    prelude::*,
    types::{BotCommand, ChatId, InlineKeyboardButton, InlineKeyboardMarkup, ParseMode},
    utils::html,
};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, instrument, warn};

use crate::{AppState, rate_limit::RateLimit, tracker};

const CONNECT_HINT: &str = "Open Courier → Settings → Telegram → Connect to link this chat.";
const RETRY_HINT: &str = "Something went wrong. Please try again.";

/// Inline-keyboard actions, encoded into `callback_data`. Telegram caps that
/// field at 64 bytes, so a variant may carry ids but never free text; anything
/// bigger has to be looked up from the subscriber instead of round-tripped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Callback {
    Fetch,
}

impl fmt::Display for Callback {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fetch => f.write_str("fetch"),
        }
    }
}

impl FromStr for Callback {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Already split on ':' so that `fetch:<id>`-style arguments slot in
        // later without touching any call site.
        match s.split(':').next() {
            Some("fetch") => Ok(Self::Fetch),
            _ => Err(()),
        }
    }
}

/// The actions offered under a message, in one place so every reply that wants
/// buttons offers the same set.
fn actions_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new([[InlineKeyboardButton::callback("🔄 Latest match", Callback::Fetch.to_string())]])
}

/// Run the update dispatcher: messages (the deep-link `/start <token>` that
/// binds a chat, plus commands) and inline-keyboard callbacks.
///
/// Shutdown is driven by `shutdown` rather than teloxide's own Ctrl-C handler,
/// so the dispatcher stops on the same signal as the HTTP server and tracker.
#[instrument(skip_all)]
pub async fn run(bot: Bot, state: AppState, shutdown: CancellationToken) {
    // Telegram recommends specific commands over one command taking arguments,
    // so each future feature earns its own entry in the client's menu button.
    let commands = [
        BotCommand::new("fetch", "Show the latest match for everyone you track"),
        BotCommand::new("help", "What this bot can do"),
    ];
    if let Err(e) = bot.set_my_commands(commands).await {
        warn!("failed to publish the command list: {e}");
    }
    info!("Telegram dispatcher started");

    let handler = dptree::entry()
        .branch(Update::filter_message().endpoint(on_message))
        .branch(Update::filter_callback_query().endpoint(on_callback));
    let mut dispatcher = Dispatcher::builder(bot, handler).dependencies(dptree::deps![state]).build();

    let dispatcher_shutdown = dispatcher.shutdown_token();
    tokio::spawn(async move {
        shutdown.cancelled().await;
        let Ok(finished) = dispatcher_shutdown.shutdown().inspect_err(|e| warn!("dispatcher was not running: {e}")) else {
            return;
        };
        finished.await;
    });

    dispatcher.dispatch().await;
    info!("Telegram dispatcher stopped");
}

#[instrument(skip_all, fields(chat_id = msg.chat.id.0))]
async fn on_message(bot: Bot, msg: Message, state: AppState) -> ResponseResult<()> {
    let Some(text) = msg.text().map(str::trim) else {
        return Ok(());
    };

    if let Some(token) = text.strip_prefix("/start") {
        return on_start(&bot, &msg, &state, token.trim()).await;
    }
    if text.starts_with("/fetch") {
        return on_fetch(&bot, msg.chat.id, &state).await;
    }
    if text.starts_with("/help") {
        bot.send_message(msg.chat.id, format!("{CONNECT_HINT}\n\nOnce linked, /fetch shows the latest match for everyone you track."))
            .reply_markup(actions_keyboard())
            .await?;
        return Ok(());
    }

    bot.send_message(msg.chat.id, CONNECT_HINT).await?;
    Ok(())
}

#[instrument(skip_all, fields(chat_id = msg.chat.id.0))]
async fn on_start(bot: &Bot, msg: &Message, state: &AppState, token: &str) -> ResponseResult<()> {
    if token.is_empty() {
        bot.send_message(msg.chat.id, CONNECT_HINT).await?;
        return Ok(());
    }

    match state.store.bind(token, msg.chat.id.0).await {
        Ok(Some(subscriber_id)) => {
            info!("Chat bound to subscriber {subscriber_id}");
            bot.send_message(msg.chat.id, "✅ Connected to Courier. You'll get match updates here.")
                .reply_markup(actions_keyboard())
                .await?;
        }
        Ok(None) => {
            info!("Rejected a link token that is unknown or already spent");
            bot.send_message(msg.chat.id, "This link is invalid or already used. Start again from Courier.").await?;
        }
        Err(e) => {
            error!("Failed to bind link token: {e}");
            bot.send_message(msg.chat.id, RETRY_HINT).await?;
        }
    }
    Ok(())
}

/// Callbacks are answered first thing: an unanswered query leaves the client
/// spinning on the button until it times out.
#[instrument(skip_all, fields(data = q.data.as_deref().unwrap_or_default()))]
async fn on_callback(bot: Bot, q: CallbackQuery, state: AppState) -> ResponseResult<()> {
    bot.answer_callback_query(q.id.clone()).await?;

    let Some(Callback::Fetch) = q.data.as_deref().and_then(|data| Callback::from_str(data).ok()) else {
        return Ok(());
    };
    let Some(chat_id) = q.message.as_ref().map(|msg| msg.chat().id) else {
        warn!("callback without a chat, ignoring");
        return Ok(());
    };
    on_fetch(&bot, chat_id, &state).await
}

/// Run a pass the user asked for. Unlike the background loop this ignores the
/// watermark ([`tracker::Mode::Latest`]), so the button always says something.
#[instrument(skip_all, fields(chat_id = chat_id.0))]
async fn on_fetch(bot: &Bot, chat_id: ChatId, state: &AppState) -> ResponseResult<()> {
    let subscriber = match state.store.subscriber_by_chat(chat_id.0).await {
        Ok(Some(subscriber)) => subscriber,
        Ok(None) => {
            info!("Fetch from an unlinked chat, sending the connect hint");
            bot.send_message(chat_id, CONNECT_HINT).await?;
            return Ok(());
        }
        Err(e) => {
            error!("failed to resolve the subscriber for chat {chat_id}: {e}");
            bot.send_message(chat_id, RETRY_HINT).await?;
            return Ok(());
        }
    };

    match tracker::cycle_subscriber(state, &subscriber, tracker::Mode::Latest).await {
        Ok(0) => {
            info!("Fetch for {} had nothing to show", subscriber.subscriber_id);
            bot.send_message(chat_id, "Nothing to show yet — follow players in Courier, then press Sync.")
                .reply_markup(actions_keyboard())
                .await?;
        }
        Ok(count) => info!("fetch for {} pushed {count} match(es)", subscriber.subscriber_id),
        Err(e) => {
            error!("fetch failed for {}: {e}", subscriber.subscriber_id);
            bot.send_message(chat_id, "Couldn't reach OpenDota just now. Please try again.").await?;
        }
    }
    Ok(())
}

/// Push a notification to `chat_id`, waiting on the Telegram quota first. The
/// payload is plain text, so both fields are HTML-escaped before the only markup
/// we add — a bold headline — goes on; that keeps a persona name containing `<`
/// from breaking the message.
#[instrument(skip(bot, limits, notification), fields(title = %notification.title))]
pub async fn push(bot: &Bot, limits: &RateLimit, chat_id: i64, notification: &Notification) -> ResponseResult<()> {
    limits.telegram().await;

    let text = format!("<b>{}</b>\n{}", html::escape(&notification.title), html::escape(&notification.body));
    bot.send_message(ChatId(chat_id), text).parse_mode(ParseMode::Html).await?;
    Ok(())
}
