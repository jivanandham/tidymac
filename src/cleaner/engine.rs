//! Turn scan results into deletion requests for the unified deleter.

use anyhow::Result;

use super::deleter::{self, DeleteOptions, DeleteRequest};
pub use super::deleter::{CleanMode, CleanReport};
use super::manifest::{CleanManifest, SkippedEntry};
use crate::common::config::Config;
use crate::common::format;
use crate::scanner::targets::{ScanItem, ToolAction};

/// Extra knobs for [`clean_with`].
#[derive(Debug, Clone, Default)]
pub struct CleanOptions {
    pub show_progress: bool,
    /// Allow operations above the hard item limit.
    pub force: bool,
    /// Paths to leave alone even though their item is selected.
    pub exclude_paths: Vec<std::path::PathBuf>,
}

/// Execute a cleaning operation on the given scan items.
///
/// - Only the entries listed in each item are touched — never the item's
///   root folder.
/// - Items rebuilt from the scan cache (no entry list) and review-only items
///   (large files) are skipped.
/// - Tool-backed items run their tool instead.
pub fn clean(items: &[ScanItem], mode: CleanMode, profile_name: &str, show_progress: bool) -> Result<CleanReport> {
    clean_with(
        items,
        mode,
        profile_name,
        &CleanOptions {
            show_progress,
            ..Default::default()
        },
    )
}

/// [`clean`] with options.
pub fn clean_with(items: &[ScanItem], mode: CleanMode, profile_name: &str, opts: &CleanOptions) -> Result<CleanReport> {
    let (requests, tools, pre_skipped) = requests_for(items, &opts.exclude_paths);

    let mut options = DeleteOptions::new(mode, profile_name);
    options.show_progress = opts.show_progress;
    options.force = opts.force;

    let mut report = deleter::execute(requests, tools, &options)?;
    let mut all_skipped = pre_skipped;
    all_skipped.append(&mut report.skipped);
    report.skipped = all_skipped;
    Ok(report)
}

/// Convert scan items into deletion requests and tool actions.
pub fn requests_for(
    items: &[ScanItem],
    exclude_paths: &[std::path::PathBuf],
) -> (Vec<DeleteRequest>, Vec<ToolAction>, Vec<SkippedEntry>) {
    let mut requests = Vec::new();
    let mut tools: Vec<ToolAction> = Vec::new();
    let mut skipped = Vec::new();

    for item in items {
        if item.from_cache {
            skipped.push(SkippedEntry {
                path: item.path.clone(),
                reason: format!("'{}' came from the scan cache — rescan before cleaning", item.name),
            });
            continue;
        }
        if item.review_only {
            skipped.push(SkippedEntry {
                path: item.path.clone(),
                reason: format!("'{}' is review-only — remove files individually", item.name),
            });
            continue;
        }
        if let Some(ref action) = item.tool_action {
            if !tools.contains(action) {
                tools.push(action.clone());
            }
            continue;
        }
        let roots = if item.roots.is_empty() {
            vec![item.path.clone()]
        } else {
            item.roots.clone()
        };
        for entry in &item.files {
            if exclude_paths.iter().any(|x| x == &entry.path) {
                continue;
            }
            requests.push(DeleteRequest {
                path: entry.path.clone(),
                size_bytes: entry.size_bytes,
                category: item.category.to_string(),
                safety: format!("{:?}", item.safety),
                roots: roots.clone(),
                policy: item.policy,
            });
        }
    }
    (requests, tools, skipped)
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
