//! Installed-app discovery and leftover detection.
//!
//! Leftovers are matched by **exact bundle identifier** (and the bundle's
//! executable name for crash reports). A name-based match is only used for a
//! few folder types, only for distinctive names, and is flagged so the user
//! confirms it item by item.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Source of application installation
#[derive(Debug, Clone, PartialEq)]
pub enum AppSource {
    Applications,     // /Applications
    UserApplications, // ~/Applications
    Homebrew,         // Homebrew cask
    System,           // System apps (not removable)
}

impl std::fmt::Display for AppSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppSource::Applications => write!(f, "Applications"),
            AppSource::UserApplications => write!(f, "User Apps"),
            AppSource::Homebrew => write!(f, "Homebrew"),
            AppSource::System => write!(f, "System"),
        }
    }
}

/// Information about an installed application
#[derive(Debug, Clone)]
pub struct InstalledApp {
    pub name: String,
    pub bundle_id: Option<String>,
    pub version: Option<String>,
    /// `CFBundleExecutable`
    pub executable: Option<String>,
    pub path: PathBuf,
    pub app_size: u64,
    pub associated_files: Vec<AssociatedFile>,
    pub total_size: u64,
    pub last_opened: Option<SystemTime>,
    pub source: AppSource,
}

impl InstalledApp {
    /// Apple's own apps are never offered for removal.
    pub fn is_apple(&self) -> bool {
        self.bundle_id
            .as_deref()
            .map(|b| b.to_lowercase().starts_with("com.apple."))
            .unwrap_or(false)
    }

    /// Bytes of leftovers (everything except the bundle).
    pub fn leftovers_size(&self) -> u64 {
        self.associated_files.iter().filter(|f| f.exists).map(|f| f.size).sum()
    }
}

/// How an associated file was linked to its app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchKind {
    /// Exact bundle identifier (or `<id>.*`, `group.<id>`, `<TEAM>.<id>`).
    BundleId,
    /// Exact executable name (crash reports).
    Executable,
    /// Folder named exactly like the app — needs confirmation.
    Name,
    /// Listed in the app's Homebrew cask `zap` stanza.
    BrewZap,
}

impl std::fmt::Display for MatchKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MatchKind::BundleId => write!(f, "bundle id"),
            MatchKind::Executable => write!(f, "executable name"),
            MatchKind::Name => write!(f, "app name (confirm)"),
            MatchKind::BrewZap => write!(f, "Homebrew zap"),
        }
    }
}

/// A file or directory associated with an application
#[derive(Debug, Clone)]
pub struct AssociatedFile {
    pub path: PathBuf,
    pub size: u64,
    pub kind: AssociatedKind,
    pub exists: bool,
    pub matched_by: MatchKind,
}

impl AssociatedFile {
    /// Needs the user to confirm before removal (name-based guess).
    pub fn needs_confirmation(&self) -> bool {
        self.matched_by == MatchKind::Name
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssociatedKind {
    AppSupport,
    Cache,
    Preferences,
    SavedState,
    Container,
    GroupContainer,
    Cookies,
    HttpStorage,
    WebKit,
    Logs,
    LaunchAgent,
    LoginItem,
    CrashReports,
    Other,
}

impl AssociatedKind {
    /// Data the app recreates by itself; safe to clear while keeping the app.
    pub fn is_regenerable(&self) -> bool {
        matches!(
            self,
            AssociatedKind::Cache
                | AssociatedKind::Logs
                | AssociatedKind::SavedState
                | AssociatedKind::CrashReports
                | AssociatedKind::HttpStorage
                | AssociatedKind::WebKit
        )
    }

    /// Infer a kind from a path (used for Homebrew zap entries and orphans).
    pub fn from_path(path: &Path) -> Self {
        let s = path.to_string_lossy();
        let table = [
            ("/Library/Application Support/", AssociatedKind::AppSupport),
            ("/Library/Caches/", AssociatedKind::Cache),
            ("/Library/Preferences/", AssociatedKind::Preferences),
            ("/Library/Saved Application State/", AssociatedKind::SavedState),
            ("/Library/Containers/", AssociatedKind::Container),
            ("/Library/Group Containers/", AssociatedKind::GroupContainer),
            ("/Library/Cookies/", AssociatedKind::Cookies),
            ("/Library/HTTPStorages/", AssociatedKind::HttpStorage),
            ("/Library/WebKit/", AssociatedKind::WebKit),
            ("/Library/Logs/DiagnosticReports/", AssociatedKind::CrashReports),
            ("/Library/Logs/", AssociatedKind::Logs),
            ("/Library/LaunchAgents/", AssociatedKind::LaunchAgent),
        ];
        for (needle, kind) in table {
            if s.contains(needle) {
                return kind;
            }
        }
        AssociatedKind::Other
    }
}

impl std::fmt::Display for AssociatedKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssociatedKind::AppSupport => write!(f, "App Support"),
            AssociatedKind::Cache => write!(f, "Cache"),
            AssociatedKind::Preferences => write!(f, "Preferences"),
            AssociatedKind::SavedState => write!(f, "Saved State"),
            AssociatedKind::Container => write!(f, "Container"),
            AssociatedKind::GroupContainer => write!(f, "Group Container"),
            AssociatedKind::Cookies => write!(f, "Cookies"),
            AssociatedKind::HttpStorage => write!(f, "HTTP Storage"),
            AssociatedKind::WebKit => write!(f, "WebKit Data"),
            AssociatedKind::Logs => write!(f, "Logs"),
            AssociatedKind::LaunchAgent => write!(f, "Launch Agent"),
            AssociatedKind::LoginItem => write!(f, "Login Item"),
            AssociatedKind::CrashReports => write!(f, "Crash Reports"),
            AssociatedKind::Other => write!(f, "Other"),
        }
    }
}

/// Discover all installed applications
pub fn discover_apps() -> Vec<InstalledApp> {
    let mut apps = Vec::new();

    scan_app_directory(Path::new("/Applications"), AppSource::Applications, &mut apps, 1);
    if let Some(home) = dirs::home_dir() {
        scan_app_directory(&home.join("Applications"), AppSource::UserApplications, &mut apps, 1);
    }

    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps
}

/// Scan a directory for .app bundles (and one level of plain sub-folders,
/// e.g. /Applications/Utilities or /Applications/Adobe Foo/).
fn scan_app_directory(dir: &Path, source: AppSource, apps: &mut Vec<InstalledApp>, depth: usize) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        if !is_dir {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) == Some("app") {
            if let Some(app) = parse_app_bundle(&path, source.clone()) {
                apps.push(app);
            }
        } else if depth > 0 {
            scan_app_directory(&path, source.clone(), apps, depth - 1);
        }
    }
}

/// Parse an .app bundle to extract metadata
pub fn parse_app_bundle(app_path: &Path, source: AppSource) -> Option<InstalledApp> {
    let name = app_path.file_stem()?.to_string_lossy().to_string();
    let info = parse_info_plist(&app_path.join("Contents/Info.plist"));

    let app_size = crate::scanner::walker::dir_size(app_path);
    let last_opened = std::fs::metadata(app_path).ok().and_then(|m| m.accessed().ok());

    let associated = find_associated_files_full(&name, info.bundle_id.as_deref(), info.executable.as_deref());
    let assoc_size: u64 = associated.iter().filter(|a| a.exists).map(|a| a.size).sum();

    let source = if info
        .bundle_id
        .as_deref()
        .map(|b| b.starts_with("com.apple."))
        .unwrap_or(false)
    {
        AppSource::System
    } else {
        source
    };

    Some(InstalledApp {
        name,
        bundle_id: info.bundle_id,
        version: info.version,
        executable: info.executable,
        path: app_path.to_path_buf(),
        app_size,
        associated_files: associated,
        total_size: app_size + assoc_size,
        last_opened,
        source,
    })
}

#[derive(Default)]
struct BundleInfo {
    bundle_id: Option<String>,
    version: Option<String>,
    executable: Option<String>,
}

fn parse_info_plist(path: &Path) -> BundleInfo {
    let mut info = BundleInfo::default();
    let value = match plist::Value::from_file(path) {
        Ok(v) => v,
        Err(_) => return info,
    };
    if let Some(dict) = value.as_dictionary() {
        let get = |k: &str| dict.get(k).and_then(|v| v.as_string()).map(|s| s.to_string());
        info.bundle_id = get("CFBundleIdentifier");
        info.version = get("CFBundleShortVersionString");
        info.executable = get("CFBundleExecutable");
    }
    info
}

/// Read just the bundle identifier of an app bundle.
pub fn bundle_id_of(app_path: &Path) -> Option<String> {
    parse_info_plist(&app_path.join("Contents/Info.plist")).bundle_id
}

/// Names too generic to use for name-based matching.
const GENERIC_NAMES: &[&str] = &[
    "app", "apps", "data", "cache", "caches", "logs", "helper", "update", "updater", "agent",
    "google", "microsoft", "adobe", "apple", "mozilla", "jetbrains", "default", "common",
    "shared", "temp", "tmp", "user", "users", "settings", "config", "electron",
];

fn name_is_distinctive(name: &str) -> bool {
    name.chars().count() >= 4 && !GENERIC_NAMES.contains(&name.to_lowercase().as_str())
}

/// Find all files associated with an application (compat signature).
pub fn find_associated_files(app_name: &str, bundle_id: Option<&str>) -> Vec<AssociatedFile> {
    find_associated_files_full(app_name, bundle_id, None)
}

/// Find all files associated with an application.
pub fn find_associated_files_full(
    app_name: &str,
    bundle_id: Option<&str>,
    executable: Option<&str>,
) -> Vec<AssociatedFile> {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return Vec::new(),
    };
    let lib = home.join("Library");
    let mut files = Vec::new();

    if let Some(bid) = bundle_id {
        // Never attribute Apple's shared system data to an app.
        if !bid.to_lowercase().starts_with("com.apple.") || bid.contains(".dt.") {
            let exact = |rel: &str, kind: AssociatedKind, files: &mut Vec<AssociatedFile>| {
                check_associated(&lib.join(rel).join(bid), kind, MatchKind::BundleId, files);
            };
            exact("Application Support", AssociatedKind::AppSupport, &mut files);
            exact("Caches", AssociatedKind::Cache, &mut files);
            exact("Containers", AssociatedKind::Container, &mut files);
            exact("HTTPStorages", AssociatedKind::HttpStorage, &mut files);
            exact("WebKit", AssociatedKind::WebKit, &mut files);
            exact("Logs", AssociatedKind::Logs, &mut files);
            exact("Application Scripts", AssociatedKind::Other, &mut files);

            check_associated(
                &lib.join("Preferences").join(format!("{}.plist", bid)),
                AssociatedKind::Preferences,
                MatchKind::BundleId,
                &mut files,
            );
            check_associated(
                &lib.join("Saved Application State").join(format!("{}.savedState", bid)),
                AssociatedKind::SavedState,
                MatchKind::BundleId,
                &mut files,
            );
            check_associated(
                &lib.join("Cookies").join(format!("{}.binarycookies", bid)),
                AssociatedKind::Cookies,
                MatchKind::BundleId,
                &mut files,
            );
            check_associated(
                &lib.join("HTTPStorages").join(format!("{}.binarycookies", bid)),
                AssociatedKind::Cookies,
                MatchKind::BundleId,
                &mut files,
            );

            // Prefixed entries: <id>.<something>
            scan_dir_matching(&lib.join("Preferences/ByHost"), AssociatedKind::Preferences, &mut files, |n| {
                has_id_prefix(n, bid)
            });
            scan_dir_matching(&lib.join("LaunchAgents"), AssociatedKind::LaunchAgent, &mut files, |n| {
                n.ends_with(".plist") && (n == format!("{}.plist", bid) || has_id_prefix(n, bid))
            });
            scan_dir_matching(&lib.join("Caches"), AssociatedKind::Cache, &mut files, |n| {
                has_id_prefix(n, bid)
            });
            // Group containers: group.<id>, <TEAMID>.<id>, either optionally followed by .suffix
            scan_dir_matching(&lib.join("Group Containers"), AssociatedKind::GroupContainer, &mut files, |n| {
                group_container_matches(n, bid)
            });
        }
    }

    // Crash reports: <Executable>-<date>.ips / <Executable>_<date>.crash
    let exe = executable.unwrap_or(app_name);
    if exe.chars().count() >= 3 {
        scan_dir_matching_as(
            &lib.join("Logs/DiagnosticReports"),
            AssociatedKind::CrashReports,
            MatchKind::Executable,
            &mut files,
            |n| n.len() > exe.len() && n.starts_with(exe) && matches!(n.as_bytes()[exe.len()], b'-' | b'_' | b'.'),
        );
    }

    // Name-based guesses (only for distinctive names, flagged for confirmation)
    if name_is_distinctive(app_name) {
        for (rel, kind) in [
            ("Application Support", AssociatedKind::AppSupport),
            ("Caches", AssociatedKind::Cache),
            ("Logs", AssociatedKind::Logs),
        ] {
            check_associated(&lib.join(rel).join(app_name), kind, MatchKind::Name, &mut files);
        }
    }

    // Deduplicate by path, keeping the strongest match (BundleId first).
    files.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then((a.matched_by == MatchKind::Name).cmp(&(b.matched_by == MatchKind::Name)))
    });
    files.dedup_by(|a, b| a.path == b.path);
    files
}

/// `name` is `<id>.<suffix>` (case-insensitive).
fn has_id_prefix(name: &str, id: &str) -> bool {
    let n = name.to_lowercase();
    let i = id.to_lowercase();
    n.len() > i.len() && n.starts_with(&i) && n.as_bytes()[i.len()] == b'.'
}

fn group_container_matches(name: &str, id: &str) -> bool {
    let n = name.to_lowercase();
    let i = id.to_lowercase();
    let rest = if let Some(r) = n.strip_prefix("group.") {
        r.to_string()
    } else if let Some(idx) = name.find('.') {
        // <TEAMID>.<id> — team ids are 10 upper-case alphanumerics
        let team = &name[..idx];
        if team.len() == 10 && team.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()) {
            name[idx + 1..].to_lowercase()
        } else {
            return false;
        }
    } else {
        return false;
    };
    rest == i || (rest.len() > i.len() && rest.starts_with(&i) && rest.as_bytes()[i.len()] == b'.')
}

fn scan_dir_matching<F: Fn(&str) -> bool>(dir: &Path, kind: AssociatedKind, files: &mut Vec<AssociatedFile>, pred: F) {
    scan_dir_matching_as(dir, kind, MatchKind::BundleId, files, pred)
}

fn scan_dir_matching_as<F: Fn(&str) -> bool>(
    dir: &Path,
    kind: AssociatedKind,
    matched_by: MatchKind,
    files: &mut Vec<AssociatedFile>,
    pred: F,
) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let name = entry.file_name().to_string_lossy().to_string();
            if pred(&name) {
                check_associated(&entry.path(), kind.clone(), matched_by.clone(), files);
            }
        }
    }
}

/// Add a path as an associated file if it exists.
pub fn check_associated(path: &Path, kind: AssociatedKind, matched_by: MatchKind, files: &mut Vec<AssociatedFile>) {
    let md = match std::fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(_) => return,
    };
    let size = if md.is_dir() {
        crate::scanner::walker::dir_size(path)
    } else {
        use std::os::unix::fs::MetadataExt;
        md.blocks() * 512
    };
    files.push(AssociatedFile {
        path: path.to_path_buf(),
        size,
        kind,
        exists: true,
        matched_by,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_container_matching() {
        assert!(group_container_matches("group.com.tinyspeck.slackmacgap", "com.tinyspeck.slackmacgap"));
        assert!(group_container_matches("BQR82RBBHL.com.tinyspeck.slackmacgap", "com.tinyspeck.slackmacgap"));
        assert!(group_container_matches("BQR82RBBHL.com.tinyspeck.slackmacgap.shared", "com.tinyspeck.slackmacgap"));
        assert!(!group_container_matches("group.com.tinyspeck.slackmacgapx", "com.tinyspeck.slackmacgap"));
        assert!(!group_container_matches("com.other.app", "com.tinyspeck.slackmacgap"));
        assert!(!group_container_matches("lowercase1.com.tinyspeck.slackmacgap", "com.tinyspeck.slackmacgap"));
    }

    #[test]
    fn id_prefix_matching() {
        assert!(has_id_prefix("com.foo.app.plist", "com.foo.app"));
        assert!(!has_id_prefix("com.foo.application.plist", "com.foo.app"));
        assert!(!has_id_prefix("com.foo.app", "com.foo.app"));
    }

    #[test]
    fn generic_names_are_not_used() {
        assert!(!name_is_distinctive("Go"));
        assert!(!name_is_distinctive("Google"));
        assert!(name_is_distinctive("Slack"));
    }
}
