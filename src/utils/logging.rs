use crate::config::Settings;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

/// Initialize structured logging for the application
pub fn init_logging(settings: &Settings) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&settings.logging.rust_log));

    let fmt_layer = if settings.logging.log_format == "json" {
        fmt::layer().json().boxed()
    } else {
        fmt::layer().pretty().boxed()
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .init();

    tracing::info!(
        "Logging initialized - Level: {}, Format: {}",
        settings.logging.rust_log,
        settings.logging.log_format
    );
}
