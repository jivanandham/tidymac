use anyhow::{Context, Result};
use std::path::PathBuf;

use super::manifest::CleanManifest;
use crate::common::config::Config;

/// Purge expired staging sessions
///
/// This checks all sessions in the staging area and removes
/// those whose retention period has elapsed.
///
/// Returns a report of what was purged.
pub fn purge_expired() -> Result<PurgeReport> {
    let sessions = CleanManifest::list_sessions()?;
    let mut report = PurgeReport {
        purged_sessions: Vec::new(),
        total_bytes_freed: 0,
        errors: Vec::new(),
    };

    for session in &sessions {
        if !session.is_expired {
            continue;
        }

        // Don't purge restored sessions (they're already empty)
        if session.restored {
            // Just clean up the manifest directory
            let session_dir = Config::staging_dir().join(&session.session_id);
            if session_dir.exists() {
                let _ = std::fs::remove_dir_all(&session_dir);
            }
            continue;
        }

        let session_dir = Config::staging_dir().join(&session.session_id);
        if !session_dir.exists() {
            continue;
        }

        // Calculate actual size before removal
        let size = crate::scanner::walker::dir_size(&session_dir);

        match std::fs::remove_dir_all(&session_dir) {
            Ok(()) => {
                report.purged_sessions.push(PurgedSession {
                    session_id: session.session_id.clone(),
                    bytes_freed: size,
                    file_count: session.total_files,
                });
                report.total_bytes_freed += size;
            }
            Err(e) => {
                report.errors.push(format!(
                    "Failed to purge session '{}': {}",
                    session.session_id, e
                ));
            }
        }
    }

    Ok(report)
}

/// Purge a specific session by ID
pub fn purge_session(session_id: &str) -> Result<u64> {
    let session_dir = Config::staging_dir().join(session_id);

    if !session_dir.exists() {
        anyhow::bail!("Session '{}' not found", session_id);
    }

    let size = crate::scanner::walker::dir_size(&session_dir);

    std::fs::remove_dir_all(&session_dir)
        .with_context(|| format!("Failed to purge session: {}", session_id))?;

    Ok(size)
}

/// Purge ALL sessions (nuclear option)
pub fn purge_all() -> Result<PurgeReport> {
    let staging_dir = Config::staging_dir();
    let mut report = PurgeReport {
        purged_sessions: Vec::new(),
        total_bytes_freed: 0,
        errors: Vec::new(),
    };

    if !staging_dir.exists() {
        return Ok(report);
    }

    for entry in std::fs::read_dir(&staging_dir)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let session_id = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let size = crate::scanner::walker::dir_size(&path);

        match std::fs::remove_dir_all(&path) {
            Ok(()) => {
                report.purged_sessions.push(PurgedSession {
                    session_id,
                    bytes_freed: size,
                    file_count: 0,
                });
                report.total_bytes_freed += size;
            }
            Err(e) => {
                report
                    .errors
                    .push(format!("Failed to purge '{}': {}", session_id, e));
            }
        }
    }

    Ok(report)
}

/// Generate a launchd plist for automatic purging
///
/// This creates a plist that runs `tidymac purge` daily at 3am.
/// Install to ~/Library/LaunchAgents/ and load with launchctl.
pub fn generate_purge_plist() -> String {
    let binary_path = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "/usr/local/bin/tidymac".to_string());

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.tidymac.purge</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
        <string>purge</string>
        <string>--expired</string>
    </array>
    <key>StartCalendarInterval</key>
    <dict>
        <key>Hour</key>
        <integer>3</integer>
        <key>Minute</key>
        <integer>0</integer>
    </dict>
    <key>StandardOutPath</key>
    <string>{}/logs/purge.log</string>
    <key>StandardErrorPath</key>
    <string>{}/logs/purge-error.log</string>
</dict>
</plist>"#,
        binary_path,
        Config::data_dir().display(),
        Config::data_dir().display(),
    )
}

/// Get the path where the purge plist should be installed
pub fn purge_plist_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join("Library/LaunchAgents/com.tidymac.purge.plist")
}

/// Report from a purge operation
#[derive(Debug)]
pub struct PurgeReport {
    pub purged_sessions: Vec<PurgedSession>,
    pub total_bytes_freed: u64,
    pub errors: Vec<String>,
}

#[derive(Debug)]
pub struct PurgedSession {
    pub session_id: String,
    pub bytes_freed: u64,
    pub file_count: usize,
}
