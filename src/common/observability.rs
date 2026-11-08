//! Local-only logging. TidyMac makes no network connections: there is no
//! crash reporter or telemetry SDK linked into the binary.

use anyhow::Result;
use std::path::PathBuf;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize logging to `~/Library/Logs/TidyMac/tidymac.log` (daily rotation).
pub fn init(verbose: bool) -> Result<()> {
    let log_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("Library/Logs/TidyMac");

    std::fs::create_dir_all(&log_dir)?;

    let file_appender = tracing_appender::rolling::daily(log_dir, "tidymac.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let file_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(true);

    let filter = if verbose {
        EnvFilter::new("tidymac=debug")
    } else {
        EnvFilter::new("tidymac=info")
    };

    let registry = tracing_subscriber::registry().with(filter).with(file_layer);

    // try_init: the GUI may call this more than once.
    let result = if verbose {
        let stdout_layer = fmt::layer().with_writer(std::io::stderr).with_target(false);
        registry.with(stdout_layer).try_init()
    } else {
        registry.try_init()
    };

    // Keep the background logging thread alive for the process lifetime.
    std::mem::forget(guard);
    let _ = result;
    Ok(())
}
