use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::common::config::Config;
use crate::scanner::targets::ScanItem;

/// Cache entry for a single scan target directory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// Path that was scanned
    pub path: PathBuf,
    /// Directory mtime at scan time
    pub mtime_secs: u64,
    /// Total size found
    pub size_bytes: u64,
    /// File count found
    pub file_count: usize,
    /// Category string
    pub category: String,
    /// Name of the scan item
    pub name: String,
    /// Safety level string
    pub safety: String,
    /// Reason string
    pub reason: String,
}

/// Complete scan cache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanCache {
    /// Profile name this cache was built with
    pub profile: String,
    /// When the cache was last updated
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Cached entries keyed by path string
    pub entries: HashMap<String, CacheEntry>,
    /// Cache hit statistics
    #[serde(default)]
    pub stats: CacheStats,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStats {
    pub hits: usize,
    pub misses: usize,
    pub invalidated: usize,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64 * 100.0
        }
    }
}

impl ScanCache {
    /// Create a new empty cache for a profile
    pub fn new(profile: &str) -> Self {
        Self {
            profile: profile.to_string(),
            timestamp: chrono::Utc::now(),
            entries: HashMap::new(),
            stats: CacheStats::default(),
        }
    }

    /// Get cache file path
    pub fn cache_path() -> PathBuf {
        Config::data_dir().join("scan_cache.json")
    }

    /// Load cache from disk, returns None if missing or invalid
    pub fn load(profile: &str) -> Option<Self> {
        let path = Self::cache_path();
        if !path.exists() {
            return None;
        }

        let contents = std::fs::read_to_string(&path).ok()?;
        let cache: ScanCache = serde_json::from_str(&contents).ok()?;

        // Invalidate if profile changed
        if cache.profile != profile {
            return None;
        }

        Some(cache)
    }

    /// Save cache to disk
    pub fn save(&mut self) -> Result<()> {
        self.timestamp = chrono::Utc::now();
        let path = Self::cache_path();

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(self)
            .context("Failed to serialize scan cache")?;
        std::fs::write(&path, json)
            .with_context(|| format!("Failed to write cache: {}", path.display()))?;

        Ok(())
    }

    /// Check if a path has changed since last scan
    /// Returns the cached entry if still valid, None if needs re-scan
    pub fn check(&mut self, path: &Path) -> Option<&CacheEntry> {
        let key = path.display().to_string();

        // Get current directory mtime
        let current_mtime = dir_mtime(path)?;

        if let Some(entry) = self.entries.get(&key) {
            if entry.mtime_secs == current_mtime {
                self.stats.hits += 1;
                return Some(entry);
            } else {
                self.stats.invalidated += 1;
                return None;
            }
        }

        self.stats.misses += 1;
        None
    }

    /// Store a scan result in the cache
    pub fn store(&mut self, item: &ScanItem) {
        let key = item.path.display().to_string();
        let mtime = dir_mtime(&item.path).unwrap_or(0);

        self.entries.insert(
            key,
            CacheEntry {
                path: item.path.clone(),
                mtime_secs: mtime,
                size_bytes: item.size_bytes,
                file_count: item.file_count,
                category: format!("{}", item.category),
                name: item.name.clone(),
                safety: format!("{:?}", item.safety),
                reason: item.reason.clone(),
            },
        );
    }

    /// Remove a path from cache (after cleaning)
    pub fn invalidate(&mut self, path: &Path) {
        let key = path.display().to_string();
        self.entries.remove(&key);
    }

    /// Clear the entire cache
    pub fn clear() -> Result<()> {
        let path = Self::cache_path();
        if path.exists() {
            std::fs::remove_file(&path)
                .with_context(|| format!("Failed to remove cache: {}", path.display()))?;
        }
        Ok(())
    }

    /// Get cache age as a human-readable string
    pub fn age_string(&self) -> String {
        let now = chrono::Utc::now();
        let duration = now - self.timestamp;
        let secs = duration.num_seconds();

        if secs < 60 {
            format!("{}s ago", secs)
        } else if secs < 3600 {
            format!("{}m ago", secs / 60)
        } else if secs < 86400 {
            format!("{}h ago", secs / 3600)
        } else {
            format!("{}d ago", secs / 86400)
        }
    }

    /// Number of cached entries
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
}

/// Get a directory's modification time as seconds since epoch
fn dir_mtime(path: &Path) -> Option<u64> {
    std::fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs())
}
