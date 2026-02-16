use anyhow::Result;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use std::os::darwin::fs::MetadataExt;
use walkdir::WalkDir;

use super::targets::{FileEntry, ScanItem, ScanTarget};
use crate::common::permissions;

/// Walk a scan target and collect file information
pub fn walk_target(target: &ScanTarget) -> Result<ScanItem> {
    let total_size = AtomicU64::new(0);
    let total_count = AtomicUsize::new(0);
    let files = Arc::new(Mutex::new(Vec::new()));
    let errors = Arc::new(Mutex::new(Vec::new()));

    let expanded_paths = expand_paths(&target.paths);

    // Process each expanded path
    expanded_paths.par_iter().for_each(|base_path| {
        if !base_path.exists() {
            return;
        }

        if permissions::is_sip_protected(base_path) {
            errors
                .lock()
                .unwrap()
                .push(format!("Skipped SIP-protected: {}", base_path.display()));
            return;
        }

        let walker = if target.recursive {
            WalkDir::new(base_path).follow_links(false)
        } else {
            WalkDir::new(base_path).follow_links(false).max_depth(1)
        };

        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();

            // Skip directories themselves (we only count files)
            if entry.file_type().is_dir() {
                continue;
            }

            // Apply DMG filter for download targets
            if target.category == crate::scanner::targets::Category::DownloadedDmg {
                if let Some(ext) = path.extension() {
                    if ext != "dmg" && ext != "pkg" {
                        continue;
                    }
                } else {
                    continue;
                }
            }

            // Get file metadata
            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            // Use actual physical disk usage (not logical size) for sparse files
            let size = metadata.st_blocks() * 512;
            let modified = metadata.modified().ok();

            // Apply minimum age filter
            if let Some(min_days) = target.min_age_days {
                if let Some(mod_time) = modified {
                    let age = SystemTime::now()
                        .duration_since(mod_time)
                        .unwrap_or_default();
                    if age.as_secs() < (min_days as u64 * 86400) {
                        continue;
                    }
                }
            }

            total_size.fetch_add(size, Ordering::Relaxed);
            total_count.fetch_add(1, Ordering::Relaxed);

            files.lock().unwrap().push(FileEntry {
                path: path.to_path_buf(),
                size_bytes: size,
                modified,
            });
        }
    });

    let size = total_size.load(Ordering::Relaxed);
    let count = total_count.load(Ordering::Relaxed);
    let collected_files = Arc::try_unwrap(files).unwrap().into_inner().unwrap();

    Ok(ScanItem {
        name: target.name.clone(),
        category: target.category.clone(),
        path: expanded_paths.first().cloned().unwrap_or_default(),
        size_bytes: size,
        file_count: count,
        safety: target.safety.clone(),
        reason: target.reason.clone(),
        files: collected_files,
    })
}

/// Walk multiple targets in parallel
pub fn walk_targets(targets: &[ScanTarget]) -> Vec<Result<ScanItem>> {
    targets.par_iter().map(|t| walk_target(t)).collect()
}

/// Expand ~ and glob patterns in paths
pub fn expand_paths(paths: &[String]) -> Vec<PathBuf> {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/Users/unknown"));
    let mut expanded = Vec::new();

    for path_str in paths {
        let resolved = path_str.replace('~', &home.to_string_lossy());

        // Handle glob patterns
        if resolved.contains('*') {
            if let Ok(entries) = glob::glob(&resolved) {
                for entry in entries.filter_map(|e| e.ok()) {
                    expanded.push(entry);
                }
            }
        } else {
            expanded.push(PathBuf::from(resolved));
        }
    }

    expanded
}

/// Find node_modules directories in common project locations
pub fn find_node_modules(search_roots: &[PathBuf], stale_days: u32) -> Vec<FileEntry> {
    let results = Arc::new(Mutex::new(Vec::new()));
    let stale_threshold = std::time::Duration::from_secs(stale_days as u64 * 86400);

    search_roots.par_iter().for_each(|root| {
        if !root.exists() {
            return;
        }

        // Walk looking for node_modules directories
        for entry in WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                // Don't descend into node_modules or hidden dirs
                !(name == "node_modules" && e.depth() > 0)
                    && !name.starts_with('.')
                    && name != "Library"
            })
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy();

            if name == "node_modules" && entry.file_type().is_dir() {
                // Check if parent project is stale
                let package_json = path.parent().map(|p| p.join("package.json"));
                let is_stale = package_json
                    .and_then(|pj| std::fs::metadata(&pj).ok())
                    .and_then(|m| m.modified().ok())
                    .map(|mod_time| {
                        SystemTime::now()
                            .duration_since(mod_time)
                            .unwrap_or_default()
                            > stale_threshold
                    })
                    .unwrap_or(true);

                if is_stale {
                    let size = dir_size(path);
                    results.lock().unwrap().push(FileEntry {
                        path: path.to_path_buf(),
                        size_bytes: size,
                        modified: std::fs::metadata(path)
                            .ok()
                            .and_then(|m| m.modified().ok()),
                    });
                }
            }
        }
    });

    Arc::try_unwrap(results).unwrap().into_inner().unwrap()
}

/// Find Python virtual environments in common locations
pub fn find_venvs(search_roots: &[PathBuf], stale_days: u32) -> Vec<FileEntry> {
    let results = Arc::new(Mutex::new(Vec::new()));
    let stale_threshold = std::time::Duration::from_secs(stale_days as u64 * 86400);
    let venv_names = [".venv", "venv", ".env", "env"];

    search_roots.par_iter().for_each(|root| {
        if !root.exists() {
            return;
        }

        for entry in WalkDir::new(root)
            .follow_links(false)
            .max_depth(3)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !name.starts_with('.') || venv_names.contains(&name.as_ref())
            })
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy();

            if entry.file_type().is_dir() && venv_names.contains(&name.as_ref()) {
                // Verify it's actually a venv (has pyvenv.cfg or bin/python)
                let is_venv = path.join("pyvenv.cfg").exists()
                    || path.join("bin/python").exists()
                    || path.join("Scripts/python.exe").exists();

                if !is_venv {
                    continue;
                }

                let is_stale = std::fs::metadata(path)
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .map(|mod_time| {
                        SystemTime::now()
                            .duration_since(mod_time)
                            .unwrap_or_default()
                            > stale_threshold
                    })
                    .unwrap_or(true);

                if is_stale {
                    let size = dir_size(path);
                    results.lock().unwrap().push(FileEntry {
                        path: path.to_path_buf(),
                        size_bytes: size,
                        modified: std::fs::metadata(path)
                            .ok()
                            .and_then(|m| m.modified().ok()),
                    });
                }
            }
        }
    });

    Arc::try_unwrap(results).unwrap().into_inner().unwrap()
}

/// Calculate total size of a directory (physical disk usage)
pub fn dir_size(path: &Path) -> u64 {
    WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.metadata().map(|m| m.st_blocks() * 512).unwrap_or(0))
        .sum()
}

/// Find large files in a directory
pub fn find_large_files(root: &Path, threshold_bytes: u64) -> Vec<FileEntry> {
    let mut results = Vec::new();

    if !root.exists() {
        return results;
    }

    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            // Skip hidden dirs, node_modules, .git, etc.
            !name.starts_with('.') && name != "node_modules" && name != "Library"
        })
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            if let Ok(metadata) = entry.metadata() {
                let size = metadata.st_blocks() * 512;
                if size >= threshold_bytes {
                    results.push(FileEntry {
                        path: entry.path().to_path_buf(),
                        size_bytes: size,
                        modified: metadata.modified().ok(),
                    });
                }
            }
        }
    }

    // Sort by size descending
    results.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
    results
}
