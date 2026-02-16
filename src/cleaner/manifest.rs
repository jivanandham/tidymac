use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::common::config::Config;

/// A complete manifest for one cleaning session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanManifest {
    /// Unique session identifier (timestamp-based)
    pub session_id: String,

    /// When the clean started
    pub timestamp: DateTime<Utc>,

    /// Profile used for this clean
    pub profile: String,

    /// Cleaning mode used
    pub mode: String,

    /// Total bytes freed (or staged)
    pub total_bytes: u64,

    /// Total files affected
    pub total_files: usize,

    /// When staged files expire (soft-delete only)
    pub expires_at: Option<DateTime<Utc>>,

    /// Whether this session has been restored via undo
    pub restored: bool,

    /// Individual items that were cleaned
    pub items: Vec<ManifestItem>,

    /// Errors encountered
    pub errors: Vec<String>,
}

/// A single file/directory entry in the manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestItem {
    /// Original path before cleaning
    pub original_path: PathBuf,

    /// Path in staging area (soft-delete only)
    pub staged_path: Option<PathBuf>,

    /// File size in bytes
    pub size_bytes: u64,

    /// Category label
    pub category: String,

    /// Safety level
    pub safety: String,

    /// Whether this was a file or directory
    pub is_dir: bool,

    /// Whether this item was successfully processed
    pub success: bool,

    /// Error message if processing failed
    pub error: Option<String>,
}

impl CleanManifest {
    /// Create a new manifest for a cleaning session
    pub fn new(profile: &str, mode: &str, retention_days: u32) -> Self {
        let now = Utc::now();
        let session_id = now.format("%Y-%m-%dT%H-%M-%S").to_string();
        let expires_at = if mode == "soft_delete" {
            Some(now + Duration::days(retention_days as i64))
        } else {
            None
        };

        Self {
            session_id,
            timestamp: now,
            profile: profile.to_string(),
            mode: mode.to_string(),
            total_bytes: 0,
            total_files: 0,
            expires_at,
            restored: false,
            items: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Add a successfully processed item
    pub fn add_item(&mut self, item: ManifestItem) {
        if item.success {
            self.total_bytes += item.size_bytes;
            self.total_files += 1;
        }
        self.items.push(item);
    }

    /// Add an error
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    /// Check if this session's staged files have expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() > expires_at
        } else {
            false
        }
    }

    /// Get the staging directory for this session
    pub fn staging_session_dir(&self) -> PathBuf {
        Config::staging_dir().join(&self.session_id)
    }

    /// Get the files subdirectory within the staging session
    pub fn staging_files_dir(&self) -> PathBuf {
        self.staging_session_dir().join("files")
    }

    /// Save manifest to disk as JSON
    pub fn save(&self) -> Result<()> {
        // Save to staging session directory (for soft-delete)
        if self.mode == "soft_delete" {
            let session_dir = self.staging_session_dir();
            std::fs::create_dir_all(&session_dir).with_context(|| {
                format!("Failed to create session dir: {}", session_dir.display())
            })?;

            let manifest_path = session_dir.join("manifest.json");
            let json =
                serde_json::to_string_pretty(self).context("Failed to serialize manifest")?;
            std::fs::write(&manifest_path, &json).with_context(|| {
                format!("Failed to write manifest: {}", manifest_path.display())
            })?;
        }

        // Append to the daily log file (JSONL format)
        let log_dir = Config::logs_dir();
        std::fs::create_dir_all(&log_dir)
            .with_context(|| format!("Failed to create logs dir: {}", log_dir.display()))?;

        let log_date = self.timestamp.format("%Y-%m-%d").to_string();
        let log_path = log_dir.join(format!("clean-{}.jsonl", log_date));

        let log_entry =
            serde_json::to_string(self).context("Failed to serialize log entry")?;

        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .with_context(|| format!("Failed to open log: {}", log_path.display()))?;
        writeln!(file, "{}", log_entry)?;

        Ok(())
    }

    /// Load a manifest from a session directory
    pub fn load_from_session(session_id: &str) -> Result<Self> {
        let manifest_path = Config::staging_dir()
            .join(session_id)
            .join("manifest.json");

        if !manifest_path.exists() {
            anyhow::bail!("Session '{}' not found", session_id);
        }

        let contents = std::fs::read_to_string(&manifest_path).with_context(|| {
            format!("Failed to read manifest: {}", manifest_path.display())
        })?;

        let manifest: CleanManifest = serde_json::from_str(&contents).with_context(|| {
            format!("Failed to parse manifest: {}", manifest_path.display())
        })?;

        Ok(manifest)
    }

    /// List all available sessions in the staging area
    pub fn list_sessions() -> Result<Vec<SessionSummary>> {
        let staging_dir = Config::staging_dir();
        if !staging_dir.exists() {
            return Ok(Vec::new());
        }

        let mut sessions = Vec::new();

        for entry in std::fs::read_dir(&staging_dir)
            .with_context(|| format!("Failed to read staging dir: {}", staging_dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            let manifest_path = path.join("manifest.json");
            if !manifest_path.exists() {
                continue;
            }

            if let Ok(contents) = std::fs::read_to_string(&manifest_path) {
                if let Ok(manifest) = serde_json::from_str::<CleanManifest>(&contents) {
                    let staged_size = crate::scanner::walker::dir_size(&path);
                    let is_expired = manifest.is_expired();
                    sessions.push(SessionSummary {
                        session_id: manifest.session_id.clone(),
                        timestamp: manifest.timestamp,
                        profile: manifest.profile.clone(),
                        mode: manifest.mode.clone(),
                        total_bytes: manifest.total_bytes,
                        total_files: manifest.total_files,
                        staged_size,
                        expires_at: manifest.expires_at,
                        restored: manifest.restored,
                        is_expired,
                    });
                }
            }
        }

        // Sort by timestamp descending (most recent first)
        sessions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(sessions)
    }

    /// Get the most recent session ID
    pub fn most_recent_session() -> Result<Option<String>> {
        let sessions = Self::list_sessions()?;
        Ok(sessions.first().map(|s| s.session_id.clone()))
    }

    /// Mark this session as restored
    pub fn mark_restored(&mut self) -> Result<()> {
        self.restored = true;
        self.save()?;
        Ok(())
    }
}

/// Summary info about a staging session (for listing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub timestamp: DateTime<Utc>,
    pub profile: String,
    pub mode: String,
    pub total_bytes: u64,
    pub total_files: usize,
    pub staged_size: u64,
    pub expires_at: Option<DateTime<Utc>>,
    pub restored: bool,
    pub is_expired: bool,
}
