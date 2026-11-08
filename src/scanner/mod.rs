pub mod cache;
pub mod dev_detector;
pub mod docker;
pub mod targets;
pub mod tools;
pub mod walker;

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::time::Instant;

use cache::ScanCache;
use targets::{ScanResults, ScanTarget};

/// Main scan orchestrator (fresh scan, no cache). Results can be cleaned.
///
/// `large_file_threshold`: `Some(bytes)` to include a review-only list of large
/// files in your home folder, `None` to skip that (slow) walk entirely.
pub fn run_scan(
    targets: &[ScanTarget],
    show_progress: bool,
    include_dev_projects: bool,
    stale_days: u32,
    large_file_threshold: Option<u64>,
) -> Result<ScanResults> {
    run_scan_with_cache(
        targets,
        show_progress,
        include_dev_projects,
        stale_days,
        large_file_threshold,
        false,
        "quick",
    )
}

/// Main scan with cache control.
///
/// Items served from the cache carry only totals (no entry list) and are
/// marked `from_cache`; they are for display and are never cleaned.
pub fn run_scan_with_cache(
    targets: &[ScanTarget],
    show_progress: bool,
    include_dev_projects: bool,
    stale_days: u32,
    large_file_threshold: Option<u64>,
    use_cache: bool,
    profile_name: &str,
) -> Result<ScanResults> {
    let start = Instant::now();
    let mut results = ScanResults::new();

    let mut scan_cache = if use_cache {
        ScanCache::load(profile_name).unwrap_or_else(|| ScanCache::new(profile_name))
    } else {
        ScanCache::new(profile_name)
    };

    let total_steps = 1 + if include_dev_projects { 2 } else { 0 } + large_file_threshold.map(|_| 1).unwrap_or(0);
    let pb = if show_progress {
        let pb = ProgressBar::new(total_steps as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.cyan} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("━━░"),
        );
        Some(pb)
    } else {
        None
    };

    if let Some(ref pb) = pb {
        pb.set_message("Scanning caches, logs and developer folders...");
    }

    // Every target's roots are "claimed": another target never counts them again.
    let mut claimed: Vec<String> = Vec::new();
    for t in crate::scanner::targets::all_targets().iter().chain(targets.iter()) {
        for r in walker::expand_paths(&t.paths) {
            claimed.push(r.to_string_lossy().to_lowercase());
        }
    }
    claimed.sort();
    claimed.dedup();

    let ctx = walker::WalkContext {
        claimed_roots: claimed,
        config: crate::common::config::Config::load().ok(),
        links: walker::LinkTracker::default(),
    };

    // 1. Targets (cache hits first)
    let mut to_scan: Vec<&ScanTarget> = Vec::new();
    for target in targets {
        if use_cache && target.tool_action.is_none() {
            let expanded = walker::expand_paths(&target.paths);
            let mut cached_items = Vec::new();
            let mut all_cached = !expanded.is_empty();
            for path in &expanded {
                match scan_cache.check(path) {
                    Some(c) => {
                        if c.size_bytes > 0 {
                            cached_items.push(targets::ScanItem {
                                name: target.name.clone(),
                                category: target.category.clone(),
                                path: c.path.clone(),
                                size_bytes: c.size_bytes,
                                reclaimable_bytes: c.size_bytes,
                                file_count: c.file_count,
                                safety: target.safety.clone(),
                                reason: c.reason.clone(),
                                roots: expanded.clone(),
                                policy: target.policy,
                                from_cache: true,
                                ..Default::default()
                            });
                        }
                    }
                    None => all_cached = false,
                }
            }
            if all_cached {
                results.items.extend(cached_items);
                continue;
            }
        }
        to_scan.push(target);
    }

    let scanned: Vec<targets::ScanItem> = to_scan
        .par_iter()
        .filter_map(|t| {
            let mut item = walker::walk_target_with(t, &ctx);
            if item.tool_action.is_some() && !tools::estimate(&mut item) {
                return None;
            }
            Some(item)
        })
        .collect();

    for item in scanned {
        if use_cache && item.tool_action.is_none() {
            scan_cache.store(&item);
        }
        if item.size_bytes > 0 {
            results.items.push(item);
        }
    }
    if let Some(ref pb) = pb {
        pb.inc(1);
    }

    if walker::cancelled() {
        anyhow::bail!("Scan cancelled");
    }

    // 2. Stale node_modules and venvs
    if include_dev_projects {
        if let Some(ref pb) = pb {
            pb.set_message("Looking for stale node_modules...");
        }
        let nm_item = dev_detector::scan_node_modules(stale_days);
        if nm_item.size_bytes > 0 {
            results.items.push(nm_item);
        }
        if let Some(ref pb) = pb {
            pb.inc(1);
            pb.set_message("Looking for stale Python venvs...");
        }
        let venv_item = dev_detector::scan_venvs(stale_days);
        if venv_item.size_bytes > 0 {
            results.items.push(venv_item);
        }
        if let Some(ref pb) = pb {
            pb.inc(1);
        }
    }

    // 3. Large files — only when the profile asks for them, and review-only.
    if let Some(threshold) = large_file_threshold {
        if let Some(ref pb) = pb {
            pb.set_message("Looking for large files...");
        }
        let home = dirs::home_dir().unwrap_or_default();
        let large_files = walker::find_large_files(&home, threshold);
        if !large_files.is_empty() {
            let total_size: u64 = large_files.iter().map(|f| f.size_bytes).sum();
            results.items.push(targets::ScanItem {
                name: format!(
                    "Large files (>{})",
                    crate::common::format::format_size(threshold)
                ),
                category: targets::Category::LargeFile,
                path: home.clone(),
                size_bytes: total_size,
                reclaimable_bytes: total_size,
                file_count: large_files.len(),
                safety: targets::SafetyLevel::Caution,
                reason: "Large personal files — review and remove individually".into(),
                files: large_files,
                roots: vec![home],
                review_only: true,
                policy: crate::common::safety::DeletePolicy::UserFile,
                ..Default::default()
            });
        }
        if let Some(ref pb) = pb {
            pb.inc(1);
        }
    }

    if walker::cancelled() {
        anyhow::bail!("Scan cancelled");
    }

    results.items.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
    results.recalculate();
    results.duration_secs = start.elapsed().as_secs_f64();

    if use_cache {
        if let Err(e) = scan_cache.save() {
            results.errors.push(format!("Cache save warning: {}", e));
        }
    }

    if let Some(ref pb) = pb {
        pb.finish_and_clear();
    }

    Ok(results)
}
