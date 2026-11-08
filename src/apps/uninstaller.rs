//! Uninstalling apps and cleaning their leftovers — through the unified deleter.

use anyhow::Result;
use std::path::PathBuf;

use super::detector::{AssociatedFile, InstalledApp};
use crate::cleaner::{self, CleanMode, DeleteOptions, DeleteRequest, SkippedEntry};
use crate::common::running::RunningApps;
use crate::common::safety::DeletePolicy;

/// Result of an app uninstall operation
#[derive(Debug, Default)]
pub struct UninstallReport {
    pub app_name: String,
    pub mode: UninstallMode,
    pub files_removed: usize,
    /// Bytes freed now (permanent delete) — 0 for soft delete.
    pub bytes_freed: u64,
    /// Bytes moved to staging (soft delete).
    pub bytes_staged: u64,
    pub removed_paths: Vec<PathBuf>,
    pub skipped: Vec<SkippedEntry>,
    pub session_id: Option<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum UninstallMode {
    #[default]
    DryRun,
    Remove,
}

/// What to remove.
#[derive(Debug, Clone)]
pub struct UninstallOptions {
    pub mode: CleanMode,
    /// Remove the `.app` bundle too (false = leftovers only).
    pub remove_bundle: bool,
    /// Only regenerable data (caches, logs, saved state…). Used when the app stays installed.
    pub regenerable_only: bool,
    /// Include name-based guesses (the user confirmed them).
    pub include_name_matches: bool,
    /// Extra files (e.g. Homebrew zap paths) to include.
    pub extra: Vec<AssociatedFile>,
    /// If set, only these data paths are removed (the GUI passes the user's
    /// ticked items; an empty list means "no data, just the bundle").
    pub only_paths: Option<Vec<PathBuf>>,
}

impl Default for UninstallOptions {
    fn default() -> Self {
        Self {
            mode: CleanMode::SoftDelete,
            remove_bundle: true,
            regenerable_only: false,
            include_name_matches: false,
            extra: Vec::new(),
            only_paths: None,
        }
    }
}

/// Uninstall an application and all its associated files (compat wrapper).
/// Uses soft delete, so `tidymac undo` can bring the app back.
pub fn uninstall_app(app: &InstalledApp, dry_run: bool) -> Result<UninstallReport> {
    uninstall_app_with(
        app,
        &UninstallOptions {
            mode: if dry_run { CleanMode::DryRun } else { CleanMode::SoftDelete },
            ..Default::default()
        },
    )
}

/// Uninstall or clean an app according to `opts`.
pub fn uninstall_app_with(app: &InstalledApp, opts: &UninstallOptions) -> Result<UninstallReport> {
    if app.is_apple() {
        anyhow::bail!("'{}' is part of macOS and can't be removed.", app.name);
    }

    let dry_run = opts.mode == CleanMode::DryRun;
    if !dry_run && opts.remove_bundle {
        let running = RunningApps::snapshot();
        let by_id = app.bundle_id.as_deref().map(|b| running.is_bundle_running(b)).unwrap_or(false);
        if by_id || running.is_app_path_running(&app.path) {
            anyhow::bail!("'{}' is running. Quit it first.", app.name);
        }
    }

    let library = dirs::home_dir().unwrap_or_default().join("Library");
    let mut requests = Vec::new();
    let mut report = UninstallReport {
        app_name: app.name.clone(),
        mode: if dry_run { UninstallMode::DryRun } else { UninstallMode::Remove },
        ..Default::default()
    };

    let candidates = app.associated_files.iter().chain(opts.extra.iter());
    for f in candidates {
        if !f.exists {
            continue;
        }
        if let Some(ref only) = opts.only_paths {
            if !only.contains(&f.path) {
                continue;
            }
        }
        if opts.regenerable_only && !f.kind.is_regenerable() {
            continue;
        }
        let explicitly_chosen = opts.only_paths.as_ref().map(|o| o.contains(&f.path)).unwrap_or(false);
        if f.needs_confirmation() && !opts.include_name_matches && !explicitly_chosen {
            report.skipped.push(SkippedEntry {
                path: f.path.clone(),
                reason: "matched by app name only — confirm it to include".into(),
            });
            continue;
        }
        requests.push(DeleteRequest {
            path: f.path.clone(),
            size_bytes: f.size,
            category: format!("App leftover: {}", f.kind),
            safety: "Caution".into(),
            roots: vec![library.clone()],
            policy: DeletePolicy::AppData,
        });
    }

    if opts.remove_bundle {
        requests.push(DeleteRequest {
            path: app.path.clone(),
            size_bytes: app.app_size,
            category: "Application".into(),
            safety: "Caution".into(),
            roots: Vec::new(),
            policy: DeletePolicy::AppBundle,
        });
    }

    let label = if opts.remove_bundle {
        format!("uninstall:{}", app.name)
    } else {
        format!("leftovers:{}", app.name)
    };
    let mut del_opts = DeleteOptions::new(opts.mode.clone(), &label);
    // When uninstalling we already checked the app itself; its helper agents
    // may still be loaded, so don't block on them. When only cleaning caches of
    // an installed app, skip anything the running app owns.
    del_opts.skip_in_use = !opts.remove_bundle;

    let result = cleaner::execute(requests, Vec::new(), &del_opts)?;
    report.files_removed = result.files_removed;
    report.bytes_freed = result.bytes_freed;
    report.bytes_staged = result.bytes_staged;
    report.removed_paths = result.removed_paths;
    report.skipped.extend(result.skipped);
    report.session_id = result.session_id;
    report.errors = result.errors;
    Ok(report)
}

/// Problem resolving an app from a user-supplied identifier.
#[derive(Debug)]
pub enum AppLookup<'a> {
    Found(&'a InstalledApp),
    NotFound(Vec<&'a InstalledApp>),
    Ambiguous(Vec<&'a InstalledApp>),
}

/// Resolve an app exactly: bundle id, then bundle path, then exact name
/// (case-insensitive, must be unique). Substring matches are only returned as
/// suggestions, never acted on.
pub fn resolve_app<'a>(apps: &'a [InstalledApp], query: &str) -> AppLookup<'a> {
    let q = query.trim();
    let ql = q.to_lowercase();

    if let Some(a) = apps
        .iter()
        .find(|a| a.bundle_id.as_deref().map(|b| b.eq_ignore_ascii_case(q)).unwrap_or(false))
    {
        return AppLookup::Found(a);
    }
    if let Some(a) = apps.iter().find(|a| a.path.to_string_lossy().eq_ignore_ascii_case(q)) {
        return AppLookup::Found(a);
    }
    let exact: Vec<&InstalledApp> = apps
        .iter()
        .filter(|a| a.name.to_lowercase() == ql || format!("{}.app", a.name.to_lowercase()) == ql)
        .collect();
    match exact.len() {
        1 => AppLookup::Found(exact[0]),
        0 => AppLookup::NotFound(find_app_by_name(apps, q)),
        _ => AppLookup::Ambiguous(exact),
    }
}

/// Find apps by name: exact (case-insensitive) matches if any, otherwise
/// substring matches. For display and suggestions only — use [`resolve_app`]
/// before acting.
pub fn find_app_by_name<'a>(apps: &'a [InstalledApp], name: &str) -> Vec<&'a InstalledApp> {
    let lower = name.to_lowercase();
    let exact: Vec<&InstalledApp> = apps.iter().filter(|a| a.name.to_lowercase() == lower).collect();
    if !exact.is_empty() {
        return exact;
    }
    apps.iter().filter(|a| a.name.to_lowercase().contains(&lower)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apps::detector::AppSource;

    fn app(name: &str, id: &str) -> InstalledApp {
        InstalledApp {
            name: name.into(),
            bundle_id: Some(id.into()),
            version: None,
            executable: None,
            path: PathBuf::from(format!("/Applications/{}.app", name)),
            app_size: 0,
            associated_files: Vec::new(),
            total_size: 0,
            last_opened: None,
            source: AppSource::Applications,
        }
    }

    #[test]
    fn resolve_is_exact() {
        let apps = vec![app("Xcode", "com.apple.dt.Xcode"), app("Visual Studio Code", "com.microsoft.VSCode")];
        // "Code" is a substring of both — must not resolve to either.
        assert!(matches!(resolve_app(&apps, "Code"), AppLookup::NotFound(_)));
        assert!(matches!(resolve_app(&apps, "com.microsoft.VSCode"), AppLookup::Found(a) if a.name == "Visual Studio Code"));
        assert!(matches!(resolve_app(&apps, "xcode"), AppLookup::Found(a) if a.name == "Xcode"));
    }
}
