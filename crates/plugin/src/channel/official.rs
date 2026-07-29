//! The official Telegram bot, reached through the hosted Courier hub. Push and
//! the device-linking handshake are plain REST calls over the shared [`Client`],
//! so no Telegram SDK is pulled into the desktop app.

use reqwest::Method;
use serde_json::Value;
use shared::hub::{Ack, LinkNew, LinkStatus, Notification, NotifyRequest, SyncRequest};
use snafu::ensure;

use super::Channel;
use crate::{
    client::{Client, Endpoint},
    error::ChannelSnafu,
};

const CHANNEL_ID: &str = "telegram-official";

/// Trim a trailing slash so `format!("{base_url}/notify")` never doubles it.
/// Every entry point normalizes here, so the endpoints below can just append.
fn base(url: &str) -> String {
    url.trim_end_matches('/').to_owned()
}

/// Pushes notifications to the user's Telegram via the hub's `/notify`,
/// authenticating with the per-subscriber secret minted at link time.
pub struct OfficialHub {
    client: Client,
    base_url: String,
    secret: String,
}

impl OfficialHub {
    pub fn new(client: Client, base_url: String, secret: String) -> Self {
        Self {
            client,
            base_url: base(&base_url),
            secret,
        }
    }
}

#[async_trait::async_trait]
impl Channel for OfficialHub {
    fn id(&self) -> &'static str {
        CHANNEL_ID
    }

    async fn deliver(&self, notification: &Notification) -> crate::Result<()> {
        let ack = self
            .client
            .execute(NotifyEndpoint {
                base_url: self.base_url.clone(),
                secret: self.secret.clone(),
                body: NotifyRequest {
                    notification: notification.clone(),
                },
            })
            .await?;
        ensure!(
            ack.ok,
            ChannelSnafu {
                channel: CHANNEL_ID,
                message: "hub rejected the notification".to_owned(),
            }
        );
        Ok(())
    }
}

/// Begin device linking: ask the hub for a single-use token + Telegram deep link.
/// `access_code` is the official hub's early-access invite code; `None` (or an
/// open self-hosted hub) skips the gate.
pub async fn link_new(client: &Client, base_url: &str, access_code: Option<String>) -> crate::Result<LinkNew> {
    client
        .execute(LinkNewEndpoint {
            base_url: base(base_url),
            access_code,
        })
        .await
}

/// Poll whether the user has completed the deep-link `/start`; once `linked`, the
/// reply carries the subscriber credential to persist.
pub async fn link_status(client: &Client, base_url: &str, token: &str) -> crate::Result<LinkStatus> {
    client
        .execute(LinkStatusEndpoint {
            base_url: base(base_url),
            token: token.to_owned(),
        })
        .await
}

/// Upload the tracking snapshot (tracked accounts, locale, hero names, offline
/// flag) that the hub's own tracker runs on when offline mode is enabled.
pub async fn sync(client: &Client, base_url: &str, secret: &str, request: SyncRequest) -> crate::Result<()> {
    let _: Ack = client
        .execute(SyncEndpoint {
            base_url: base(base_url),
            secret: secret.to_owned(),
            body: request,
        })
        .await?;
    Ok(())
}

/// Revoke the subscriber, unbinding the Telegram chat from this install.
pub async fn unlink(client: &Client, base_url: &str, secret: &str) -> crate::Result<()> {
    let _: Ack = client
        .execute(UnlinkEndpoint {
            base_url: base(base_url),
            secret: secret.to_owned(),
        })
        .await?;
    Ok(())
}

struct NotifyEndpoint {
    base_url: String,
    secret: String,
    body: NotifyRequest,
}

impl Endpoint for NotifyEndpoint {
    type Response = Ack;

    fn method(&self) -> Method {
        Method::POST
    }

    fn url(&self) -> String {
        format!("{}/notify", self.base_url)
    }

    fn body(&self) -> Option<Value> {
        serde_json::to_value(&self.body).ok()
    }

    fn bearer(&self) -> Option<String> {
        Some(self.secret.clone())
    }
}

struct LinkNewEndpoint {
    base_url: String,
    access_code: Option<String>,
}

impl Endpoint for LinkNewEndpoint {
    type Response = LinkNew;

    fn method(&self) -> Method {
        Method::POST
    }

    fn url(&self) -> String {
        format!("{}/link/new", self.base_url)
    }

    fn bearer(&self) -> Option<String> {
        self.access_code.clone()
    }
}

struct LinkStatusEndpoint {
    base_url: String,
    token: String,
}

impl Endpoint for LinkStatusEndpoint {
    type Response = LinkStatus;

    fn url(&self) -> String {
        format!("{}/link/status", self.base_url)
    }

    fn query(&self) -> Vec<(&'static str, String)> {
        vec![("token", self.token.clone())]
    }
}

struct SyncEndpoint {
    base_url: String,
    secret: String,
    body: SyncRequest,
}

impl Endpoint for SyncEndpoint {
    type Response = Ack;

    fn method(&self) -> Method {
        Method::POST
    }

    fn url(&self) -> String {
        format!("{}/sync", self.base_url)
    }

    fn body(&self) -> Option<Value> {
        serde_json::to_value(&self.body).ok()
    }

    fn bearer(&self) -> Option<String> {
        Some(self.secret.clone())
    }
}

struct UnlinkEndpoint {
    base_url: String,
    secret: String,
}

impl Endpoint for UnlinkEndpoint {
    type Response = Ack;

    fn method(&self) -> Method {
        Method::POST
    }

    fn url(&self) -> String {
        format!("{}/unlink", self.base_url)
    }

    fn bearer(&self) -> Option<String> {
        Some(self.secret.clone())
    }
}
