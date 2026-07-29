use std::num::NonZeroU32;

use governor::{DefaultDirectRateLimiter, Quota, RateLimiter};

/// OpenDota's free tier allows 60 calls per minute.
const OPENDOTA_PER_MINUTE: u32 = 60;

/// Telegram tolerates roughly 30 outbound messages per second across all chats;
/// stay under it.
const TELEGRAM_PER_SECOND: u32 = 25;

pub struct RateLimit {
    opendota: DefaultDirectRateLimiter,
    telegram: DefaultDirectRateLimiter,
}

impl RateLimit {
    pub fn new() -> Self {
        #[allow(clippy::unwrap_used, reason = "Consts defined above will never be zero")]
        Self {
            opendota: RateLimiter::direct(Quota::per_minute(NonZeroU32::new(OPENDOTA_PER_MINUTE).unwrap())),
            telegram: RateLimiter::direct(Quota::per_second(NonZeroU32::new(TELEGRAM_PER_SECOND).unwrap())),
        }
    }

    /// Wait until another OpenDota request fits in the quota.
    pub async fn opendota(&self) {
        self.opendota.until_ready().await;
    }

    /// Wait until another Telegram message fits in the quota.
    pub async fn telegram(&self) {
        self.telegram.until_ready().await;
    }
}
