//! The single deletion engine.
//!
//! Every removal in TidyMac — profile cleans, app uninstalls, leftover and
//! orphan cleanup, privacy cleanup, duplicates and large files — goes through
//! [`execute`]. That guarantees, for every path:
//!
//! * the safety guard (`common::safety::check_deletable`) runs right before
//!   the path is touched,
//! * items belonging to a running app are skipped,
//! * soft delete stages into `~/.tidymac/staging/<session>` with a manifest and
//!   a crash-safe journal, so `tidymac undo` works,
//! * every action lands in the audit log.

use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;

use super::manifest::{CleanManifest, ManifestItem, SkippedEntry};
use super::staging::{self, CrossVolume};
use crate::common::format;
use crate::common::running::RunningApps;
use crate::common::safety::{self, DeletePolicy, ValidationResult};
use crate::scanner::targets::ToolAction;

/// Clean mode determines how files are removed
#[derive(Debug, Clone, PartialEq)]
pub enum CleanMode {
    /// Show what would be done without doing it
    DryRun,
    /// Move to staging area for recovery (default)
    SoftDelete,
    /// Permanent removal — no undo
    HardDelete,
}

impl Default for CleanMode {
    fn default() -> Self {
        CleanMode::DryRun
    }
}

impl std::fmt::Display for CleanMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CleanMode::DryRun => write!(f, "dry_run"),
            CleanMode::SoftDelete => write!(f, "soft_delete"),
            CleanMode::HardDelete => write!(f, "hard_delete"),
        }
    }
}

/// One path to remove.
#[derive(Debug, Clone)]
pub struct DeleteRequest {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub category: String,
    pub safety: String,
    /// The path must be strictly inside one of these (ignored for AppBundle/UserFile).
    pub roots: Vec<PathBuf>,
    pub policy: DeletePolicy,
}

/// Options for an [`execute`] call.
#[derive(Debug, Clone)]
pub struct DeleteOptions {
    pub mode: CleanMode,
    /// Label recorded in the manifest (profile name, "uninstall:Slack", …).
    pub label: String,
    pub retention_days: u32,
    pub show_progress: bool,
    /// Allow operations above the hard item limit.
    pub force: bool,
    /// Skip items owned by running apps (default true).
    pub skip_in_use: bool,
}

impl DeleteOptions {
    pub fn new(mode: CleanMode, label: &str) -> Self {
        let retention_days = crate::common::config::Config::load()
            .map(|c| c.staging_retention_days)
            .unwrap_or(7);
        Self {
            mode,
            label: label.to_string(),
            retention_days,
            show_progress: false,
            force: false,
            skip_in_use: true,
        }
    }
}

/// Report from a clean operation
#[derive(Debug, Clone, Default)]
pub struct CleanReport {
    pub mode: CleanMode,
    /// Entries removed or staged successfully.
    pub files_removed: usize,
    /// Bytes actually freed now (permanent delete only).
    pub bytes_freed: u64,
    /// Bytes moved to staging (freed when the session expires or is purged).
    pub bytes_staged: u64,
    pub session_id: Option<String>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub errors: Vec<String>,
    pub skipped: Vec<SkippedEntry>,
    pub warnings: Vec<String>,
    /// Tool actions that ran (not undoable).
    pub tool_actions: Vec<String>,
    pub removed_paths: Vec<PathBuf>,
}

/// Check a request; returns the normalized path or a skip reason.
fn vet(req: &DeleteRequest, running: Option<&RunningApps>) -> std::result::Result<PathBuf, String> {
    let norm = safety::check_deletable(&req.path, &req.roots, req.policy)?;
    if req.policy == DeletePolicy::UserFile {
        match std::fs::symlink_metadata(&norm) {
            Ok(m) if m.is_file() => {}
            _ => return Err("only single files can be removed this way".into()),
        }
    }
    if let Some(r) = running {
        if let Some(owner) = r.owner_running(&norm) {
            return Err(format!("in use — quit {} first", owner));
        }
    }
    Ok(norm)
}

/// Remove `requests` (and run `tools`) according to `opts`.
pub fn execute(requests: Vec<DeleteRequest>, tools: Vec<ToolAction>, opts: &DeleteOptions) -> Result<CleanReport> {
    let running = if opts.skip_in_use {
        Some(RunningApps::snapshot())
    } else {
        None
    };

    let mut report = CleanReport {
        mode: opts.mode.clone(),
        ..Default::default()
    };

    // ── 1. Vet everything up front (counts, dedupe, nesting) ──────────────
    let mut accepted: Vec<(PathBuf, DeleteRequest)> = Vec::new();
    for req in requests {
        match vet(&req, running.as_ref()) {
            Ok(norm) => accepted.push((norm, req)),
            Err(reason) => report.skipped.push(SkippedEntry {
                path: req.path.clone(),
                reason,
            }),
        }
    }
    // Shortest paths first so nested duplicates can be dropped.
    accepted.sort_by(|a, b| a.0.as_os_str().len().cmp(&b.0.as_os_str().len()));
    let mut kept: Vec<(PathBuf, DeleteRequest)> = Vec::new();
    for (norm, req) in accepted {
        let nested = kept.iter().any(|(k, _)| norm == *k || norm.starts_with(k));
        if !nested {
            kept.push((norm, req));
        }
    }

    let total_bytes: u64 = kept.iter().map(|(_, r)| r.size_bytes).sum();
    match safety::validate_clean_operation(kept.len(), total_bytes) {
        Ok(ValidationResult::Ok) => {}
        Ok(ValidationResult::Warning(w)) => report.warnings.push(w.message),
        Err(e) => {
            if !opts.force {
                anyhow::bail!(e);
            }
            report.warnings.push(e);
        }
    }

    // ── 2. Dry run ────────────────────────────────────────────────────────
    if opts.mode == CleanMode::DryRun {
        report.files_removed = kept.len();
        report.bytes_freed = total_bytes;
        report.removed_paths = kept.into_iter().map(|(p, _)| p).collect();
        report.tool_actions = tools.iter().map(|t| format!("would run: {}", t.command_line())).collect();
        return Ok(report);
    }

    crate::common::config::Config::init_dirs()?;

    let soft = opts.mode == CleanMode::SoftDelete;
    let mut manifest = CleanManifest::new(
        &opts.label,
        if soft { "soft_delete" } else { "hard_delete" },
        if soft { opts.retention_days } else { 0 },
    );
    manifest.complete = false;
    if soft && !kept.is_empty() {
        manifest.save().context("Failed to create session manifest")?;
    }

    let pb = if opts.show_progress && !kept.is_empty() {
        let pb = ProgressBar::new(kept.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template(if soft {
                    "{spinner:.cyan} [{bar:40.cyan/blue}] {pos}/{len} Staging... {msg}"
                } else {
                    "{spinner:.red} [{bar:40.red/blue}] {pos}/{len} Deleting... {msg}"
                })
                .unwrap()
                .progress_chars("━━░"),
        );
        Some(pb)
    } else {
        None
    };

    // ── 3. Remove ─────────────────────────────────────────────────────────
    let files_dir = manifest.staging_files_dir();
    for (index, (_, req)) in kept.iter().enumerate() {
        if let Some(ref pb) = pb {
            pb.set_message(format::truncate(&format::format_path(&req.path), 40));
        }

        // Re-check immediately before touching the path (it may have changed).
        let norm = match vet(req, running.as_ref()) {
            Ok(n) => n,
            Err(reason) => {
                report.skipped.push(SkippedEntry { path: req.path.clone(), reason });
                if let Some(ref pb) = pb {
                    pb.inc(1);
                }
                continue;
            }
        };
        let is_dir = std::fs::symlink_metadata(&norm).map(|m| m.is_dir()).unwrap_or(false);

        let (staged_path, result) = if soft {
            let staged = staging::staged_path_for(&files_dir, index + 1, &norm);
            let r = staging::stage_single_path(&norm, &staged);
            (Some(staged), r)
        } else {
            (None, staging::remove_tree(&norm))
        };

        match result {
            Ok(()) => {
                let item = ManifestItem {
                    original_path: norm.clone(),
                    staged_path: staged_path.clone(),
                    size_bytes: req.size_bytes,
                    category: req.category.clone(),
                    safety: req.safety.clone(),
                    is_dir,
                    success: true,
                    error: None,
                };
                if soft {
                    let _ = manifest.journal_append(&item);
                }
                manifest.add_item(item);
                report.removed_paths.push(norm.clone());
                if soft {
                    report.bytes_staged += req.size_bytes;
                } else {
                    report.bytes_freed += req.size_bytes;
                }
                report.files_removed += 1;
            }
            Err(e) => {
                if e.downcast_ref::<CrossVolume>().is_some() {
                    report.skipped.push(SkippedEntry {
                        path: norm.clone(),
                        reason: e.to_string(),
                    });
                } else {
                    let msg = format!("{}: {}", format::format_path(&norm), e);
                    manifest.add_item(ManifestItem {
                        original_path: norm.clone(),
                        staged_path: None,
                        size_bytes: req.size_bytes,
                        category: req.category.clone(),
                        safety: req.safety.clone(),
                        is_dir,
                        success: false,
                        error: Some(e.to_string()),
                    });
                    manifest.add_error(msg.clone());
                    report.errors.push(msg);
                }
            }
        }

        if soft && (index + 1) % 50 == 0 {
            let _ = manifest.save();
        }
        if let Some(ref pb) = pb {
            pb.inc(1);
        }
    }
    if let Some(ref pb) = pb {
        pb.finish_and_clear();
    }

    // ── 4. Tool actions (not undoable) ────────────────────────────────────
    for tool in &tools {
        match crate::scanner::tools::execute(tool) {
            Ok(msg) => {
                manifest.tool_actions.push(tool.command_line().to_string());
                report.tool_actions.push(msg);
            }
            Err(e) => {
                let msg = format!("{}: {}", tool.command_line(), e);
                manifest.add_error(msg.clone());
                report.errors.push(msg);
            }
        }
    }

    // ── 5. Finalize ───────────────────────────────────────────────────────
    manifest.skipped = report.skipped.clone();
    manifest.complete = true;
    if soft && manifest.total_files > 0 {
        manifest.save().context("Failed to save clean manifest")?;
        report.session_id = Some(manifest.session_id.clone());
        report.expires_at = manifest.expires_at;
    } else if soft {
        // Nothing was staged: remove the empty session folder.
        let _ = staging::remove_tree(&manifest.staging_session_dir());
    }
    if manifest.total_files > 0 || !manifest.tool_actions.is_empty() || !manifest.errors.is_empty() {
        let _ = manifest.append_log();
    }

    Ok(report)
}
