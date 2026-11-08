//! Moving entries into the staging area (soft delete) and back (undo).

use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};

use super::manifest::{CleanManifest, ManifestItem};
use crate::common::format;

/// Error raised when an entry lives on a different volume than the staging
/// area. Moving it would copy the data onto the startup disk, so we don't.
#[derive(Debug)]
pub struct CrossVolume;

impl std::fmt::Display for CrossVolume {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "on a different volume than the staging area — use permanent delete")
    }
}

impl std::error::Error for CrossVolume {}

const EXDEV: i32 = 18;

/// Build the staging path for the n-th entry, keeping its original name for readability.
pub fn staged_path_for(files_dir: &Path, index: usize, original: &Path) -> PathBuf {
    let base = original
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "item".to_string());
    files_dir.join(format!("{:06}-{}", index, base))
}

/// Move a single file or directory into the staging area (same volume only).
pub fn stage_single_path(original: &Path, staged: &Path) -> Result<()> {
    if std::fs::symlink_metadata(original).is_err() {
        anyhow::bail!("Path does not exist: {}", original.display());
    }
    if let Some(parent) = staged.parent() {
        std::fs::create_dir_all(parent)?;
    }
    match std::fs::rename(original, staged) {
        Ok(()) => Ok(()),
        Err(e) if e.raw_os_error() == Some(EXDEV) => Err(CrossVolume.into()),
        Err(e) => Err(anyhow::Error::new(e)
            .context(format!("Failed to move '{}' to staging", original.display()))),
    }
}

/// Make a tree writable by its owner so it can be removed (Go's module cache,
/// some npm/pnpm installs and app bundles contain read-only folders).
fn make_owner_writable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    for entry in walkdir::WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_symlink() {
            continue;
        }
        if let Ok(md) = entry.metadata() {
            let mode = md.permissions().mode();
            if mode & 0o200 == 0 {
                let mut perms = md.permissions();
                perms.set_mode(mode | 0o200);
                let _ = std::fs::set_permissions(entry.path(), perms);
            }
        }
    }
}

/// Permanently remove a file, symlink or directory tree. Symlinks are removed,
/// never followed. Retries once after fixing read-only permissions.
pub fn remove_tree(path: &Path) -> Result<()> {
    let md = match std::fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(_) => return Ok(()), // already gone
    };
    let attempt = |p: &Path| -> std::io::Result<()> {
        if md.is_dir() {
            std::fs::remove_dir_all(p)
        } else {
            std::fs::remove_file(p)
        }
    };
    match attempt(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            make_owner_writable(path);
            attempt(path).with_context(|| format!("Failed to remove {}", path.display()))
        }
        Err(e) => Err(anyhow::Error::new(e).context(format!("Failed to remove {}", path.display()))),
    }
}

/// Restore files from staging back to their original locations
pub fn restore_session(session_id: &str, show_progress: bool) -> Result<RestoreReport> {
    let mut manifest = CleanManifest::load_from_session(session_id)?;

    if manifest.restored {
        anyhow::bail!("Session '{}' has already been restored", session_id);
    }

    let restorable_items: Vec<ManifestItem> = manifest
        .items
        .iter()
        .filter(|i| i.success && i.staged_path.is_some())
        .cloned()
        .collect();

    if restorable_items.is_empty() {
        anyhow::bail!("No restorable items in session '{}'", session_id);
    }

    let pb = if show_progress {
        let pb = ProgressBar::new(restorable_items.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.green/blue}] {pos}/{len} Restoring... {msg}")
                .unwrap()
                .progress_chars("━━░"),
        );
        Some(pb)
    } else {
        None
    };

    let mut restored_count = 0usize;
    let mut restored_bytes = 0u64;
    let mut errors = Vec::new();

    for item in &restorable_items {
        let staged_path = item.staged_path.as_ref().unwrap();
        let original_path = &item.original_path;

        if let Some(ref pb) = pb {
            pb.set_message(format::truncate(&format::format_path(original_path), 40));
        }

        if let Some(parent) = original_path.parent() {
            if !parent.exists() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    errors.push(format!(
                        "Failed to create parent dir '{}': {}",
                        parent.display(),
                        e
                    ));
                    if let Some(ref pb) = pb {
                        pb.inc(1);
                    }
                    continue;
                }
            }
        }

        match restore_single_path(staged_path, original_path) {
            Ok(()) => {
                restored_count += 1;
                restored_bytes += item.size_bytes;
            }
            Err(e) => {
                errors.push(format!(
                    "Failed to restore '{}': {}",
                    original_path.display(),
                    e
                ));
            }
        }

        if let Some(ref pb) = pb {
            pb.inc(1);
        }
    }

    if let Some(ref pb) = pb {
        pb.finish_and_clear();
    }

    manifest.mark_restored()?;

    let files_dir = manifest.staging_files_dir();
    let _ = cleanup_empty_dirs(&files_dir);

    Ok(RestoreReport {
        session_id: session_id.to_string(),
        restored_count,
        restored_bytes,
        errors,
    })
}

/// Move a single file/directory back from staging to original path
fn restore_single_path(staged: &Path, original: &Path) -> Result<()> {
    if std::fs::symlink_metadata(staged).is_err() {
        anyhow::bail!("Staged file no longer exists: {}", staged.display());
    }

    // Don't overwrite if something already exists at the original path.
    if std::fs::symlink_metadata(original).is_ok() {
        anyhow::bail!(
            "Original path already exists (won't overwrite): {}. \
             Move the existing item away first, then undo again.",
            original.display()
        );
    }

    std::fs::rename(staged, original)
        .with_context(|| format!("Failed to move '{}' back", staged.display()))
}

/// Remove empty directories recursively (cleanup after restore)
fn cleanup_empty_dirs(dir: &Path) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let ft = entry.file_type()?;
        if ft.is_dir() {
            cleanup_empty_dirs(&entry.path())?;
        }
    }

    if std::fs::read_dir(dir)?.next().is_none() {
        std::fs::remove_dir(dir)?;
    }

    Ok(())
}

/// Report from a restore operation
#[derive(Debug)]
pub struct RestoreReport {
    pub session_id: String,
    pub restored_count: usize,
    pub restored_bytes: u64,
    pub errors: Vec<String>,
}
