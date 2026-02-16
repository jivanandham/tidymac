use anyhow::{Context, Result};
use std::path::PathBuf;

use super::detector::InstalledApp;

/// Result of an app uninstall operation
#[derive(Debug)]
pub struct UninstallReport {
    pub app_name: String,
    pub mode: UninstallMode,
    pub files_removed: usize,
    pub bytes_freed: u64,
    pub removed_paths: Vec<PathBuf>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum UninstallMode {
    DryRun,
    Remove,
}

/// Uninstall an application and all its associated files
pub fn uninstall_app(app: &InstalledApp, dry_run: bool) -> Result<UninstallReport> {
    let mode = if dry_run {
        UninstallMode::DryRun
    } else {
        UninstallMode::Remove
    };

    let mut report = UninstallReport {
        app_name: app.name.clone(),
        mode,
        files_removed: 0,
        bytes_freed: 0,
        removed_paths: Vec::new(),
        errors: Vec::new(),
    };

    // Check if app is currently running
    if !dry_run && is_app_running(&app.name) {
        anyhow::bail!(
            "'{}' appears to be running. Please quit it first.",
            app.name
        );
    }

    // Collect all paths to remove: associated files first, then the app itself
    let mut paths_to_remove: Vec<(PathBuf, u64)> = Vec::new();

    for assoc in &app.associated_files {
        if assoc.exists {
            paths_to_remove.push((assoc.path.clone(), assoc.size));
        }
    }

    // App bundle itself (last)
    paths_to_remove.push((app.path.clone(), app.app_size));

    // Execute removal
    for (path, size) in &paths_to_remove {
        if dry_run {
            report.removed_paths.push(path.clone());
            report.bytes_freed += size;
            report.files_removed += 1;
        } else {
            match remove_path(path) {
                Ok(()) => {
                    report.removed_paths.push(path.clone());
                    report.bytes_freed += size;
                    report.files_removed += 1;
                }
                Err(e) => {
                    report.errors.push(format!(
                        "Failed to remove '{}': {}",
                        path.display(),
                        e
                    ));
                }
            }
        }
    }

    Ok(report)
}

/// Remove a file or directory
fn remove_path(path: &PathBuf) -> Result<()> {
    if path.is_dir() {
        std::fs::remove_dir_all(path)
            .with_context(|| format!("Failed to remove dir: {}", path.display()))?;
    } else {
        std::fs::remove_file(path)
            .with_context(|| format!("Failed to remove file: {}", path.display()))?;
    }
    Ok(())
}

/// Check if an app is currently running using pgrep
fn is_app_running(app_name: &str) -> bool {
    std::process::Command::new("pgrep")
        .arg("-f")
        .arg(app_name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Find an app by name (case-insensitive partial match)
pub fn find_app_by_name<'a>(apps: &'a [InstalledApp], name: &str) -> Vec<&'a InstalledApp> {
    let lower = name.to_lowercase();
    apps.iter()
        .filter(|a| a.name.to_lowercase().contains(&lower))
        .collect()
}
