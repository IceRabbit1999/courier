use axum::{http::StatusCode, response::IntoResponse};
use snafu::{Location, Snafu};
use tracing::error;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Telegram bot token is not configured (set `token` under [telegram] in config/config.toml)"))]
    #[snafu(context(name(MissingTokenSnafu)))]
    MissingToken {
        #[snafu(implicit, provide)]
        location: Location,
    },
    #[snafu(display("Failed to load the hub configuration"))]
    #[snafu(context(name(ConfigSnafu)))]
    Config {
        source: config::ConfigError,
        #[snafu(implicit, provide)]
        location: Location,
    },
    #[snafu(display("Failed to reach Telegram"))]
    #[snafu(context(name(TelegramSnafu)))]
    Telegram {
        source: teloxide::RequestError,
        #[snafu(implicit, provide)]
        location: Location,
    },
    #[snafu(display("Failed to open the hub storage"))]
    #[snafu(context(name(StorageSnafu)))]
    Storage {
        source: storage::Error,
        #[snafu(implicit, provide)]
        location: Location,
    },
    #[snafu(display("The request was not authorized (missing bearer credential or wrong invite code)"))]
    #[snafu(context(name(AuthSnafu)))]
    Auth {
        #[snafu(implicit, provide)]
        location: Location,
    },
    #[snafu(display("{purpose}"))]
    #[snafu(context(name(PluginSnafu)))]
    Plugin {
        source: plugin::Error,
        purpose: String,
        #[snafu(implicit, provide)]
        location: Location,
    },
    /// Catch-all for one-off failures not worth their own variant (binding the
    /// listener, creating the log dir). Reach for `.whatever_context("…")`
    /// rather than adding a variant the rest of the code never matches on.
    #[snafu(whatever, display("{message}"))]
    Whatever {
        message: String,
        #[snafu(source(from(Box<dyn std::error::Error + Send + Sync>, Some)))]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        error!("{self:?}");
        match self {
            Self::Auth { .. } => (StatusCode::UNAUTHORIZED, "Unauthorized"),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "Internal Error"),
        }
        .into_response()
    }
}
