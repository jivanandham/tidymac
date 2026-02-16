pub mod cache;
pub mod dev_detector;
pub mod docker;
pub mod targets;
pub mod walker;

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Instant;

use cache::ScanCache;
use targets::{ScanResults, ScanTarget};

/// Main scan orchestrator - runs all targets and collects results
pub fn run_scan(
    targets: &[ScanTarget],
    show_progress: bool,
    include_dev_projects: bool,
    stale_days: u32,
    large_file_threshold: u64,
) -> Result<ScanResults> {
    run_scan_with_cache(targets, show_progress, include_dev_projects, stale_days, large_file_threshold, true, "quick")
}

/// Main scan with cache control
pub fn run_scan_with_cache(
    targets: &[ScanTarget],
    show_progress: bool,
    include_dev_projects: bool,
    stale_days: u32,
    large_file_threshold: u64,
    use_cache: bool,
    profile_name: &str,
) -> Result<ScanResults> {
    let start = Instant::now();
    let mut results = ScanResults::new();

    // Load or create cache
    let mut scan_cache = if use_cache {
        ScanCache::load(profile_name).unwrap_or_else(|| ScanCache::new(profile_name))
    } else {
        ScanCache::new(profile_name)
    };

    // Set up progress bar
    let total_steps = targets.len()
        + if include_dev_projects { 2 } else { 0 }
        + 1;

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

    // 1. Scan all defined targets (with cache)
    if let Some(ref pb) = pb {
        pb.set_message("Scanning system locations...");
    }

    if use_cache {
        // Check cache for each target individually
        let mut uncached_targets = Vec::new();

        for target in targets {
            let expanded = walker::expand_paths(&target.paths);
            let mut all_cached = true;
            let mut cached_items = Vec::new();

            for path in &expanded {
                if let Some(cached) = scan_cache.check(path) {
                    // Store cached result temporarily
                    if cached.size_bytes > 0 {
                        cached_items.push(targets::ScanItem {
                            name: cached.name.clone(),
                            category: target.category.clone(),
                            path: cached.path.clone(),
                            size_bytes: cached.size_bytes,
                            file_count: cached.file_count,
                            safety: target.safety.clone(),
                            reason: cached.reason.clone(),
                            files: Vec::new(), // Cached results don't store individual files
                        });
                    }
                } else {
                    all_cached = false;
                }
            }

            if all_cached {
                // Only add cached results if target is fully cached
                results.items.extend(cached_items);
            } else {
                uncached_targets.push(target.clone());
            }
        }

        // Only scan uncached targets
        if !uncached_targets.is_empty() {
            let scan_results = walker::walk_targets(&uncached_targets);
            for result in scan_results {
                match result {
                    Ok(item) => {
                        scan_cache.store(&item);
                        if item.size_bytes > 0 {
                            results.items.push(item);
                        }
                    }
                    Err(e) => results.errors.push(format!("Scan error: {}", e)),
                }
            }
        }
    } else {
        // No cache — scan everything
        let scan_results = walker::walk_targets(targets);
        for result in scan_results {
            match result {
                Ok(item) => {
                    scan_cache.store(&item);
                    if item.size_bytes > 0 {
                        results.items.push(item);
                    }
                }
                Err(e) => results.errors.push(format!("Scan error: {}", e)),
            }
        }
    }

    if let Some(ref pb) = pb {
        pb.set_position(targets.len() as u64);
    }

    // 2. Scan for stale node_modules and venvs
    if include_dev_projects {
        if let Some(ref pb) = pb {
            pb.set_message("Scanning for stale node_modules...");
        }
        let nm_item = dev_detector::scan_node_modules(stale_days);
        if nm_item.size_bytes > 0 {
            results.items.push(nm_item);
        }
        if let Some(ref pb) = pb {
            pb.inc(1);
            pb.set_message("Scanning for stale Python venvs...");
        }
        let venv_item = dev_detector::scan_venvs(stale_days);
        if venv_item.size_bytes > 0 {
            results.items.push(venv_item);
        }
        if let Some(ref pb) = pb {
            pb.inc(1);
        }
    }

    // 3. Scan for large files
    if let Some(ref pb) = pb {
        pb.set_message("Scanning for large files...");
    }
    let home = dirs::home_dir().unwrap_or_default();
    let large_files = walker::find_large_files(&home, large_file_threshold);
    if !large_files.is_empty() {
        let total_size: u64 = large_files.iter().map(|f| f.size_bytes).sum();
        let count = large_files.len();
        results.items.push(targets::ScanItem {
            name: format!(
                "Large files (>{})",
                crate::common::format::format_size(large_file_threshold)
            ),
            category: targets::Category::LargeFile,
            path: home,
            size_bytes: total_size,
            file_count: count,
            safety: targets::SafetyLevel::Caution,
            reason: "Large files that may no longer be needed".into(),
            files: large_files,
        });
    }
    if let Some(ref pb) = pb {
        pb.inc(1);
    }

    // Sort by size descending
    results.items.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

    // Calculate totals
    results.recalculate();
    results.duration_secs = start.elapsed().as_secs_f64();

    // Save cache
    if use_cache {
        if let Err(e) = scan_cache.save() {
            results.errors.push(format!("Cache save warning: {}", e));
        }

        // Add cache stats to results
        if show_progress && (scan_cache.stats.hits > 0 || scan_cache.stats.misses > 0) {
            results.errors.push(format!(
                "Cache: {} hits, {} misses, {} invalidated ({:.0}% hit rate)",
                scan_cache.stats.hits,
                scan_cache.stats.misses,
                scan_cache.stats.invalidated,
                scan_cache.stats.hit_rate()
            ));
        }
    }

    if let Some(ref pb) = pb {
        pb.finish_and_clear();
    }

    Ok(results)
}
