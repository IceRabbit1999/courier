#![feature(error_generic_member_access)]

use std::time::Duration;

use snafu::{ResultExt, Snafu};
use tracing::info;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use ui::App;

/// The app's startup errors are all one-shot and only ever reported (via
/// `#[snafu::report]`), never matched on, so a single catch-all carries them
/// rather than a variant per failure point.
#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(whatever, display("{message}"))]
    Whatever {
        message: String,
        #[snafu(source(from(Box<dyn std::error::Error + Send + Sync>, Some)))]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[snafu::report]
fn main() -> Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().pretty().with_thread_ids(true).with_target(true))
        .with(filter)
        .init();

    info!("Starting Courier v{}", env!("CARGO_PKG_VERSION"));

    #[allow(clippy::expect_used, reason = "app cannot start without a tokio runtime")]
    let runtime = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    let runtime_handle = runtime.handle().clone();

    let storage = runtime_handle
        .block_on(storage::Storage::open(configs::storage_path()))
        .whatever_context("Failed to open storage")?;
    let proxy = configs::read().network.proxy.clone();
    let client = plugin::Client::new(proxy.as_deref()).whatever_context("Failed to build the HTTP client")?;

    run_app(runtime_handle, storage, client)?;

    runtime.shutdown_timeout(Duration::from_secs(5));

    Ok(())
}

fn run_app(runtime: tokio::runtime::Handle, storage: storage::Storage, client: plugin::Client) -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 800.0]).with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native("Courier", options, Box::new(|cc| Ok(Box::new(App::new(cc, runtime, storage, client))))).whatever_context("eframe exited with an error")
}
