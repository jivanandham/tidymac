//! Detect running applications so we never delete caches, browser data or app
//! files out from under a live process.
//!
//! We take one snapshot of the process table (`ps -axo comm=`), find every
//! process whose executable lives inside an `.app` bundle, and read that
//! bundle's `CFBundleIdentifier`. Matching is by exact bundle identifier or
//! exact bundle path — never by substring.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct RunningApps {
    /// Lower-cased bundle identifiers of running apps (incl. helpers).
    bundle_ids: HashSet<String>,
    /// Lower-cased bundle paths of running apps.
    bundle_paths: HashSet<String>,
    /// Lower-cased display names (CFBundleName / bundle file stem), ≥ 4 chars.
    names: HashSet<String>,
    /// Map bundle id → display name, for messages.
    labels: HashMap<String, String>,
}

impl RunningApps {
    /// An empty snapshot (nothing considered running). Useful in tests.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Take a snapshot of currently running apps.
    pub fn snapshot() -> Self {
        let mut apps = Self::default();
        let output = match std::process::Command::new("ps").args(["-axo", "comm="]).output() {
            Ok(o) => o,
            Err(_) => return apps,
        };
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut seen_bundles: HashSet<String> = HashSet::new();

        for line in stdout.lines() {
            let line = line.trim();
            // Every .app bundle on the path (outer app and nested helper apps)
            let mut search_from = 0;
            while let Some(idx) = line[search_from..].find(".app/") {
                let end = search_from + idx + 4; // include ".app"
                let bundle = &line[..end];
                if seen_bundles.insert(bundle.to_string()) {
                    apps.add_bundle(Path::new(bundle));
                }
                search_from = end;
            }
        }
        apps
    }

    fn add_bundle(&mut self, bundle: &Path) {
        self.bundle_paths
            .insert(bundle.to_string_lossy().to_lowercase());
        let stem = bundle
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        if stem.chars().count() >= 4 {
            self.names.insert(stem.to_lowercase());
        }
        let plist_path = bundle.join("Contents/Info.plist");
        if let Ok(value) = plist::Value::from_file(&plist_path) {
            if let Some(dict) = value.as_dictionary() {
                if let Some(id) = dict.get("CFBundleIdentifier").and_then(|v| v.as_string()) {
                    let lower = id.to_lowercase();
                    self.labels.insert(lower.clone(), stem.clone());
                    self.bundle_ids.insert(lower);
                }
                if let Some(name) = dict.get("CFBundleName").and_then(|v| v.as_string()) {
                    if name.chars().count() >= 4 {
                        self.names.insert(name.to_lowercase());
                    }
                }
            }
        }
    }

    /// Mark a bundle id as running (tests / callers with extra knowledge).
    pub fn insert_bundle_id(&mut self, id: &str) {
        self.bundle_ids.insert(id.to_lowercase());
    }

    /// Is an app with exactly this bundle identifier running?
    pub fn is_bundle_running(&self, bundle_id: &str) -> bool {
        self.bundle_ids.contains(&bundle_id.to_lowercase())
    }

    /// Is the app at this bundle path running?
    pub fn is_app_path_running(&self, app_path: &Path) -> bool {
        self.bundle_paths
            .contains(&app_path.to_string_lossy().to_lowercase())
    }

    /// If any path component names a running app (exact bundle id, `<bundle id>.*`
    /// such as `com.foo.app.savedState` / `com.foo.app.plist`, or exact app name),
    /// return a label for it.
    pub fn owner_running(&self, path: &Path) -> Option<String> {
        for comp in path.components() {
            let c = comp.as_os_str().to_string_lossy().to_lowercase();
            if c.is_empty() {
                continue;
            }
            if self.bundle_ids.contains(&c) {
                return Some(self.labels.get(&c).cloned().unwrap_or(c));
            }
            for id in &self.bundle_ids {
                if c.len() > id.len() && c.starts_with(id.as_str()) && c.as_bytes()[id.len()] == b'.' {
                    return Some(self.labels.get(id).cloned().unwrap_or_else(|| id.clone()));
                }
            }
            if self.names.contains(&c) {
                return Some(c);
            }
        }
        None
    }

    pub fn running_bundle_paths(&self) -> Vec<PathBuf> {
        self.bundle_paths.iter().map(PathBuf::from).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_matches_exact_ids_only() {
        let mut r = RunningApps::empty();
        r.insert_bundle_id("com.google.Chrome");
        assert!(r
            .owner_running(Path::new("/Users/x/Library/Caches/com.google.Chrome"))
            .is_some());
        assert!(r
            .owner_running(Path::new("/Users/x/Library/Saved Application State/com.google.Chrome.savedState"))
            .is_some());
        // a different app whose id merely contains the running id's text
        assert!(r
            .owner_running(Path::new("/Users/x/Library/Caches/com.google.ChromeHelperX"))
            .is_none());
        assert!(r
            .owner_running(Path::new("/Users/x/Library/Caches/com.other.app"))
            .is_none());
    }

    #[test]
    fn snapshot_does_not_panic() {
        let _ = RunningApps::snapshot();
    }
}
