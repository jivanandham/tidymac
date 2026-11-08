//! "System Data" explainer.
//!
//! macOS lumps many things into System Data (and since macOS 27, purgeable
//! space too). This module measures the parts we can see and says, for each,
//! whether and how it can be reduced.

use serde::Serialize;
use std::path::{Path, PathBuf};

use crate::scanner::walker;

#[derive(Debug, Clone, Serialize)]
pub struct SystemDataEntry {
    pub name: String,
    pub description: String,
    /// Measured size in bytes (`None` if macOS doesn't let us measure it).
    pub size: Option<u64>,
    pub path: Option<PathBuf>,
    /// Can TidyMac reduce this (via a scan profile)?
    pub tidymac_can_clean: bool,
    /// What the user can do about it.
    pub advice: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemDataReport {
    pub entries: Vec<SystemDataEntry>,
    pub local_snapshots: Vec<String>,
    pub measured_total: u64,
    pub notes: Vec<String>,
}

fn measured(path: &Path) -> Option<u64> {
    if !path.exists() {
        return None;
    }
    match std::fs::read_dir(path) {
        Ok(_) => Some(walker::dir_size(path)),
        Err(_) => None, // no permission (Full Disk Access)
    }
}

fn swap_bytes() -> Option<u64> {
    let out = std::process::Command::new("/usr/sbin/sysctl")
        .args(["-n", "vm.swapusage"])
        .output()
        .ok()?;
    // "total = 2048.00M  used = 1024.50M  free = 1023.50M  (encrypted)"
    let text = String::from_utf8_lossy(&out.stdout);
    let total = text.split("total =").nth(1)?.split_whitespace().next()?;
    parse_size_suffix(total)
}

fn parse_size_suffix(s: &str) -> Option<u64> {
    let (num, mult) = match s.chars().last()? {
        'K' => (&s[..s.len() - 1], 1024f64),
        'M' => (&s[..s.len() - 1], 1024f64 * 1024.0),
        'G' => (&s[..s.len() - 1], 1024f64 * 1024.0 * 1024.0),
        _ => (s, 1f64),
    };
    num.parse::<f64>().ok().map(|n| (n * mult) as u64)
}

/// Local Time Machine snapshot names on the startup volume.
pub fn local_snapshots() -> Vec<String> {
    let out = match std::process::Command::new("/usr/bin/tmutil")
        .args(["listlocalsnapshots", "/"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| l.starts_with("com.apple."))
        .collect()
}

/// Build the System Data report.
pub fn explain() -> SystemDataReport {
    let home = dirs::home_dir().unwrap_or_default();
    let lib = home.join("Library");
    let snapshots = local_snapshots();
    let mut entries = Vec::new();

    let mut push = |name: &str, desc: &str, path: Option<PathBuf>, size: Option<u64>, clean: bool, advice: &str| {
        entries.push(SystemDataEntry {
            name: name.into(),
            description: desc.into(),
            size,
            path,
            tidymac_can_clean: clean,
            advice: advice.into(),
        });
    };

    push(
        "Local Time Machine snapshots",
        &format!("{} snapshot(s) on the startup disk. Their space is shown as purgeable/System Data.", snapshots.len()),
        None,
        None,
        false,
        "macOS deletes them automatically when space is needed (and after 24 h). To free space now: \
         `tmutil thinlocalsnapshots / 99999999999 4`.",
    );
    push(
        "Swap files",
        "Virtual memory paged to disk.",
        Some(PathBuf::from("/private/var/vm")),
        swap_bytes(),
        false,
        "Shrinks after quitting memory-heavy apps; reset by restarting.",
    );
    push(
        "Downloaded system assets",
        "On-device models and resources macOS downloads: Apple Intelligence (up to ~14 GB on supported Macs), Siri voices, dictation, fonts.",
        Some(PathBuf::from("/System/Library/AssetsV2")),
        measured(Path::new("/System/Library/AssetsV2")),
        false,
        "Managed by macOS. Turning off Apple Intelligence or removing voices in System Settings frees part of it.",
    );
    push(
        "App caches",
        "Caches in ~/Library/Caches.",
        Some(lib.join("Caches")),
        measured(&lib.join("Caches")),
        true,
        "Run a Quick Sweep scan.",
    );
    push(
        "Logs and crash reports",
        "~/Library/Logs",
        Some(lib.join("Logs")),
        measured(&lib.join("Logs")),
        true,
        "Run a Quick Sweep scan (logs older than 7 days).",
    );
    push(
        "Developer data",
        "Xcode DerivedData, simulators, device support, archives.",
        Some(lib.join("Developer")),
        measured(&lib.join("Developer")),
        true,
        "Run the Developer profile.",
    );
    push(
        "iPhone / iPad backups",
        "Local device backups.",
        Some(lib.join("Application Support/MobileSync/Backup")),
        measured(&lib.join("Application Support/MobileSync/Backup")),
        false,
        "Manage in Finder → your device → Manage Backups. TidyMac never deletes backups.",
    );
    push(
        "App sandboxes",
        "Data of sandboxed apps (~/Library/Containers).",
        Some(lib.join("Containers")),
        measured(&lib.join("Containers")),
        false,
        "Shrinks when you uninstall apps (Applications → Uninstall).",
    );
    push(
        "Mail",
        "Mail downloads and message store.",
        Some(lib.join("Mail")),
        measured(&lib.join("Mail")),
        false,
        "Mail Downloads can be cleaned by the Creative/Deep profiles; the message store is managed by Mail.",
    );
    let staging = crate::common::config::Config::staging_dir();
    push(
        "TidyMac undo area",
        "Items you cleaned with soft delete, kept for undo.",
        Some(staging.clone()),
        measured(&staging),
        true,
        "Freed automatically after the retention period, or now with `tidymac purge --expired` / --all.",
    );

    let measured_total = entries.iter().filter_map(|e| e.size).sum();
    let mut notes = vec![
        "On macOS 27 and later, purgeable space is counted as System Data and not as available space.".to_string(),
    ];
    if entries.iter().any(|e| e.size.is_none() && e.path.is_some() && e.path.as_ref().map(|p| p.exists()).unwrap_or(false)) {
        notes.push("Some folders couldn't be measured — grant TidyMac Full Disk Access to see them.".to_string());
    }

    SystemDataReport {
        entries,
        local_snapshots: snapshots,
        measured_total,
        notes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_swap_sizes() {
        assert_eq!(parse_size_suffix("1024.00M"), Some(1024 * 1024 * 1024));
        assert_eq!(parse_size_suffix("2.00G"), Some(2 * 1024 * 1024 * 1024));
        assert_eq!(parse_size_suffix("0.00M"), Some(0));
    }

    #[test]
    fn explain_does_not_panic() {
        let r = explain();
        assert!(!r.entries.is_empty());
    }
}
