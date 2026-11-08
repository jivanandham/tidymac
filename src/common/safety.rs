//! Deletion safety guard.
//!
//! Every path TidyMac deletes must pass [`check_deletable`]. The guard is an
//! **allowlist + denylist** design:
//!
//! 1. The path is normalized: it must be absolute, must not contain `..` or
//!    `.` components, and its parent directory is canonicalized so symlinked
//!    ancestors (`/tmp` → `/private/tmp`, a symlink inside a cache dir, …)
//!    can't smuggle a path past the checks. The final component itself is
//!    never followed, so deleting a symlink deletes the link, not its target.
//! 2. All comparisons are case-insensitive, because APFS volumes are
//!    case-insensitive by default (`~/documents` *is* `~/Documents`).
//! 3. The path must sit strictly *inside* one of the roots declared by the
//!    scan that produced it (never equal to a root).
//! 4. The path must not hit the denylist for the requested [`DeletePolicy`].

use std::path::{Component, Path, PathBuf};

/// What kind of deletion is being requested. Stricter policies are the default;
/// looser ones are only used by flows where the user picked items explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeletePolicy {
    /// Caches, logs, trash, dev caches found by profile scans.
    #[default]
    Standard,
    /// `node_modules` / Python venvs inside project folders (which may live in
    /// ~/Documents or ~/Desktop).
    DevArtifact,
    /// App-owned data the user explicitly chose to remove: app support,
    /// preferences, containers, cookies, launch agents (uninstall, privacy clean,
    /// orphan cleanup).
    AppData,
    /// An `.app` bundle directly inside /Applications or ~/Applications.
    AppBundle,
    /// A regular file the user selected by hand (duplicates, large files).
    UserFile,
}

/// Home subdirectories that hold personal files. Protected (with everything
/// inside) except for `UserFile` (single files) and `DevArtifact` (node_modules/venvs).
const PERSONAL_DIRS: &[&str] = &[
    "Desktop",
    "Documents",
    "Pictures",
    "Music",
    "Movies",
    "Public",
    "Developer",
];

/// Paths (relative to home) that are never deletable under any policy.
const CRITICAL_HOME: &[&str] = &[
    ".ssh",
    ".gnupg",
    ".aws",
    ".kube",
    ".docker",
    ".netrc",
    ".tidymac",
    "Library/Keychains",
    "Library/Mobile Documents",
    "Library/CloudStorage",
    "Library/Messages",
    "Library/Mail",
    "Library/Safari",
    "Library/Accounts",
    "Library/Calendars",
    "Library/Photos",
    "Library/Application Support/AddressBook",
    "Library/Application Support/MobileSync",
    "Library/Application Support/com.apple.TCC",
    "Library/Containers/com.docker.docker",
];

/// Paths (relative to home) that hold app *data*, not junk. Blocked for
/// `Standard`/`DevArtifact`; allowed for `AppData`.
const APP_DATA_HOME: &[&str] = &[
    "Library/Application Support",
    "Library/Preferences",
    "Library/Containers",
    "Library/Group Containers",
    "Library/LaunchAgents",
    "Library/Cookies",
    "Library/HTTPStorages",
    "Library/WebKit",
    "Library/Application Scripts",
];

/// Exact paths (relative to home) that may never be deleted themselves,
/// although things inside them may be.
const EXACT_HOME: &[&str] = &[
    "",
    "Library",
    "Downloads",
    "Applications",
    ".Trash",
    "Library/Caches",
    "Library/Logs",
    "Library/Developer",
];

/// System roots. Prefix-protected.
const SYSTEM_PREFIXES: &[&str] = &[
    "/System",
    "/usr",
    "/bin",
    "/sbin",
    "/etc",
    "/var",
    "/private",
    "/cores",
    "/opt",
    "/Library",
    "/Network",
    "/dev",
];

/// Lower-case a path for case-insensitive comparison.
fn fold(p: &Path) -> String {
    let s = p.to_string_lossy().to_lowercase();
    if s.len() > 1 {
        s.trim_end_matches('/').to_string()
    } else {
        s
    }
}

/// `child` is strictly inside `parent` (case-insensitive).
fn inside(child: &str, parent: &str) -> bool {
    if parent == "/" {
        return child.len() > 1 && child.starts_with('/');
    }
    child.len() > parent.len() && child.starts_with(parent) && child.as_bytes()[parent.len()] == b'/'
}

/// `child` equals or is inside `parent`.
fn at_or_inside(child: &str, parent: &str) -> bool {
    child == parent || inside(child, parent)
}

/// Normalize a path for safety checks: absolute, no `.`/`..`, canonical parent.
pub fn normalize(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() {
        return Err(format!("not an absolute path: {}", path.display()));
    }
    for c in path.components() {
        match c {
            Component::ParentDir | Component::CurDir => {
                return Err(format!("path contains '.' or '..': {}", path.display()))
            }
            _ => {}
        }
    }
    let name = match path.file_name() {
        Some(n) => n.to_os_string(),
        None => return Ok(path.to_path_buf()), // "/" itself
    };
    let parent = path.parent().unwrap_or(Path::new("/"));
    match std::fs::canonicalize(parent) {
        Ok(canon_parent) => Ok(canon_parent.join(name)),
        // Parent doesn't exist (yet): fall back to the lexical path.
        Err(_) => Ok(path.to_path_buf()),
    }
}

fn home_folded() -> Option<String> {
    let home = dirs::home_dir()?;
    let canon = std::fs::canonicalize(&home).unwrap_or(home);
    Some(fold(&canon))
}

/// Why a path is protected under `policy`, or `None` if it is not.
pub fn protection_reason(path: &Path, policy: DeletePolicy) -> Option<String> {
    let norm = match normalize(path) {
        Ok(p) => p,
        Err(e) => return Some(e),
    };
    let p = fold(&norm);

    // Anything inside a Photos library is never touched directly.
    if p.split('/').any(|c| c.ends_with(".photoslibrary") || c.ends_with(".photolibrary")) {
        return Some("inside a Photos library".into());
    }

    // ── Exceptions that are allowed even though they sit under system roots ──
    if let Some(tmp) = crate::common::sys::user_temp_dir() {
        let t = fold(&tmp);
        if inside(&p, &t) {
            return None; // the user's own $TMPDIR contents
        }
    }
    if let Some(rest) = p.strip_prefix("/volumes/") {
        // /Volumes/<vol>/.Trashes/<uid>/<item…> — the user's trash on an external volume
        let parts: Vec<&str> = rest.split('/').collect();
        let uid = crate::common::sys::current_uid().to_string();
        if parts.len() >= 4 && parts[1] == ".trashes" && parts[2] == uid && policy == DeletePolicy::Standard {
            return None;
        }
        return Some("on another volume".into());
    }
    if p == "/volumes" {
        return Some("system path".into());
    }

    // ── Applications ──
    for apps_root in ["/applications".to_string()]
        .into_iter()
        .chain(home_folded().map(|h| format!("{}/applications", h)))
    {
        if at_or_inside(&p, &apps_root) {
            let direct_child = inside(&p, &apps_root)
                && !p[apps_root.len() + 1..].contains('/');
            if policy == DeletePolicy::AppBundle && direct_child && p.ends_with(".app") {
                return None;
            }
            return Some("application folder".into());
        }
    }

    // ── System roots ──
    if p == "/" {
        return Some("filesystem root".into());
    }
    for root in SYSTEM_PREFIXES {
        if at_or_inside(&p, &root.to_lowercase()) {
            return Some("system location".into());
        }
    }

    // ── Users ──
    let home = match home_folded() {
        Some(h) => h,
        None => return Some("cannot determine home directory".into()),
    };
    if p == "/users" || (inside(&p, "/users") && !at_or_inside(&p, &home)) {
        return Some("another user's files".into());
    }

    if at_or_inside(&p, &home) {
        let rel = if p == home { String::new() } else { p[home.len() + 1..].to_string() };

        for exact in EXACT_HOME {
            if rel == exact.to_lowercase() {
                return Some(format!("~/{} itself is never deleted", exact));
            }
        }
        for crit in CRITICAL_HOME {
            if at_or_inside(&rel, &crit.to_lowercase()) {
                return Some(format!("~/{} is always protected", crit));
            }
        }

        let in_personal = PERSONAL_DIRS
            .iter()
            .any(|d| at_or_inside(&rel, &d.to_lowercase()));
        if in_personal {
            let is_personal_root = PERSONAL_DIRS.iter().any(|d| rel == d.to_lowercase());
            match policy {
                _ if is_personal_root => return Some("personal folder".into()),
                DeletePolicy::UserFile => {
                    // single regular files only
                    match std::fs::symlink_metadata(&norm) {
                        Ok(m) if m.is_file() => {}
                        _ => return Some("only single files may be removed here".into()),
                    }
                }
                DeletePolicy::DevArtifact => {
                    if !is_dev_artifact(&norm) {
                        return Some("personal folder".into());
                    }
                }
                _ => return Some("personal folder (Documents, Desktop, …)".into()),
            }
        }

        let in_app_data = APP_DATA_HOME
            .iter()
            .any(|d| at_or_inside(&rel, &d.to_lowercase()));
        if in_app_data && policy != DeletePolicy::AppData {
            // Allowed exceptions for Standard: caches inside sandbox containers and
            // Mail's attachment cache.
            let is_container_cache = rel.starts_with("library/containers/")
                && (rel.contains("/data/library/caches/")
                    || rel.contains("/data/library/mail downloads/"));
            if !is_container_cache {
                return Some("app data (only removed when you uninstall or pick it)".into());
            }
        }
        if policy == DeletePolicy::AppData && APP_DATA_HOME.iter().any(|d| rel == d.to_lowercase()) {
            return Some("top-level Library folder".into());
        }
        return None;
    }

    // Outside home and not a known system path: be conservative.
    Some("outside your home folder".into())
}

/// A `node_modules` directory or a Python virtualenv directory.
pub fn is_dev_artifact(path: &Path) -> bool {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let is_dir = std::fs::symlink_metadata(path)
        .map(|m| m.is_dir())
        .unwrap_or(false);
    if !is_dir {
        return false;
    }
    if name == "node_modules" {
        return true;
    }
    matches!(name.as_str(), ".venv" | "venv" | "env" | ".env") && path.join("pyvenv.cfg").is_file()
}

/// Check if a path is protected under the default (`Standard`) policy.
pub fn is_protected(path: &Path) -> bool {
    protection_reason(path, DeletePolicy::Standard).is_some()
}

/// Full deletion check. Returns the normalized path on success, or a
/// human-readable reason the path must be skipped.
pub fn check_deletable(path: &Path, roots: &[PathBuf], policy: DeletePolicy) -> Result<PathBuf, String> {
    let norm = normalize(path)?;
    if std::fs::symlink_metadata(&norm).is_err() {
        return Err("no longer exists".into());
    }
    if let Some(reason) = protection_reason(&norm, policy) {
        return Err(format!("protected: {}", reason));
    }
    if policy != DeletePolicy::AppBundle && policy != DeletePolicy::UserFile {
        if roots.is_empty() {
            return Err("no scan root recorded for this item".into());
        }
        let p = fold(&norm);
        let ok = roots.iter().any(|r| {
            let r_norm = std::fs::canonicalize(r).unwrap_or_else(|_| r.clone());
            inside(&p, &fold(&r_norm))
        });
        if !ok {
            return Err("outside the scanned locations".into());
        }
    }
    Ok(norm)
}

/// Validate that a list of paths are safe to delete.
/// Returns the paths that are NOT safe (protected).
pub fn check_paths<'a>(paths: &'a [&'a Path]) -> Vec<&'a Path> {
    paths.iter().filter(|p| is_protected(p)).copied().collect()
}

/// Maximum number of entries to delete in a single operation (hard block
/// unless the caller passes `force`).
pub const MAX_FILES_PER_OPERATION: usize = 100_000;

/// Total bytes above which the user must acknowledge a warning (50 GB).
pub const MAX_BYTES_WARNING_THRESHOLD: u64 = 50 * 1024 * 1024 * 1024;

/// A non-fatal warning about a clean operation that the user should acknowledge.
#[derive(Debug, Clone)]
pub struct CleanWarning {
    pub message: String,
}

impl std::fmt::Display for CleanWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// Result of validating a clean operation.
#[derive(Debug)]
pub enum ValidationResult {
    /// Safe to proceed without warnings.
    Ok,
    /// Non-fatal warning the user should be shown (e.g. unusually large operation).
    Warning(CleanWarning),
}

/// Validate a cleaning operation before execution.
///
/// Returns `Err` for hard blocks (too many entries). Returns
/// `Ok(ValidationResult::Warning)` for soft thresholds the user can acknowledge.
pub fn validate_clean_operation(file_count: usize, total_bytes: u64) -> Result<ValidationResult, String> {
    if file_count > MAX_FILES_PER_OPERATION {
        return Err(format!(
            "Operation would affect {} items (limit: {}). Use --force to override.",
            file_count, MAX_FILES_PER_OPERATION
        ));
    }

    if total_bytes > MAX_BYTES_WARNING_THRESHOLD {
        return Ok(ValidationResult::Warning(CleanWarning {
            message: format!(
                "Operation would remove {} (more than {}).",
                crate::common::format::format_size(total_bytes),
                crate::common::format::format_size(MAX_BYTES_WARNING_THRESHOLD),
            ),
        }));
    }

    Ok(ValidationResult::Ok)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn home() -> PathBuf {
        dirs::home_dir().unwrap()
    }

    #[test]
    fn test_root_is_protected() {
        assert!(is_protected(Path::new("/")));
    }

    #[test]
    fn test_system_dirs_protected() {
        assert!(is_protected(Path::new("/System")));
        assert!(is_protected(Path::new("/Users")));
        assert!(is_protected(Path::new("/Applications")));
        assert!(is_protected(Path::new("/Library")));
        assert!(is_protected(Path::new("/Library/Caches/com.apple.x")));
    }

    #[test]
    fn test_system_subdirs_protected_by_prefix() {
        assert!(is_protected(Path::new("/System/Volumes/Data")));
        assert!(is_protected(Path::new("/System/Library/CoreServices")));
        assert!(is_protected(Path::new("/usr/local/bin")));
        assert!(is_protected(Path::new("/private/var/run")));
    }

    #[test]
    fn test_tmp_resolves_to_private_and_is_protected() {
        // /tmp is a symlink to /private/tmp, which is shared by all users.
        assert!(is_protected(Path::new("/tmp/somefile")));
    }

    #[test]
    fn test_home_dir_protected() {
        let h = home();
        assert!(is_protected(&h));
        assert!(is_protected(&h.join("Desktop")));
        assert!(is_protected(&h.join("Documents")));
        assert!(is_protected(&h.join("Downloads")));
        assert!(is_protected(&h.join(".ssh")));
        assert!(is_protected(&h.join("Library/Caches")));
    }

    #[test]
    fn test_case_insensitive_protection() {
        let h = home();
        assert!(is_protected(&h.join("documents/report.pdf")));
        assert!(is_protected(&h.join("DESKTOP/x.txt")));
    }

    #[test]
    fn test_dotdot_is_rejected() {
        let h = home();
        assert!(is_protected(&h.join("Library/Caches/../../Documents/x")));
    }

    #[test]
    fn test_critical_paths_protected_for_every_policy() {
        let h = home();
        for policy in [
            DeletePolicy::Standard,
            DeletePolicy::AppData,
            DeletePolicy::UserFile,
            DeletePolicy::DevArtifact,
        ] {
            assert!(protection_reason(&h.join("Library/Keychains/login.keychain-db"), policy).is_some());
            assert!(protection_reason(&h.join("Library/Mobile Documents/x"), policy).is_some());
            assert!(protection_reason(&h.join(".ssh/id_ed25519"), policy).is_some());
            assert!(protection_reason(&h.join("Pictures/Photos Library.photoslibrary/x"), policy).is_some());
        }
    }

    #[test]
    fn test_app_data_needs_app_data_policy() {
        let h = home();
        let p = h.join("Library/Application Support/com.example.app");
        assert!(protection_reason(&p, DeletePolicy::Standard).is_some());
        assert!(protection_reason(&p, DeletePolicy::AppData).is_none());
        // container caches are fine for Standard
        let c = h.join("Library/Containers/com.example.app/Data/Library/Caches/x");
        assert!(protection_reason(&c, DeletePolicy::Standard).is_none());
    }

    #[test]
    fn test_cache_dir_contents_not_protected() {
        let h = home();
        assert!(!is_protected(&h.join("Library/Caches/com.example.app")));
        assert!(!is_protected(&h.join("Library/Logs/old.log")));
        assert!(!is_protected(&h.join(".Trash/deleted.txt")));
    }

    #[test]
    fn test_app_bundle_policy() {
        assert!(protection_reason(Path::new("/Applications/Foo.app"), DeletePolicy::AppBundle).is_none());
        assert!(protection_reason(Path::new("/Applications/Foo.app"), DeletePolicy::Standard).is_some());
        assert!(protection_reason(Path::new("/Applications/Utilities/Foo.app"), DeletePolicy::AppBundle).is_some());
        assert!(protection_reason(Path::new("/Applications/Foo.app/Contents"), DeletePolicy::AppBundle).is_some());
    }

    #[test]
    fn test_check_deletable_requires_inside_root() {
        let dir = tempfile::TempDir::new_in(home().join("Library/Caches")).unwrap();
        let root = dir.path().to_path_buf();
        let child = root.join("child");
        std::fs::write(&child, "x").unwrap();
        assert!(check_deletable(&child, &[root.clone()], DeletePolicy::Standard).is_ok());
        // the root itself is never deletable
        assert!(check_deletable(&root, &[root.clone()], DeletePolicy::Standard).is_err());
        // not inside the declared root
        let other = tempfile::TempDir::new_in(home().join("Library/Caches")).unwrap();
        assert!(check_deletable(&child, &[other.path().to_path_buf()], DeletePolicy::Standard).is_err());
    }

    #[test]
    fn test_validate_clean_within_limits() {
        let result = validate_clean_operation(100, 1024 * 1024);
        assert!(matches!(result, Ok(ValidationResult::Ok)));
    }

    #[test]
    fn test_validate_clean_too_many_files() {
        let result = validate_clean_operation(MAX_FILES_PER_OPERATION + 1, 1024);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_clean_large_bytes_gives_warning_not_error() {
        let result = validate_clean_operation(10, MAX_BYTES_WARNING_THRESHOLD + 1);
        assert!(matches!(result, Ok(ValidationResult::Warning(_))));
    }
}
