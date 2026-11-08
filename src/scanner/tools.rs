//! Cleanups that must go through the owning tool.
//!
//! * Simulators: deleting folders under `CoreSimulator/Devices` by hand leaves
//!   `device_set.plist` pointing at missing devices. `simctl` keeps it consistent.
//! * Docker: images and volumes live inside Docker Desktop's VM disk
//!   (`Docker.raw`). Only `docker … prune` can remove them selectively.
//! * Go: the module cache is read-only on purpose; `go clean -modcache` is the
//!   supported way to empty it.
//!
//! These actions are not reversible, so they are never part of a soft-delete
//! session's undo and are reported separately.

use anyhow::{Context, Result};
use std::path::PathBuf;

use super::targets::{FileEntry, ScanItem, ToolAction};
use super::walker;
use crate::common::sys::{find_executable, tool_command};

/// Fill in size and entries for a tool-backed scan item.
/// Returns `false` if the tool isn't available (item should be dropped).
pub fn estimate(item: &mut ScanItem) -> bool {
    let action = match item.tool_action.clone() {
        Some(a) => a,
        None => return true,
    };
    match action {
        ToolAction::DockerPrune => {
            if !super::docker::is_docker_installed() || !super::docker::is_docker_running() {
                return false;
            }
            let usage = super::docker::get_docker_usage();
            item.size_bytes = usage.reclaimable;
            item.reclaimable_bytes = usage.reclaimable;
            item.file_count = usage.containers.count + usage.images.count;
            true
        }
        ToolAction::SimctlDeleteUnavailable => {
            if find_executable("xcrun").is_none() {
                return false;
            }
            let devices = unavailable_simulators();
            item.files = devices
                .into_iter()
                .map(|p| {
                    let stats = walker::dir_stats(&p);
                    item.reclaimable_bytes += stats.reclaimable;
                    FileEntry {
                        path: p,
                        size_bytes: stats.size,
                        modified: stats.newest,
                    }
                })
                .collect();
            item.size_bytes = item.files.iter().map(|f| f.size_bytes).sum();
            item.file_count = item.files.len();
            item.size_bytes > 0
        }
        ToolAction::SimctlDeletePreviews | ToolAction::GoCleanModcache => {
            let tool = if action == ToolAction::GoCleanModcache { "go" } else { "xcrun" };
            if find_executable(tool).is_none() {
                return false;
            }
            let roots: Vec<PathBuf> = if action == ToolAction::GoCleanModcache {
                go_modcache().map(|p| vec![p]).unwrap_or_else(|| item.roots.clone())
            } else {
                item.roots.clone()
            };
            let mut size = 0;
            let mut reclaim = 0;
            for r in &roots {
                let stats = walker::dir_stats(r);
                size += stats.size;
                reclaim += stats.reclaimable;
            }
            item.roots = roots;
            item.size_bytes = size;
            item.reclaimable_bytes = reclaim;
            item.file_count = 1;
            size > 0
        }
    }
}

/// Data folders of simulators whose runtime is no longer installed.
pub fn unavailable_simulators() -> Vec<PathBuf> {
    let output = match tool_command("xcrun")
        .args(["simctl", "list", "devices", "unavailable", "-j"])
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return Vec::new(),
    };
    let json: serde_json::Value = match serde_json::from_slice(&output.stdout) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let mut paths = Vec::new();
    if let Some(runtimes) = json.get("devices").and_then(|d| d.as_object()) {
        for devices in runtimes.values() {
            for dev in devices.as_array().into_iter().flatten() {
                // dataPath is …/Devices/<UDID>/data; the device folder is its parent
                if let Some(data) = dev.get("dataPath").and_then(|v| v.as_str()) {
                    let p = PathBuf::from(data);
                    let device_dir = p.parent().map(|x| x.to_path_buf()).unwrap_or(p);
                    if device_dir.exists() {
                        paths.push(device_dir);
                    }
                }
            }
        }
    }
    paths
}

fn go_modcache() -> Option<PathBuf> {
    let out = tool_command("go").args(["env", "GOMODCACHE"]).output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(PathBuf::from(s))
    }
}

fn run(cmd: &mut std::process::Command, label: &str) -> Result<String> {
    let out = cmd.output().with_context(|| format!("Failed to run {}", label))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        anyhow::bail!("{} failed: {}", label, String::from_utf8_lossy(&out.stderr).trim())
    }
}

/// Execute a tool action. Returns a short human-readable summary.
pub fn execute(action: &ToolAction) -> Result<String> {
    match action {
        ToolAction::DockerPrune => {
            let report = super::docker::prune_dangling(false)?;
            if report.errors.is_empty() {
                Ok(format!(
                    "docker prune: {} containers, {} images, {} volumes removed",
                    report.containers_removed, report.images_removed, report.volumes_removed
                ))
            } else {
                anyhow::bail!("docker prune: {}", report.errors.join("; "))
            }
        }
        ToolAction::SimctlDeleteUnavailable => {
            run(
                tool_command("xcrun").args(["simctl", "delete", "unavailable"]),
                "xcrun simctl delete unavailable",
            )?;
            Ok("Deleted unavailable simulators".into())
        }
        ToolAction::SimctlDeletePreviews => {
            run(
                tool_command("xcrun").args(["simctl", "--set", "previews", "delete", "all"]),
                "xcrun simctl --set previews delete all",
            )?;
            Ok("Deleted SwiftUI preview simulators".into())
        }
        ToolAction::GoCleanModcache => {
            run(tool_command("go").args(["clean", "-modcache"]), "go clean -modcache")?;
            Ok("Emptied the Go module cache".into())
        }
    }
}
