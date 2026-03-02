use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use walkdir::WalkDir;

use super::hasher;
use super::perceptual::{self, MatchType, SimilarFile, SimilarGroup};

/// Configuration for duplicate scanning
#[derive(Debug, Clone)]
pub struct DupConfig {
    /// Root directory to scan
    pub root: PathBuf,
    /// Minimum file size to consider (skip tiny files)
    pub min_size: u64,
    /// Whether to include perceptual image hashing
    pub perceptual: bool,
    /// Similarity threshold for perceptual matching (0.0-1.0)
    pub threshold: f64,
    /// Show progress bars
    pub show_progress: bool,
}

/// Complete results from a duplicate scan
#[derive(Debug, Clone)]
pub struct DupResults {
    /// Exact duplicate groups (byte-identical)
    pub exact_groups: Vec<SimilarGroup>,
    /// Perceptually similar image groups
    pub similar_groups: Vec<SimilarGroup>,
    /// Total files scanned
    pub files_scanned: usize,
    /// Total duplicate groups found
    pub total_groups: usize,
    /// Total wasted bytes (across all groups)
    pub total_wasted: u64,
    /// Total duplicate file count
    pub total_duplicates: usize,
    /// Scan duration in seconds
    pub duration_secs: f64,
    /// Errors encountered
    pub errors: Vec<String>,
}

/// Run the full duplicate detection pipeline
pub fn find_duplicates(config: &DupConfig) -> Result<DupResults> {
    let start = std::time::Instant::now();
    let mut results = DupResults {
        exact_groups: Vec::new(),
        similar_groups: Vec::new(),
        files_scanned: 0,
        total_groups: 0,
        total_wasted: 0,
        total_duplicates: 0,
        duration_secs: 0.0,
        errors: Vec::new(),
    };

    // ── Step 0: Collect all files in parallel ─────────────────────────────
    let pb = make_spinner(config.show_progress, "Collecting files...");
    let all_files = collect_files_parallel(&config.root, config.min_size);
    results.files_scanned = all_files.len();
    finish_spinner(pb, &format!("Found {} files", all_files.len()));

    if all_files.is_empty() {
        results.duration_secs = start.elapsed().as_secs_f64();
        return Ok(results);
    }

    // ── Pass 1: Group by file size ────────────────────────────────────────
    let pb = make_spinner(config.show_progress, "Pass 1: Grouping by file size...");
    let size_groups = hasher::group_by_size(&all_files);
    let candidates: usize = size_groups.values().map(|v| v.len()).sum();
    finish_spinner(
        pb,
        &format!(
            "Pass 1: {} candidates in {} size groups",
            candidates,
            size_groups.len()
        ),
    );

    if size_groups.is_empty() {
        results.duration_secs = start.elapsed().as_secs_f64();
        return Ok(results);
    }

    // ── Pass 2: Quick hash (first 4KB) ────────────────────────────────────
    let pb = make_progress(
        config.show_progress,
        size_groups.len() as u64,
        "Pass 2: Quick hashing...",
    );
    let mut quick_candidates: Vec<Vec<PathBuf>> = Vec::new();

    for (_size, paths) in &size_groups {
        let quick_groups = hasher::group_by_quick_hash(paths);
        for (_hash, group) in quick_groups {
            quick_candidates.push(group);
        }
        if let Some(ref pb) = pb {
            pb.inc(1);
        }
    }
    finish_progress(
        pb,
        &format!("Pass 2: {} candidate groups", quick_candidates.len()),
    );

    if quick_candidates.is_empty() {
        results.duration_secs = start.elapsed().as_secs_f64();
        return Ok(results);
    }

    // ── Pass 3: Full SHA-256 hash ─────────────────────────────────────────
    let pb = make_progress(
        config.show_progress,
        quick_candidates.len() as u64,
        "Pass 3: Full hashing...",
    );

    for candidate_group in &quick_candidates {
        let full_groups = hasher::group_by_full_hash(candidate_group);
        for (_hash, paths) in full_groups {
            let mut members: Vec<SimilarFile> = paths
                .iter()
                .map(|p| {
                    // Use physical disk usage (st_blocks) for consistency
                    let size = {
                        #[cfg(target_os = "macos")]
                        {
                            use std::os::darwin::fs::MetadataExt;
                            std::fs::metadata(p)
                                .map(|m| m.st_blocks() * 512)
                                .unwrap_or(0)
                        }
                        #[cfg(not(target_os = "macos"))]
                        {
                            std::fs::metadata(p).map(|m| m.len()).unwrap_or(0)
                        }
                    };
                    SimilarFile {
                        path: p.clone(),
                        size_bytes: size,
                        similarity: 1.0,
                    }
                })
                .collect();

            // Sort by size descending (largest = suggested keeper)
            members.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
            let wasted: u64 = members.iter().skip(1).map(|f| f.size_bytes).sum();

            results.exact_groups.push(SimilarGroup {
                members,
                wasted_bytes: wasted,
                match_type: MatchType::Exact,
            });
        }
        if let Some(ref pb) = pb {
            pb.inc(1);
        }
    }

    // Sort exact groups by wasted space
    results
        .exact_groups
        .sort_by(|a, b| b.wasted_bytes.cmp(&a.wasted_bytes));
    finish_progress(
        pb,
        &format!(
            "Pass 3: {} exact duplicate groups",
            results.exact_groups.len()
        ),
    );

    // ── Pass 4 (optional): Perceptual image hashing ──────────────────────
    if config.perceptual {
        let pb = make_spinner(
            config.show_progress,
            "Pass 4: Scanning images for visual similarity...",
        );

        // Collect all image files (not just the ones that survived pass 1-3)
        let image_files = perceptual::filter_images(&all_files);
        finish_spinner(pb, &format!("Found {} images", image_files.len()));

        if !image_files.is_empty() {
            let pb = make_progress(
                config.show_progress,
                image_files.len() as u64,
                "Computing perceptual hashes (parallel)...",
            );

            // Compute perceptual hashes in parallel — image decoding is CPU-bound
            let errors_shared: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
            let errors_clone = Arc::clone(&errors_shared);

            let phashes: Vec<perceptual::PerceptualHash> = image_files
                .par_iter()
                .filter_map(|path| {
                    let result = perceptual::compute_perceptual_hash(path);
                    if let Some(ref pb) = pb {
                        pb.inc(1);
                    }
                    match result {
                        Ok(hash) => {
                            let size = {
                                #[cfg(target_os = "macos")]
                                {
                                    use std::os::darwin::fs::MetadataExt;
                                    std::fs::metadata(path)
                                        .map(|m| m.st_blocks() * 512)
                                        .unwrap_or(0)
                                }
                                #[cfg(not(target_os = "macos"))]
                                {
                                    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
                                }
                            };
                            Some(perceptual::PerceptualHash {
                                path: path.clone(),
                                hash,
                                size_bytes: size,
                            })
                        }
                        Err(e) => {
                            errors_clone.lock().unwrap().push(format!(
                                "Image hash failed for '{}': {}",
                                path.display(),
                                e
                            ));
                            None
                        }
                    }
                })
                .collect();

            // Drain shared errors into results
            results.errors.extend(
                Arc::try_unwrap(errors_shared)
                    .unwrap()
                    .into_inner()
                    .unwrap(),
            );

            finish_progress(pb, &format!("Hashed {} images", phashes.len()));

            // Find similar groups
            if phashes.len() >= 2 {
                let pb = make_spinner(config.show_progress, "Finding visually similar images...");
                results.similar_groups =
                    perceptual::find_similar_groups(&phashes, config.threshold);

                // Remove groups already covered by exact duplicates
                let exact_paths: std::collections::HashSet<PathBuf> = results
                    .exact_groups
                    .iter()
                    .flat_map(|g| g.members.iter().map(|m| m.path.clone()))
                    .collect();

                results
                    .similar_groups
                    .retain(|g| g.members.iter().any(|m| !exact_paths.contains(&m.path)));

                finish_spinner(
                    pb,
                    &format!(
                        "Found {} visually similar groups",
                        results.similar_groups.len()
                    ),
                );
            }
        }
    }

    // ── Compute totals ────────────────────────────────────────────────────
    results.total_groups = results.exact_groups.len() + results.similar_groups.len();
    results.total_wasted = results
        .exact_groups
        .iter()
        .map(|g| g.wasted_bytes)
        .sum::<u64>()
        + results
            .similar_groups
            .iter()
            .map(|g| g.wasted_bytes)
            .sum::<u64>();
    results.total_duplicates = results
        .exact_groups
        .iter()
        .map(|g| g.members.len() - 1)
        .sum::<usize>()
        + results
            .similar_groups
            .iter()
            .map(|g| g.members.len() - 1)
            .sum::<usize>();
    results.duration_secs = start.elapsed().as_secs_f64();

    Ok(results)
}

/// Collect all files in a directory tree in parallel, filtered by minimum size
fn collect_files_parallel(root: &PathBuf, min_size: u64) -> Vec<PathBuf> {
    // First, collect entries from the walkdir (single-threaded walk needed for
    // deterministic ordering), then filter in parallel for speed.
    let raw: Vec<PathBuf> = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            // Skip hidden dirs, node_modules, .git, Library
            !name.starts_with('.') && name != "node_modules" && name != "Library"
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_path_buf())
        .collect();

    // Parallel size-filter (metadata syscalls are cheap but many)
    raw.into_par_iter()
        .filter(|p| {
            std::fs::metadata(p)
                .map(|m| m.len() >= min_size)
                .unwrap_or(false)
        })
        .collect()
}

// ── Progress helpers ──────────────────────────────────────────────────────────

fn make_spinner(show: bool, msg: &str) -> Option<ProgressBar> {
    if show {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        pb.set_message(msg.to_string());
        Some(pb)
    } else {
        None
    }
}

fn finish_spinner(pb: Option<ProgressBar>, msg: &str) {
    if let Some(pb) = pb {
        pb.finish_with_message(msg.to_string());
    }
}

fn make_progress(show: bool, total: u64, msg: &str) -> Option<ProgressBar> {
    if show {
        let pb = ProgressBar::new(total);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.cyan} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("━━░"),
        );
        pb.set_message(msg.to_string());
        Some(pb)
    } else {
        None
    }
}

fn finish_progress(pb: Option<ProgressBar>, msg: &str) {
    if let Some(pb) = pb {
        pb.finish_with_message(msg.to_string());
    }
}
