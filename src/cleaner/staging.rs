use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;

use super::manifest::{CleanManifest, ManifestItem};
use crate::common::config::Config;
use crate::common::format;
use crate::scanner::targets::ScanItem;

/// Move files to the staging area for soft-delete with recovery
pub fn stage_files(
    items: &[ScanItem],
    manifest: &mut CleanManifest,
    show_progress: bool,
) -> Result<()> {
    let files_dir = manifest.staging_files_dir();
    std::fs::create_dir_all(&files_dir)
        .with_context(|| format!("Failed to create staging files dir: {}", files_dir.display()))?;

    // Count total files for progress bar
    let total_files: usize = items.iter().map(|item| {
        if item.files.is_empty() { 1 } else { item.files.len() }
    }).sum();

    let pb = if show_progress {
        let pb = ProgressBar::new(total_files as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.cyan} [{bar:40.cyan/blue}] {pos}/{len} Staging files... {msg}")
                .unwrap()
                .progress_chars("━━░"),
        );
        Some(pb)
    } else {
        None
    };

    let mut file_counter: usize = 0;

    for item in items {
        if item.files.is_empty() {
            // The item path itself is the target (e.g., a directory)
            file_counter += 1;
            let staged_name = format!("{:06}", file_counter);
            let staged_path = files_dir.join(&staged_name);

            match stage_single_path(&item.path, &staged_path) {
                Ok(()) => {
                    manifest.add_item(ManifestItem {
                        original_path: item.path.clone(),
                        staged_path: Some(staged_path),
                        size_bytes: item.size_bytes,
                        category: format!("{}", item.category),
                        safety: format!("{:?}", item.safety),
                        is_dir: item.path.is_dir(),
                        success: true,
                        error: None,
                    });
                }
                Err(e) => {
                    let err_msg = format!(
                        "Failed to stage '{}': {}",
                        item.path.display(),
                        e
                    );
                    manifest.add_item(ManifestItem {
                        original_path: item.path.clone(),
                        staged_path: None,
                        size_bytes: item.size_bytes,
                        category: format!("{}", item.category),
                        safety: format!("{:?}", item.safety),
                        is_dir: item.path.is_dir(),
                        success: false,
                        error: Some(err_msg.clone()),
                    });
                    manifest.add_error(err_msg);
                }
            }

            if let Some(ref pb) = pb {
                pb.inc(1);
            }
        } else {
            // Process individual files within the item
            for file_entry in &item.files {
                file_counter += 1;
                let staged_name = format!("{:06}", file_counter);
                let staged_path = files_dir.join(&staged_name);

                match stage_single_path(&file_entry.path, &staged_path) {
                    Ok(()) => {
                        manifest.add_item(ManifestItem {
                            original_path: file_entry.path.clone(),
                            staged_path: Some(staged_path),
                            size_bytes: file_entry.size_bytes,
                            category: format!("{}", item.category),
                            safety: format!("{:?}", item.safety),
                            is_dir: file_entry.path.is_dir(),
                            success: true,
                            error: None,
                        });
                    }
                    Err(e) => {
                        let err_msg = format!(
                            "Failed to stage '{}': {}",
                            file_entry.path.display(),
                            e
                        );
                        manifest.add_item(ManifestItem {
                            original_path: file_entry.path.clone(),
                            staged_path: None,
                            size_bytes: file_entry.size_bytes,
                            category: format!("{}", item.category),
                            safety: format!("{:?}", item.safety),
                            is_dir: file_entry.path.is_dir(),
                            success: false,
                            error: Some(err_msg.clone()),
                        });
                        manifest.add_error(err_msg);
                    }
                }

                if let Some(ref pb) = pb {
                    pb.set_message(format::truncate(
                        &format::format_path(&file_entry.path),
                        40,
                    ));
                    pb.inc(1);
                }
            }
        }
    }

    if let Some(ref pb) = pb {
        pb.finish_and_clear();
    }

    Ok(())
}

/// Move a single file or directory to the staging area
fn stage_single_path(original: &Path, staged: &Path) -> Result<()> {
    if !original.exists() {
        anyhow::bail!("Path does not exist: {}", original.display());
    }

    // Try rename first (fast, same filesystem)
    if std::fs::rename(original, staged).is_ok() {
        return Ok(());
    }

    // Fallback: copy then delete (cross-filesystem)
    if original.is_dir() {
        copy_dir_recursive(original, staged)?;
        std::fs::remove_dir_all(original).with_context(|| {
            format!(
                "Staged copy successful but failed to remove original: {}",
                original.display()
            )
        })?;
    } else {
        // Ensure parent directory exists
        if let Some(parent) = staged.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(original, staged).with_context(|| {
            format!(
                "Failed to copy '{}' to staging",
                original.display()
            )
        })?;
        std::fs::remove_file(original).with_context(|| {
            format!(
                "Staged copy successful but failed to remove original: {}",
                original.display()
            )
        })?;
    }

    Ok(())
}

/// Recursively copy a directory
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

/// Restore files from staging back to their original locations
pub fn restore_session(session_id: &str, show_progress: bool) -> Result<RestoreReport> {
    let mut manifest = CleanManifest::load_from_session(session_id)?;

    if manifest.restored {
        anyhow::bail!(
            "Session '{}' has already been restored",
            session_id
        );
    }

    let restorable_items: Vec<&ManifestItem> = manifest
        .items
        .iter()
        .filter(|i| i.success && i.staged_path.is_some())
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
            pb.set_message(format::truncate(
                &format::format_path(original_path),
                40,
            ));
        }

        // Ensure parent directory exists for restoration
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

        // Move from staging back to original location
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

    // Mark session as restored
    manifest.mark_restored()?;

    // Clean up the now-empty staging session directory
    let session_dir = Config::staging_dir().join(session_id);
    let _ = cleanup_empty_dirs(&session_dir);

    Ok(RestoreReport {
        session_id: session_id.to_string(),
        restored_count,
        restored_bytes,
        errors,
    })
}

/// Move a single file/directory back from staging to original path
fn restore_single_path(staged: &Path, original: &Path) -> Result<()> {
    if !staged.exists() {
        anyhow::bail!(
            "Staged file no longer exists: {}",
            staged.display()
        );
    }

    // Don't overwrite if something already exists at the original path
    if original.exists() {
        anyhow::bail!(
            "Original path already exists (won't overwrite): {}",
            original.display()
        );
    }

    // Try rename first
    if std::fs::rename(staged, original).is_ok() {
        return Ok(());
    }

    // Fallback: copy then delete
    if staged.is_dir() {
        copy_dir_recursive(staged, original)?;
        std::fs::remove_dir_all(staged)?;
    } else {
        if let Some(parent) = original.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(staged, original)?;
        std::fs::remove_file(staged)?;
    }

    Ok(())
}

/// Remove empty directories recursively (cleanup after restore)
fn cleanup_empty_dirs(dir: &Path) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    // Recurse into subdirectories first
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if entry.path().is_dir() {
            cleanup_empty_dirs(&entry.path())?;
        }
    }

    // If directory is now empty, remove it
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
