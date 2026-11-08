//! Leftovers of apps that are no longer installed.
//!
//! We look for folders/files in `~/Library` named after a bundle identifier
//! (reverse-DNS) that no installed or running app owns. To stay conservative:
//!
//! * Apple identifiers are ignored.
//! * If any installed app shares the vendor prefix (`com.microsoft.*`), the
//!   item is ignored — vendors share helpers and frameworks between apps.
//! * Items modified in the last 30 days are ignored (something still uses them).
//!
//! Results are suggestions; nothing is removed without the user selecting it.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use super::detector::{AssociatedFile, AssociatedKind, MatchKind};

/// Leftover files grouped by the bundle identifier they belong to.
#[derive(Debug, Clone)]
pub struct OrphanGroup {
    pub bundle_id: String,
    pub files: Vec<AssociatedFile>,
    pub total_size: u64,
}

/// Is this string shaped like a bundle identifier?
pub fn looks_like_bundle_id(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() < 3 {
        return false;
    }
    let tld = parts[0];
    if !(2..=6).contains(&tld.len()) || !tld.chars().all(|c| c.is_ascii_lowercase()) {
        return false;
    }
    parts
        .iter()
        .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'))
}

fn vendor(id: &str) -> String {
    id.to_lowercase().split('.').take(2).collect::<Vec<_>>().join(".")
}

/// Bundle identifiers of every app we can find (installed, Spotlight-indexed, running).
pub fn known_bundle_ids() -> HashSet<String> {
    let mut ids: HashSet<String> = HashSet::new();
    for app in super::detector::discover_apps() {
        if let Some(b) = app.bundle_id {
            ids.insert(b.to_lowercase());
        }
    }
    // Apps anywhere on disk (Spotlight), incl. ones in odd locations.
    if let Ok(out) = std::process::Command::new("/usr/bin/mdfind")
        .arg("kMDItemContentType == 'com.apple.application-bundle'")
        .output()
    {
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            if let Some(b) = super::detector::bundle_id_of(Path::new(line.trim())) {
                ids.insert(b.to_lowercase());
            }
        }
    }
    ids
}

/// Find orphaned leftovers.
pub fn find_orphans() -> Vec<OrphanGroup> {
    let installed = known_bundle_ids();
    let running = crate::common::running::RunningApps::snapshot();
    find_orphans_with(&installed, &running)
}

/// [`find_orphans`] with an explicit set of known ids (testable).
pub fn find_orphans_with(installed: &HashSet<String>, running: &crate::common::running::RunningApps) -> Vec<OrphanGroup> {
    let lib = match dirs::home_dir() {
        Some(h) => h.join("Library"),
        None => return Vec::new(),
    };
    let vendors: HashSet<String> = installed.iter().map(|i| vendor(i)).collect();
    let recent = Duration::from_secs(30 * 86_400);

    let locations: [(&str, AssociatedKind, &str); 9] = [
        ("Application Support", AssociatedKind::AppSupport, ""),
        ("Caches", AssociatedKind::Cache, ""),
        ("Containers", AssociatedKind::Container, ""),
        ("HTTPStorages", AssociatedKind::HttpStorage, ""),
        ("WebKit", AssociatedKind::WebKit, ""),
        ("Logs", AssociatedKind::Logs, ""),
        ("Saved Application State", AssociatedKind::SavedState, ".savedState"),
        ("Preferences", AssociatedKind::Preferences, ".plist"),
        ("LaunchAgents", AssociatedKind::LaunchAgent, ".plist"),
    ];

    let mut groups: BTreeMap<String, Vec<AssociatedFile>> = BTreeMap::new();

    for (rel, kind, suffix) in locations.iter() {
        let dir = lib.join(rel);
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.filter_map(|e| e.ok()) {
            let name = entry.file_name().to_string_lossy().to_string();
            let id = if suffix.is_empty() {
                name.clone()
            } else if let Some(stripped) = name.strip_suffix(suffix) {
                stripped.to_string()
            } else {
                continue;
            };
            let lower = id.to_lowercase();
            if !looks_like_bundle_id(&lower) || lower.starts_with("com.apple.") {
                continue;
            }
            if is_owned(&lower, installed) || vendors.contains(&vendor(&lower)) {
                continue;
            }
            if running.owner_running(&entry.path()).is_some() {
                continue;
            }
            let path = entry.path();
            let newest = crate::scanner::walker::dir_stats(&path).newest;
            if let Some(t) = newest {
                if SystemTime::now().duration_since(t).unwrap_or_default() < recent {
                    continue;
                }
            }
            let mut tmp = Vec::new();
            super::detector::check_associated(&path, kind.clone(), MatchKind::BundleId, &mut tmp);
            groups.entry(lower).or_default().extend(tmp);
        }
    }

    let mut result: Vec<OrphanGroup> = groups
        .into_iter()
        .map(|(bundle_id, files)| OrphanGroup {
            total_size: files.iter().map(|f| f.size).sum(),
            bundle_id,
            files,
        })
        .filter(|g| !g.files.is_empty())
        .collect();
    result.sort_by(|a, b| b.total_size.cmp(&a.total_size));
    result
}

/// An id is owned if an installed id equals it, or one is a dotted prefix of
/// the other (helpers: `com.foo.app.helper` ↔ `com.foo.app`).
fn is_owned(id: &str, installed: &HashSet<String>) -> bool {
    if installed.contains(id) {
        return true;
    }
    installed.iter().any(|i| {
        (id.len() > i.len() && id.starts_with(i.as_str()) && id.as_bytes()[i.len()] == b'.')
            || (i.len() > id.len() && i.starts_with(id) && i.as_bytes()[id.len()] == b'.')
    })
}

/// Paths of all files in the given groups (for deletion requests).
pub fn paths_of(groups: &[OrphanGroup]) -> Vec<PathBuf> {
    groups.iter().flat_map(|g| g.files.iter().map(|f| f.path.clone())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_id_shape() {
        assert!(looks_like_bundle_id("com.tinyspeck.slackmacgap"));
        assert!(looks_like_bundle_id("org.mozilla.firefox"));
        assert!(!looks_like_bundle_id("Google"));
        assert!(!looks_like_bundle_id("com.foo"));
        assert!(!looks_like_bundle_id("Some Folder.with.dots"));
    }

    #[test]
    fn ownership_by_prefix() {
        let mut installed = HashSet::new();
        installed.insert("com.docker.docker".to_string());
        assert!(is_owned("com.docker.docker.helper", &installed));
        assert!(is_owned("com.docker.docker", &installed));
        assert!(!is_owned("com.docker.dockerx", &installed));
        assert!(!is_owned("com.other.app", &installed));
    }
}
