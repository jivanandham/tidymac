use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
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

    // ── Step 0: Collect all files ─────────────────────────────────────────
    let pb = make_spinner(config.show_progress, "Collecting files...");
    let all_files = collect_files(&config.root, config.min_size);
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
    finish_spinner(pb, &format!("Pass 1: {} candidates in {} size groups", candidates, size_groups.len()));

    if size_groups.is_empty() {
        results.duration_secs = start.elapsed().as_secs_f64();
        return Ok(results);
    }

    // ── Pass 2: Quick hash (first 4KB) ────────────────────────────────────
    let pb = make_progress(config.show_progress, size_groups.len() as u64, "Pass 2: Quick hashing...");
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
    finish_progress(pb, &format!("Pass 2: {} candidate groups", quick_candidates.len()));

    if quick_candidates.is_empty() {
        results.duration_secs = start.elapsed().as_secs_f64();
        return Ok(results);
    }

    // ── Pass 3: Full SHA-256 hash ─────────────────────────────────────────
    let pb = make_progress(config.show_progress, quick_candidates.len() as u64, "Pass 3: Full hashing...");

    for candidate_group in &quick_candidates {
        let full_groups = hasher::group_by_full_hash(candidate_group);
        for (_hash, paths) in full_groups {
            let mut members: Vec<SimilarFile> = paths
                .iter()
                .map(|p| {
                    let size = std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
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
    results.exact_groups.sort_by(|a, b| b.wasted_bytes.cmp(&a.wasted_bytes));
    finish_progress(pb, &format!("Pass 3: {} exact duplicate groups", results.exact_groups.len()));

    // ── Pass 4 (optional): Perceptual image hashing ──────────────────────
    if config.perceptual {
        let pb = make_spinner(config.show_progress, "Pass 4: Scanning images for visual similarity...");

        // Collect all image files (not just the ones that survived pass 1-3)
        let image_files = perceptual::filter_images(&all_files);
        finish_spinner(pb, &format!("Found {} images", image_files.len()));

        if !image_files.is_empty() {
            let pb = make_progress(
                config.show_progress,
                image_files.len() as u64,
                "Computing perceptual hashes...",
            );

            // Compute perceptual hashes with progress
            let mut phashes = Vec::new();
            for path in &image_files {
                match perceptual::compute_perceptual_hash(path) {
                    Ok(hash) => {
                        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
                        phashes.push(perceptual::PerceptualHash {
                            path: path.clone(),
                            hash,
                            size_bytes: size,
                        });
                    }
                    Err(e) => {
                        results.errors.push(format!("Image hash failed for '{}': {}", path.display(), e));
                    }
                }
                if let Some(ref pb) = pb {
                    pb.inc(1);
                }
            }
            finish_progress(pb, &format!("Hashed {} images", phashes.len()));

            // Find similar groups
            if phashes.len() >= 2 {
                let pb = make_spinner(config.show_progress, "Finding visually similar images...");
                results.similar_groups = perceptual::find_similar_groups(&phashes, config.threshold);

                // Remove groups that are already covered by exact duplicates
                // (no need to report the same pair twice)
                let exact_paths: std::collections::HashSet<PathBuf> = results
                    .exact_groups
                    .iter()
                    .flat_map(|g| g.members.iter().map(|m| m.path.clone()))
                    .collect();

                results.similar_groups.retain(|g| {
                    // Keep if at least one member is NOT in an exact group
                    g.members.iter().any(|m| !exact_paths.contains(&m.path))
                });

                finish_spinner(
                    pb,
                    &format!("Found {} visually similar groups", results.similar_groups.len()),
                );
            }
        }
    }

    // ── Compute totals ────────────────────────────────────────────────────
    results.total_groups = results.exact_groups.len() + results.similar_groups.len();
    results.total_wasted = results.exact_groups.iter().map(|g| g.wasted_bytes).sum::<u64>()
        + results.similar_groups.iter().map(|g| g.wasted_bytes).sum::<u64>();
    results.total_duplicates = results.exact_groups.iter().map(|g| g.members.len() - 1).sum::<usize>()
        + results.similar_groups.iter().map(|g| g.members.len() - 1).sum::<usize>();
    results.duration_secs = start.elapsed().as_secs_f64();

    Ok(results)
}

/// Collect all files in a directory tree, filtered by minimum size
fn collect_files(root: &PathBuf, min_size: u64) -> Vec<PathBuf> {
    let mut files = Vec::new();

    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            // Skip hidden dirs, node_modules, .git, Library
            !name.starts_with('.') && name != "node_modules" && name != "Library"
        })
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            if let Ok(meta) = entry.metadata() {
                if meta.len() >= min_size {
                    files.push(entry.path().to_path_buf());
                }
            }
        }
    }

    files
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
