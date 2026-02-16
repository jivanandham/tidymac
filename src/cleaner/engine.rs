use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;

use super::manifest::{CleanManifest, ManifestItem};
use super::staging;
use crate::common::config::Config;
use crate::common::format;
use crate::scanner::targets::ScanItem;

/// Clean mode determines how files are removed
#[derive(Debug, Clone, PartialEq)]
pub enum CleanMode {
    /// Show what would be done without doing it
    DryRun,
    /// Move to staging area for recovery (default)
    SoftDelete,
    /// Permanent removal — no undo
    HardDelete,
}

impl std::fmt::Display for CleanMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CleanMode::DryRun => write!(f, "dry_run"),
            CleanMode::SoftDelete => write!(f, "soft_delete"),
            CleanMode::HardDelete => write!(f, "hard_delete"),
        }
    }
}

/// Report from a clean operation
#[derive(Debug)]
pub struct CleanReport {
    pub mode: CleanMode,
    pub files_removed: usize,
    pub bytes_freed: u64,
    pub session_id: Option<String>,
    pub errors: Vec<String>,
}

/// Execute a cleaning operation on the given scan items
///
/// This is the main entry point for all cleaning operations.
/// - DryRun: just reports what would be done
/// - SoftDelete: moves files to staging area with manifest for undo
/// - HardDelete: permanently removes files
pub fn clean(
    items: &[ScanItem],
    mode: CleanMode,
    profile_name: &str,
    show_progress: bool,
) -> Result<CleanReport> {
    let config = Config::load()?;

    // Ensure TidyMac directories exist
    Config::init_dirs()?;

    // Safety check: validate no protected paths are being cleaned
    for item in items {
        if crate::common::safety::is_protected(&item.path) {
            anyhow::bail!(
                "SAFETY: Refusing to clean protected path: {}",
                item.path.display()
            );
        }
        for f in &item.files {
            if crate::common::safety::is_protected(&f.path) {
                anyhow::bail!(
                    "SAFETY: Refusing to clean protected path: {}",
                    f.path.display()
                );
            }
        }
    }

    match mode {
        CleanMode::DryRun => clean_dry_run(items),
        CleanMode::SoftDelete => {
            clean_soft_delete(items, profile_name, config.staging_retention_days, show_progress)
        }
        CleanMode::HardDelete => clean_hard_delete(items, profile_name, show_progress),
    }
}

/// Dry run — just tally up what would be cleaned
fn clean_dry_run(items: &[ScanItem]) -> Result<CleanReport> {
    let mut total_files = 0usize;
    let mut total_bytes = 0u64;

    for item in items {
        if item.files.is_empty() {
            total_files += 1;
            total_bytes += item.size_bytes;
        } else {
            total_files += item.files.len();
            total_bytes += item.files.iter().map(|f| f.size_bytes).sum::<u64>();
        }
    }

    Ok(CleanReport {
        mode: CleanMode::DryRun,
        files_removed: total_files,
        bytes_freed: total_bytes,
        session_id: None,
        errors: Vec::new(),
    })
}

/// Soft delete — move files to staging area with manifest
fn clean_soft_delete(
    items: &[ScanItem],
    profile_name: &str,
    retention_days: u32,
    show_progress: bool,
) -> Result<CleanReport> {
    let mut manifest = CleanManifest::new(profile_name, "soft_delete", retention_days);

    // Stage all files
    staging::stage_files(items, &mut manifest, show_progress)?;

    // Save the manifest
    manifest.save().context("Failed to save clean manifest")?;

    let session_id = manifest.session_id.clone();
    let report = CleanReport {
        mode: CleanMode::SoftDelete,
        files_removed: manifest.total_files,
        bytes_freed: manifest.total_bytes,
        session_id: Some(session_id),
        errors: manifest.errors.clone(),
    };

    Ok(report)
}

/// Hard delete — permanently remove files with manifest logging
fn clean_hard_delete(
    items: &[ScanItem],
    profile_name: &str,
    show_progress: bool,
) -> Result<CleanReport> {
    let mut manifest = CleanManifest::new(profile_name, "hard_delete", 0);

    // Count total files for progress
    let total_files: usize = items
        .iter()
        .map(|item| if item.files.is_empty() { 1 } else { item.files.len() })
        .sum();

    let pb = if show_progress {
        let pb = ProgressBar::new(total_files as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner:.red} [{bar:40.red/blue}] {pos}/{len} Deleting... {msg}",
                )
                .unwrap()
                .progress_chars("━━░"),
        );
        Some(pb)
    } else {
        None
    };

    for item in items {
        if item.files.is_empty() {
            // Delete the item path directly
            let result = hard_delete_path(&item.path);
            manifest.add_item(ManifestItem {
                original_path: item.path.clone(),
                staged_path: None,
                size_bytes: item.size_bytes,
                category: format!("{}", item.category),
                safety: format!("{:?}", item.safety),
                is_dir: item.path.is_dir(),
                success: result.is_ok(),
                error: result.err().map(|e| e.to_string()),
            });

            if let Some(ref pb) = pb {
                pb.inc(1);
            }
        } else {
            for file_entry in &item.files {
                if let Some(ref pb) = pb {
                    pb.set_message(format::truncate(
                        &format::format_path(&file_entry.path),
                        40,
                    ));
                }

                let result = hard_delete_path(&file_entry.path);
                manifest.add_item(ManifestItem {
                    original_path: file_entry.path.clone(),
                    staged_path: None,
                    size_bytes: file_entry.size_bytes,
                    category: format!("{}", item.category),
                    safety: format!("{:?}", item.safety),
                    is_dir: file_entry.path.is_dir(),
                    success: result.is_ok(),
                    error: result.err().map(|e| e.to_string()),
                });

                if let Some(ref pb) = pb {
                    pb.inc(1);
                }
            }
        }
    }

    if let Some(ref pb) = pb {
        pb.finish_and_clear();
    }

    // Save manifest to logs (even for hard delete, for audit trail)
    manifest
        .save()
        .context("Failed to save clean manifest log")?;

    Ok(CleanReport {
        mode: CleanMode::HardDelete,
        files_removed: manifest.total_files,
        bytes_freed: manifest.total_bytes,
        session_id: None,
        errors: manifest.errors.clone(),
    })
}

/// Delete a single file or directory permanently
fn hard_delete_path(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(()); // Already gone
    }

    if path.is_dir() {
        std::fs::remove_dir_all(path)
            .with_context(|| format!("Failed to remove directory: {}", path.display()))?;
    } else {
        std::fs::remove_file(path)
            .with_context(|| format!("Failed to remove file: {}", path.display()))?;
    }

    Ok(())
}

/// Check if the staging area is getting too large and warn
pub fn check_staging_health() -> Result<StagingHealth> {
    let staging_dir = Config::staging_dir();
    if !staging_dir.exists() {
        return Ok(StagingHealth {
            total_size: 0,
            session_count: 0,
            expired_count: 0,
            expired_size: 0,
            warning: None,
        });
    }

    let sessions = CleanManifest::list_sessions()?;
    let total_size: u64 = sessions.iter().map(|s| s.staged_size).sum();
    let expired: Vec<_> = sessions.iter().filter(|s| s.is_expired).collect();
    let expired_size: u64 = expired.iter().map(|s| s.staged_size).sum();

    let warning = if total_size > 5 * 1024 * 1024 * 1024 {
        // > 5GB
        Some(format!(
            "Staging area is using {}. Run 'tidymac purge' to free space.",
            format::format_size(total_size)
        ))
    } else if expired.len() > 10 {
        Some(format!(
            "{} expired sessions found. Run 'tidymac purge --expired' to clean up.",
            expired.len()
        ))
    } else {
        None
    };

    Ok(StagingHealth {
        total_size,
        session_count: sessions.len(),
        expired_count: expired.len(),
        expired_size,
        warning,
    })
}

/// Health status of the staging area
#[derive(Debug)]
pub struct StagingHealth {
    pub total_size: u64,
    pub session_count: usize,
    pub expired_count: usize,
    pub expired_size: u64,
    pub warning: Option<String>,
}
