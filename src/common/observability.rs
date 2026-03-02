use anyhow::Result;
use std::path::PathBuf;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn init(verbose: bool, sentry_dsn: Option<&str>) -> Result<()> {
    // 1. Setup Sentry
    let _sentry = if let Some(dsn) = sentry_dsn {
        Some(sentry::init((dsn, sentry::ClientOptions {
            release: sentry::release_name!(),
            ..Default::default()
        })))
    } else {
        None
    };

    // 2. Setup Logging
    let log_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("Library/Logs/TidyMac");

    std::fs::create_dir_all(&log_dir)?;

    let file_appender = tracing_appender::rolling::daily(log_dir, "tidymac.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let file_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(true);

    let filter = if verbose {
        EnvFilter::new("tidymac=debug")
    } else {
        EnvFilter::new("tidymac=info")
    };

    let registry = tracing_subscriber::registry()
        .with(filter)
        .with(file_layer);

    if verbose {
        let stdout_layer = fmt::layer().with_writer(std::io::stderr).with_target(false);
        registry.with(stdout_layer).init();
    } else {
        registry.init();
    }

    // Leaking the guard to keep the background logging thread alive
    std::mem::forget(_guard);
    // Leaking sentry guard as well if we want it to persist for the app duration
    if let Some(s) = _sentry {
        std::mem::forget(s);
    }

    Ok(())
}
