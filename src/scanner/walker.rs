use anyhow::Result;
use jwalk::{WalkDir, Parallelism};
use rayon::prelude::*;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::collections::HashSet;
use std::time::{SystemTime, Duration};

use super::targets::{FileEntry, ScanItem, ScanTarget};
use crate::common::permissions;
use crate::ffi::CANCEL_FLAG;

/// Walk a scan target and collect file information
pub fn walk_target(target: &ScanTarget) -> Result<ScanItem> {
    let total_size = AtomicU64::new(0);
    let total_count = AtomicUsize::new(0);
    let files = Arc::new(Mutex::new(Vec::new()));
    let visited_inodes = Arc::new(Mutex::new(HashSet::new()));

    let expanded_paths = expand_paths(&target.paths);

    // Process each expanded path
    expanded_paths.par_iter().for_each(|base_path| {
        if !base_path.exists() {
            return;
        }

        if permissions::is_sip_protected(base_path) {
            return;
        }

        let mut walker = WalkDir::new(base_path)
            .follow_links(false)
            .sort(false)
            .parallelism(Parallelism::RayonDefaultPool { busy_timeout: Duration::from_secs(1) });

        if !target.recursive {
            walker = walker.max_depth(1);
        }

        for entry_res in walker {
            if CANCEL_FLAG.load(Ordering::Relaxed) {
                break;
            }

            let entry = match entry_res {
                Ok(e) => e,
                Err(e) => {
                    if let Some(io_err) = e.io_error() {
                        if io_err.kind() == std::io::ErrorKind::PermissionDenied {
                            tracing::warn!("Permission denied accessing: {:?}", e);
                            continue;
                        }
                    }
                    tracing::error!("Scanner error: {:?}", e);
                    continue;
                }
            };

            if entry.file_type.is_file() {
                let path = entry.path();
                
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

                let metadata = match entry.metadata() {
                    Ok(m) => m,
                    Err(e) => {
                        tracing::debug!("Failed to get metadata for {:?}: {}", path, e);
                        continue;
                    }
                };

                // Symlink loop detection (inode check)
                let inode = metadata.ino();
                {
                    let mut visited = visited_inodes.lock().unwrap();
                    if !visited.insert(inode) {
                        tracing::trace!("Already visited inode {}, skipping {:?}", inode, path);
                        continue; 
                    }
                }

                let size = metadata.blocks() * 512;
                let modified = metadata.modified().ok();

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
    let stale_threshold = Duration::from_secs(stale_days as u64 * 86400);

    search_roots.par_iter().for_each(|root| {
        if !root.exists() {
            return;
        }

        let walker = WalkDir::new(root)
            .follow_links(false)
            .parallelism(Parallelism::RayonDefaultPool { busy_timeout: Duration::from_secs(1) });

        for entry_res in walker {
            if CANCEL_FLAG.load(Ordering::Relaxed) {
                break;
            }
            let entry = match entry_res {
                Ok(e) => e,
                Err(_) => continue,
            };

            let name = entry.file_name.to_string_lossy();
            if (name == "node_modules" && entry.depth > 0)
                || name.starts_with('.')
                || name == "Library"
            {
                continue;
            }

            let path = entry.path();
            if name == "node_modules" && entry.file_type.is_dir() {
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
                    results.lock().unwrap().push(FileEntry {
                        path: path.to_path_buf(),
                        size_bytes: dir_size(&path),
                        modified: std::fs::metadata(&path).ok().and_then(|m| m.modified().ok()),
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
    let stale_threshold = Duration::from_secs(stale_days as u64 * 86400);
    let venv_names = [".venv", "venv", ".env", "env"];

    search_roots.par_iter().for_each(|root| {
        if !root.exists() {
            return;
        }

        let walker = WalkDir::new(root)
            .follow_links(false)
            .max_depth(3)
            .parallelism(Parallelism::RayonDefaultPool { busy_timeout: Duration::from_secs(1) });

        for entry_res in walker {
            if CANCEL_FLAG.load(Ordering::Relaxed) {
                break;
            }
            let entry = match entry_res {
                Ok(e) => e,
                Err(_) => continue,
            };

            let name = entry.file_name.to_string_lossy();
            if entry.file_type.is_dir() && venv_names.contains(&name.as_ref()) {
                let path = entry.path();
                let is_venv = path.join("pyvenv.cfg").exists()
                    || path.join("bin/python").exists()
                    || path.join("Scripts/python.exe").exists();

                if is_venv {
                    let is_stale = std::fs::metadata(&path)
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
                        results.lock().unwrap().push(FileEntry {
                            path: path.to_path_buf(),
                            size_bytes: dir_size(&path),
                            modified: std::fs::metadata(&path).ok().and_then(|m| m.modified().ok()),
                        });
                    }
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
        .sort(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type.is_file())
        .map(|e| e.metadata().map(|m| m.blocks() * 512).unwrap_or(0))
        .sum()
}

/// Find large files in a directory
pub fn find_large_files(root: &Path, threshold_bytes: u64) -> Vec<FileEntry> {
    let mut results = Vec::new();
    if !root.exists() {
        return results;
    }

    let walker = WalkDir::new(root)
        .follow_links(false)
        .parallelism(Parallelism::RayonDefaultPool { busy_timeout: Duration::from_secs(1) });

    for entry_res in walker {
        if CANCEL_FLAG.load(Ordering::Relaxed) {
            break;
        }
        let entry = match entry_res {
            Ok(e) => e,
            Err(_) => continue,
        };

        let name = entry.file_name.to_string_lossy();
        if (name.starts_with('.') || name == "node_modules" || name == "Library") && entry.file_type.is_dir() {
            continue;
        }

        if entry.file_type.is_file() {
            if let Ok(metadata) = entry.metadata() {
                let size = metadata.blocks() * 512;
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

    results.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_paths() {
        let paths = vec!["~/Desktop".to_string(), "/tmp/*".to_string()];
        let expanded = expand_paths(&paths);
        assert!(!expanded.is_empty());
        assert!(expanded.iter().any(|p| p.to_string_lossy().contains("Desktop")));
    }
}
