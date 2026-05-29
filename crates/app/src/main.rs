#![feature(error_generic_member_access)]

use std::time::Duration;

use snafu::{Location, ResultExt, Snafu};
use tracing::info;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use ui::App;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("eframe error"))]
    Eframe {
        source: eframe::Error,
        #[snafu(implicit)]
        location: Location,
    },
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[snafu::report]
fn main() -> Result<()> {
    init_logging();

    info!("Starting Courier v{}", env!("CARGO_PKG_VERSION"));

    #[allow(clippy::expect_used, reason = "app cannot start without a tokio runtime")]
    let runtime = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    let runtime_handle = runtime.handle().clone();

    run_app(runtime_handle)?;

    runtime.shutdown_timeout(Duration::from_secs(5));

    Ok(())
}

fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().pretty().with_thread_ids(true).with_target(true))
        .with(filter)
        .init();
}

fn run_app(runtime: tokio::runtime::Handle) -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 800.0]).with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native("Courier", options, Box::new(|cc| Ok(Box::new(App::new(cc, runtime))))).context(EframeSnafu)
}
