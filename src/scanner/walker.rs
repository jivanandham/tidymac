//! File-system walking.
//!
//! * Pruning uses `walkdir::filter_entry`, which really stops descent into a
//!   directory (the previous jwalk-based code used `continue`, which only skips
//!   the entry itself and still walks everything below it).
//! * Walks never cross onto another volume (`same_file_system`).
//! * Hard links are counted once per (device, inode).
//! * APFS clones are counted by their private bytes (see `common::apfs`).

use anyhow::Result;
use rayon::prelude::*;
use std::collections::HashSet;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};
use walkdir::WalkDir;

use super::targets::{FileEntry, ScanItem, ScanTarget};
use crate::common::{apfs, permissions};
use crate::ffi::CANCEL_FLAG;

/// Aggregate statistics for a file or directory tree.
#[derive(Debug, Clone, Copy, Default)]
pub struct DirStats {
    /// Allocated bytes (`st_blocks * 512`), hard links counted once.
    pub size: u64,
    /// Bytes deleting this would actually free (APFS clones → private bytes).
    pub reclaimable: u64,
    /// Number of regular files.
    pub files: usize,
    /// Newest modification time of anything inside (files and folders).
    pub newest: Option<SystemTime>,
}

impl DirStats {
    fn touch(&mut self, t: Option<SystemTime>) {
        if let Some(t) = t {
            self.newest = Some(match self.newest {
                Some(cur) if cur >= t => cur,
                _ => t,
            });
        }
    }
}

/// Tracks multiply-linked files so each inode is counted once per scan.
#[derive(Debug, Default)]
pub struct LinkTracker(Mutex<HashSet<(u64, u64)>>);

impl LinkTracker {
    /// Returns true the first time this (device, inode) pair is seen.
    pub fn first_sighting(&self, dev: u64, ino: u64) -> bool {
        self.0.lock().map(|mut s| s.insert((dev, ino))).unwrap_or(true)
    }
}

/// Has the user asked to cancel the current scan?
pub fn cancelled() -> bool {
    CANCEL_FLAG.load(Ordering::Relaxed)
}

fn file_sizes(path: &Path, md: &std::fs::Metadata, links: &LinkTracker, clone_aware: bool) -> (u64, u64) {
    if md.nlink() > 1 && !links.first_sighting(md.dev(), md.ino()) {
        return (0, 0);
    }
    let size = md.blocks() * 512;
    let reclaim = if clone_aware && size > 0 {
        apfs::reclaimable_bytes(path, size)
    } else {
        size
    };
    (size, reclaim)
}

/// Statistics for a path (file or directory tree), sharing a link tracker.
pub fn dir_stats_with(path: &Path, links: &LinkTracker, clone_aware: bool) -> DirStats {
    let mut stats = DirStats::default();
    let md = match std::fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(_) => return stats,
    };
    if md.file_type().is_symlink() {
        stats.touch(md.modified().ok());
        return stats;
    }
    if md.is_file() {
        let (size, reclaim) = file_sizes(path, &md, links, clone_aware);
        stats.size = size;
        stats.reclaimable = reclaim;
        stats.files = 1;
        stats.touch(md.modified().ok());
        return stats;
    }

    for entry in WalkDir::new(path)
        .follow_links(false)
        .same_file_system(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if cancelled() {
            break;
        }
        let emd = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        stats.touch(emd.modified().ok());
        if entry.file_type().is_file() {
            let (size, reclaim) = file_sizes(entry.path(), &emd, links, clone_aware);
            stats.size += size;
            stats.reclaimable += reclaim;
            stats.files += 1;
        }
    }
    stats
}

/// Statistics for a path with a fresh link tracker, clone-aware.
pub fn dir_stats(path: &Path) -> DirStats {
    dir_stats_with(path, &LinkTracker::default(), true)
}

/// Total allocated size of a directory (physical disk usage).
pub fn dir_size(path: &Path) -> u64 {
    dir_stats_with(path, &LinkTracker::default(), false).size
}

/// Expand `~`, `{uid}`, `{user_tmp}` and glob patterns in paths.
pub fn expand_paths(paths: &[String]) -> Vec<PathBuf> {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/Users/unknown"));
    let home_str = home.to_string_lossy().to_string();
    let uid = crate::common::sys::current_uid().to_string();
    let mut expanded = Vec::new();

    for path_str in paths {
        let mut resolved = path_str.clone();
        if resolved.contains("{user_tmp}") {
            match crate::common::sys::user_temp_dir() {
                Some(t) => resolved = resolved.replace("{user_tmp}", &t.to_string_lossy()),
                None => continue,
            }
        }
        if resolved == "~" {
            resolved = home_str.clone();
        } else if let Some(rest) = resolved.strip_prefix("~/") {
            resolved = format!("{}/{}", home_str, rest);
        }
        resolved = resolved.replace("{uid}", &uid);

        if resolved.contains('*') || resolved.contains('?') || resolved.contains('[') {
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

/// Context shared by all targets in one scan.
#[derive(Default)]
pub struct WalkContext {
    /// Lower-cased roots of *other* targets. A child equal to (or containing)
    /// one of these is skipped so the same bytes are never counted twice.
    pub claimed_roots: Vec<String>,
    /// User exclusion patterns from config.
    pub config: Option<crate::common::config::Config>,
    pub links: LinkTracker,
}

impl WalkContext {
    fn is_claimed(&self, child: &Path, own_roots: &[String]) -> bool {
        let c = child.to_string_lossy().to_lowercase();
        self.claimed_roots.iter().any(|r| {
            if own_roots.contains(r) {
                return false;
            }
            c == *r || r.starts_with(&format!("{}/", c))
        })
    }

    fn is_excluded(&self, path: &Path) -> bool {
        self.config.as_ref().map(|c| c.is_excluded(path)).unwrap_or(false)
    }
}

fn name_excluded(name: &str, patterns: &[String]) -> bool {
    let lower = name.to_lowercase();
    patterns.iter().any(|p| {
        let p = p.to_lowercase();
        if let Some(prefix) = p.strip_suffix('*') {
            lower.starts_with(prefix)
        } else {
            lower == p
        }
    })
}

/// Walk a scan target with an empty context.
pub fn walk_target(target: &ScanTarget) -> Result<ScanItem> {
    Ok(walk_target_with(target, &WalkContext::default()))
}

/// Walk a scan target: every direct child of every root becomes one entry.
pub fn walk_target_with(target: &ScanTarget, ctx: &WalkContext) -> ScanItem {
    let roots = expand_paths(&target.paths);
    let mut item = ScanItem {
        name: target.name.clone(),
        category: target.category.clone(),
        path: roots.first().cloned().unwrap_or_default(),
        safety: target.safety.clone(),
        reason: target.reason.clone(),
        tool_action: target.tool_action.clone(),
        policy: target.policy,
        ..Default::default()
    };

    let own_roots: Vec<String> = roots.iter().map(|r| r.to_string_lossy().to_lowercase()).collect();
    let min_age = target.min_age_days.map(|d| Duration::from_secs(d as u64 * 86_400));
    let now = SystemTime::now();
    let mut collected: Vec<(FileEntry, u64, usize)> = Vec::new();

    for root in &roots {
        let md = match std::fs::symlink_metadata(root) {
            Ok(m) => m,
            Err(_) => continue,
        };
        if !md.is_dir() || permissions::is_sip_protected(root) {
            continue;
        }
        item.roots.push(root.clone());

        // Tool-backed targets are sized by scanner::tools, not walked here.
        if target.tool_action.is_some() {
            continue;
        }

        let children: Vec<PathBuf> = match std::fs::read_dir(root) {
            Ok(rd) => rd.filter_map(|e| e.ok()).map(|e| e.path()).collect(),
            Err(e) => {
                tracing::warn!("Cannot read {}: {}", root.display(), e);
                continue;
            }
        };

        let found: Vec<(FileEntry, u64, usize)> = children
            .par_iter()
            .filter_map(|child| {
                if cancelled() {
                    return None;
                }
                let name = child.file_name()?.to_string_lossy().to_string();
                if name == ".DS_Store" || name == ".localized" {
                    return None;
                }
                if name_excluded(&name, &target.exclude) || ctx.is_excluded(child) {
                    return None;
                }
                if ctx.is_claimed(child, &own_roots) {
                    return None;
                }
                let cmd = std::fs::symlink_metadata(child).ok()?;
                let ft = cmd.file_type();
                let stats = if ft.is_dir() {
                    if !target.recursive || !target.extensions.is_empty() {
                        return None;
                    }
                    dir_stats_with(child, &ctx.links, true)
                } else if ft.is_file() || ft.is_symlink() {
                    if !target.extensions.is_empty() {
                        let ext = child
                            .extension()
                            .map(|e| e.to_string_lossy().to_lowercase())
                            .unwrap_or_default();
                        if !target.extensions.iter().any(|x| *x == ext) {
                            return None;
                        }
                    }
                    dir_stats_with(child, &ctx.links, true)
                } else {
                    return None; // sockets, fifos, devices
                };

                if stats.size == 0 && stats.files == 0 && !ft.is_symlink() {
                    return None;
                }
                if let (Some(age), Some(newest)) = (min_age, stats.newest) {
                    if now.duration_since(newest).unwrap_or_default() < age {
                        return None;
                    }
                }
                if let Some(min) = target.min_size_bytes {
                    if stats.size < min {
                        return None;
                    }
                }
                Some((
                    FileEntry {
                        path: child.clone(),
                        size_bytes: stats.size,
                        modified: stats.newest,
                    },
                    stats.reclaimable,
                    stats.files,
                ))
            })
            .collect();
        collected.extend(found);
    }

    collected.sort_by(|a, b| b.0.size_bytes.cmp(&a.0.size_bytes));
    for (entry, reclaim, files) in collected {
        item.size_bytes += entry.size_bytes;
        item.reclaimable_bytes += reclaim;
        item.file_count += files.max(1);
        item.files.push(entry);
    }
    item
}

/// Walk multiple targets in parallel (empty context).
pub fn walk_targets(targets: &[ScanTarget]) -> Vec<Result<ScanItem>> {
    targets.par_iter().map(walk_target).collect()
}

// ─── Project artifacts ───────────────────────────────────────────────────────

/// Directory names never descended into while looking for projects or large files.
fn is_pruned_dir(name: &str) -> bool {
    let lower = name.to_lowercase();
    name.starts_with('.')
        || lower == "library"
        || lower == "node_modules"
        || lower.ends_with(".app")
        || lower.ends_with(".photoslibrary")
        || lower.ends_with(".photolibrary")
        || lower.ends_with(".musiclibrary")
        || lower.ends_with(".tvlibrary")
        || lower.ends_with(".fcpbundle")
        || lower.ends_with(".xcarchive")
}

fn newest_mtime(paths: &[PathBuf]) -> Option<SystemTime> {
    paths
        .iter()
        .filter_map(|p| std::fs::metadata(p).ok())
        .filter_map(|m| m.modified().ok())
        .max()
}

fn older_than(t: Option<SystemTime>, threshold: Duration) -> bool {
    match t {
        Some(t) => SystemTime::now().duration_since(t).unwrap_or_default() > threshold,
        None => true,
    }
}

/// Find `node_modules` folders whose project hasn't been touched in `stale_days`.
///
/// Freshness is judged from the project's manifest and lockfiles and npm/yarn/pnpm's
/// install markers inside `node_modules`, not from the folder's own mtime.
pub fn find_node_modules(search_roots: &[PathBuf], stale_days: u32) -> Vec<FileEntry> {
    let threshold = Duration::from_secs(stale_days as u64 * 86_400);
    let links = LinkTracker::default();

    let mut found: Vec<PathBuf> = Vec::new();
    for root in search_roots {
        if !root.is_dir() {
            continue;
        }
        let walker = WalkDir::new(root)
            .follow_links(false)
            .same_file_system(true)
            .max_depth(10)
            .into_iter()
            .filter_entry(|e| {
                if e.depth() == 0 {
                    return true;
                }
                if !e.file_type().is_dir() {
                    return false;
                }
                let name = e.file_name().to_string_lossy();
                if name == "node_modules" {
                    found.push(e.path().to_path_buf());
                    return false; // never descend into node_modules
                }
                !is_pruned_dir(&name)
            });
        for _ in walker {
            if cancelled() {
                break;
            }
        }
    }

    found
        .par_iter()
        .filter_map(|nm| {
            let project = nm.parent()?;
            let markers = vec![
                project.join("package.json"),
                project.join("package-lock.json"),
                project.join("yarn.lock"),
                project.join("pnpm-lock.yaml"),
                project.join("bun.lockb"),
                nm.join(".package-lock.json"),
                nm.join(".yarn-state.yml"),
                nm.join(".modules.yaml"),
            ];
            let newest = newest_mtime(&markers)
                .or_else(|| std::fs::metadata(nm).ok().and_then(|m| m.modified().ok()));
            if !older_than(newest, threshold) {
                return None;
            }
            let stats = dir_stats_with(nm, &links, true);
            if stats.size == 0 {
                return None;
            }
            Some(FileEntry {
                path: nm.clone(),
                size_bytes: stats.size,
                modified: newest,
            })
        })
        .collect()
}

/// Find Python virtual environments (folders containing `pyvenv.cfg`) whose
/// project hasn't been touched in `stale_days`.
pub fn find_venvs(search_roots: &[PathBuf], stale_days: u32) -> Vec<FileEntry> {
    let threshold = Duration::from_secs(stale_days as u64 * 86_400);
    let links = LinkTracker::default();
    let venv_names = [".venv", "venv", ".env", "env"];

    let mut found: Vec<PathBuf> = Vec::new();
    for root in search_roots {
        if !root.is_dir() {
            continue;
        }
        let walker = WalkDir::new(root)
            .follow_links(false)
            .same_file_system(true)
            .max_depth(8)
            .into_iter()
            .filter_entry(|e| {
                if e.depth() == 0 {
                    return true;
                }
                if !e.file_type().is_dir() {
                    return false;
                }
                let name = e.file_name().to_string_lossy();
                if venv_names.contains(&name.as_ref()) && e.path().join("pyvenv.cfg").is_file() {
                    found.push(e.path().to_path_buf());
                    return false;
                }
                !is_pruned_dir(&name)
            });
        for _ in walker {
            if cancelled() {
                break;
            }
        }
    }

    found
        .par_iter()
        .filter_map(|venv| {
            let project = venv.parent()?;
            let markers = vec![
                project.join("pyproject.toml"),
                project.join("requirements.txt"),
                project.join("poetry.lock"),
                project.join("Pipfile.lock"),
                project.join("uv.lock"),
                project.join("setup.py"),
                venv.join("pyvenv.cfg"),
            ];
            let newest = newest_mtime(&markers);
            if !older_than(newest, threshold) {
                return None;
            }
            let stats = dir_stats_with(venv, &links, true);
            if stats.size == 0 {
                return None;
            }
            Some(FileEntry {
                path: venv.clone(),
                size_bytes: stats.size,
                modified: newest,
            })
        })
        .collect()
}

/// Find large files under `root` (sorted largest first).
///
/// Hidden folders, `~/Library`, `node_modules`, app bundles and media
/// libraries are pruned; other volumes are not entered.
pub fn find_large_files(root: &Path, threshold_bytes: u64) -> Vec<FileEntry> {
    let mut results = Vec::new();
    if !root.is_dir() {
        return results;
    }
    let walker = WalkDir::new(root)
        .follow_links(false)
        .same_file_system(true)
        .into_iter()
        .filter_entry(|e| {
            if e.depth() == 0 || !e.file_type().is_dir() {
                return true;
            }
            !is_pruned_dir(&e.file_name().to_string_lossy())
        });

    for entry in walker.filter_map(|e| e.ok()) {
        if cancelled() {
            break;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        if let Ok(md) = entry.metadata() {
            let size = md.blocks() * 512;
            if size >= threshold_bytes {
                results.push(FileEntry {
                    path: entry.path().to_path_buf(),
                    size_bytes: size,
                    modified: md.modified().ok(),
                });
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
        let paths = vec!["~/Desktop".to_string()];
        let expanded = expand_paths(&paths);
        assert!(!expanded.is_empty());
        assert!(expanded.iter().any(|p| p.to_string_lossy().contains("Desktop")));
    }

    #[test]
    fn test_expand_uid() {
        let expanded = expand_paths(&["/nonexistent-tidymac/{uid}".to_string()]);
        assert_eq!(expanded.len(), 1);
        assert!(!expanded[0].to_string_lossy().contains("{uid}"));
    }

    #[test]
    fn test_name_excluded() {
        let pats = vec!["com.apple.*".to_string(), "TidyMac".to_string()];
        assert!(name_excluded("com.apple.Safari", &pats));
        assert!(name_excluded("tidymac", &pats));
        assert!(!name_excluded("com.google.Chrome", &pats));
    }

    #[test]
    fn test_hard_links_counted_once() {
        let dir = tempfile::TempDir::new().unwrap();
        let a = dir.path().join("a.bin");
        std::fs::write(&a, vec![1u8; 64 * 1024]).unwrap();
        std::fs::hard_link(&a, dir.path().join("b.bin")).unwrap();
        let single = std::fs::metadata(&a).unwrap().blocks() * 512;
        let stats = dir_stats(dir.path());
        assert_eq!(stats.size, single);
        assert_eq!(stats.files, 2);
    }

    #[test]
    fn test_walk_target_reports_children_not_root() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir(dir.path().join("cacheA")).unwrap();
        std::fs::write(dir.path().join("cacheA/blob"), vec![0u8; 4096]).unwrap();
        std::fs::write(dir.path().join("loose.log"), "hello").unwrap();
        let target = ScanTarget {
            name: "t".into(),
            paths: vec![dir.path().to_string_lossy().to_string()],
            recursive: true,
            ..Default::default()
        };
        let item = walk_target(&target).unwrap();
        assert_eq!(item.files.len(), 2);
        assert!(item.files.iter().all(|f| f.path != dir.path()));
        assert_eq!(item.roots, vec![dir.path().to_path_buf()]);
    }

    #[test]
    fn test_find_node_modules_detects_stale_projects() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path().join("Projects");
        let project = root.join("app");
        let nm = project.join("node_modules");
        std::fs::create_dir_all(nm.join("left-pad")).unwrap();
        std::fs::write(nm.join("left-pad/index.js"), vec![b'x'; 8192]).unwrap();
        std::fs::write(project.join("package.json"), "{}").unwrap();
        // nested node_modules must not be reported separately
        std::fs::create_dir_all(nm.join("left-pad/node_modules/inner")).unwrap();

        // stale_days = 0 → everything counts as stale
        let found = find_node_modules(&[root.clone()], 0);
        assert_eq!(found.len(), 1, "found: {:?}", found);
        assert_eq!(found[0].path, nm);

        // a fresh project (just written) is not stale at 30 days
        let fresh = find_node_modules(&[root], 30);
        assert!(fresh.is_empty());
    }

    #[test]
    fn test_find_venvs_requires_pyvenv_cfg() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path().join("code");
        std::fs::create_dir_all(root.join("p1/.venv/lib")).unwrap();
        std::fs::write(root.join("p1/.venv/pyvenv.cfg"), "home = /usr/bin").unwrap();
        std::fs::write(root.join("p1/.venv/lib/x.py"), vec![b'x'; 4096]).unwrap();
        // a dotenv FILE named .env is not a venv
        std::fs::create_dir_all(root.join("p2")).unwrap();
        std::fs::write(root.join("p2/.env"), "SECRET=1").unwrap();
        let found = find_venvs(&[root], 0);
        assert_eq!(found.len(), 1);
        assert!(found[0].path.ends_with(".venv"));
    }

    #[test]
    fn test_large_files_prunes_hidden_and_library() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path().join("home");
        std::fs::create_dir_all(root.join(".hidden")).unwrap();
        std::fs::create_dir_all(root.join("Library")).unwrap();
        std::fs::create_dir_all(root.join("visible")).unwrap();
        std::fs::write(root.join(".hidden/big"), vec![0u8; 20_000]).unwrap();
        std::fs::write(root.join("Library/big"), vec![0u8; 20_000]).unwrap();
        std::fs::write(root.join("visible/big"), vec![0u8; 20_000]).unwrap();
        let found = find_large_files(&root, 10_000);
        assert_eq!(found.len(), 1);
        assert!(found[0].path.starts_with(root.join("visible")));
    }
}
