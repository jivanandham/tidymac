//! Installer receipts (`pkgutil`) and Homebrew cask `zap` stanzas.

use std::path::Path;

use super::detector::{AssociatedFile, AssociatedKind, MatchKind};
use crate::common::sys::{find_executable, tool_command};

/// A package receipt that installed (part of) an app.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PackageReceipt {
    pub pkg_id: String,
    pub version: Option<String>,
    /// Number of files the receipt lists.
    pub file_count: usize,
    /// Files outside your home folder need admin rights to remove; we only report them.
    pub forget_command: String,
}

/// Receipts whose payload includes this app bundle (`pkgutil --file-info`).
pub fn receipts_for(app_path: &Path) -> Vec<PackageReceipt> {
    let out = match tool_command("pkgutil")
        .arg("--file-info")
        .arg(app_path)
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return Vec::new(),
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let mut receipts = Vec::new();
    let mut current: Option<PackageReceipt> = None;
    for line in text.lines() {
        let line = line.trim();
        if let Some(id) = line.strip_prefix("pkgid:") {
            if let Some(r) = current.take() {
                receipts.push(r);
            }
            let id = id.trim().to_string();
            current = Some(PackageReceipt {
                file_count: pkg_file_count(&id),
                forget_command: format!("sudo pkgutil --forget {}", id),
                pkg_id: id,
                version: None,
            });
        } else if let Some(v) = line.strip_prefix("pkg-version:") {
            if let Some(ref mut r) = current {
                r.version = Some(v.trim().to_string());
            }
        }
    }
    if let Some(r) = current {
        receipts.push(r);
    }
    receipts
}

fn pkg_file_count(pkg_id: &str) -> usize {
    tool_command("pkgutil")
        .args(["--files", pkg_id])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().count())
        .unwrap_or(0)
}

/// If the app was installed with a Homebrew cask, return the cask token and
/// the existing files listed in its `zap` stanza (limited to ~/Library).
pub fn brew_zap_files(app_path: &Path) -> Option<(String, Vec<AssociatedFile>)> {
    find_executable("brew")?;
    let app_file = app_path.file_name()?.to_string_lossy().to_string();
    let out = tool_command("brew")
        .args(["info", "--cask", "--json=v2", "--installed"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    let casks = json.get("casks")?.as_array()?;

    for cask in casks {
        let artifacts = match cask.get("artifacts").and_then(|a| a.as_array()) {
            Some(a) => a,
            None => continue,
        };
        let installs_app = artifacts.iter().any(|a| {
            a.get("app")
                .and_then(|v| v.as_array())
                .map(|apps| apps.iter().any(|x| x.as_str() == Some(app_file.as_str())))
                .unwrap_or(false)
        });
        if !installs_app {
            continue;
        }
        let token = cask.get("token").and_then(|t| t.as_str()).unwrap_or("").to_string();
        let mut paths: Vec<String> = Vec::new();
        for a in artifacts {
            if let Some(zaps) = a.get("zap").and_then(|z| z.as_array()) {
                for z in zaps {
                    for key in ["trash", "delete", "rmdir"] {
                        match z.get(key) {
                            Some(serde_json::Value::String(s)) => paths.push(s.clone()),
                            Some(serde_json::Value::Array(arr)) => {
                                paths.extend(arr.iter().filter_map(|x| x.as_str().map(String::from)))
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        let library = dirs::home_dir()?.join("Library");
        let mut files = Vec::new();
        for p in crate::scanner::walker::expand_paths(&paths) {
            if !p.starts_with(&library) {
                continue; // system paths need admin rights; not handled here
            }
            let kind = AssociatedKind::from_path(&p);
            super::detector::check_associated(&p, kind, MatchKind::BrewZap, &mut files);
        }
        return Some((token, files));
    }
    None
}

/// Everything known about how an app was installed.
#[derive(Debug, Clone, Default)]
pub struct InstallInfo {
    pub receipts: Vec<PackageReceipt>,
    pub brew_cask: Option<String>,
    pub brew_zap: Vec<AssociatedFile>,
}

pub fn install_info(app_path: &Path) -> InstallInfo {
    let receipts = receipts_for(app_path);
    let (brew_cask, brew_zap) = match brew_zap_files(app_path) {
        Some((t, f)) => (Some(t), f),
        None => (None, Vec::new()),
    };
    InstallInfo {
        receipts,
        brew_cask,
        brew_zap,
    }
}
