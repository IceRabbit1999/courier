//! Outbound notification channels. A [`Channel`] delivers an already-localized
//! [`shared::hub::Notification`]; the tracker fans one event out to every
//! enabled channel. Telegram (via the official hub) is the only implementation
//! today; Discord / AI bots slot in as further `Channel`s.

mod official;

pub use official::{OfficialHub, link_new, link_status, sync, unlink};
use shared::hub::Notification;

#[async_trait::async_trait]
pub trait Channel: Send + Sync {
    /// Stable identifier used in logs and error messages.
    fn id(&self) -> &'static str;

    /// Deliver one notification. Returns an error if the channel could not accept
    /// it; the caller logs and moves on to the next channel.
    async fn deliver(&self, notification: &Notification) -> crate::Result<()>;
}
