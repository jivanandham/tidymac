use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Global TidyMac configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Default clean mode
    #[serde(default = "default_clean_mode")]
    pub default_mode: CleanMode,

    /// Default profile name
    #[serde(default = "default_profile")]
    pub default_profile: String,

    /// Staging area retention in days
    #[serde(default = "default_retention_days")]
    pub staging_retention_days: u32,

    /// Large file threshold in MB
    #[serde(default = "default_large_file_mb")]
    pub large_file_threshold_mb: u64,

    /// Stale threshold in days (for node_modules, venvs, etc.)
    #[serde(default = "default_stale_days")]
    pub stale_days: u32,

    /// Paths to exclude from scanning
    #[serde(default)]
    pub exclude_paths: Vec<String>,

    /// Output format preference
    #[serde(default)]
    pub output_format: OutputFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CleanMode {
    DryRun,
    SoftDelete,
    HardDelete,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    #[default]
    Human,
    Json,
    Quiet,
}

fn default_clean_mode() -> CleanMode {
    CleanMode::DryRun
}
fn default_profile() -> String {
    "quick_sweep".to_string()
}
fn default_retention_days() -> u32 {
    7
}
fn default_large_file_mb() -> u64 {
    500
}
fn default_stale_days() -> u32 {
    30
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_mode: default_clean_mode(),
            default_profile: default_profile(),
            staging_retention_days: default_retention_days(),
            large_file_threshold_mb: default_large_file_mb(),
            stale_days: default_stale_days(),
            exclude_paths: Vec::new(),
            output_format: OutputFormat::Human,
        }
    }
}

impl Config {
    /// Get the TidyMac data directory (~/.tidymac)
    pub fn data_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join(".tidymac")
    }

    /// Get the config file path
    pub fn config_path() -> PathBuf {
        Self::data_dir().join("config.toml")
    }

    /// Get the staging directory
    pub fn staging_dir() -> PathBuf {
        Self::data_dir().join("staging")
    }

    /// Get the logs directory
    pub fn logs_dir() -> PathBuf {
        Self::data_dir().join("logs")
    }

    /// Get the profiles directory
    pub fn profiles_dir() -> PathBuf {
        Self::data_dir().join("profiles")
    }

    /// Load config from file, or create default if not exists
    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if path.exists() {
            let contents = std::fs::read_to_string(&path)
                .with_context(|| format!("Failed to read config: {}", path.display()))?;
            let config: Config = toml::from_str(&contents)
                .with_context(|| format!("Failed to parse config: {}", path.display()))?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    /// Save config to file
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        let dir = path.parent().unwrap();
        std::fs::create_dir_all(dir)
            .with_context(|| format!("Failed to create config dir: {}", dir.display()))?;
        let contents = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;
        std::fs::write(&path, contents)
            .with_context(|| format!("Failed to write config: {}", path.display()))?;
        Ok(())
    }

    /// Initialize all TidyMac directories
    pub fn init_dirs() -> Result<()> {
        let dirs = [
            Self::data_dir(),
            Self::staging_dir(),
            Self::logs_dir(),
            Self::profiles_dir(),
        ];
        for dir in &dirs {
            std::fs::create_dir_all(dir)
                .with_context(|| format!("Failed to create directory: {}", dir.display()))?;
        }
        Ok(())
    }

    /// Get large file threshold in bytes
    pub fn large_file_threshold_bytes(&self) -> u64 {
        self.large_file_threshold_mb * 1024 * 1024
    }

    /// Check if a path should be excluded
    pub fn is_excluded(&self, path: &Path) -> bool {
        let path_str = path.display().to_string();
        self.exclude_paths.iter().any(|p| path_str.contains(p))
    }
}
