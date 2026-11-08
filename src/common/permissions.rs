use std::path::Path;

/// Paths protected by System Integrity Protection (read-only system volume and
/// SIP-restricted locations). `/usr/local` is *not* SIP-protected.
const SIP_PATHS: &[&str] = &[
    "/System",
    "/usr",
    "/bin",
    "/sbin",
    "/private/var/db",
    "/Applications/Utilities",
];

/// Paths that macOS privacy controls (TCC) only open to apps with Full Disk Access.
const FDA_PATHS: &[&str] = &[
    "Library/Mail",
    "Library/Messages",
    "Library/Safari",
    "Library/Cookies",
    "Library/HomeKit",
    "Library/IdentityServices",
    "Library/Metadata/CoreSpotlight",
    "Library/PersonalizationPortrait",
    "Library/Suggestions",
    "Library/Containers/com.apple.mail",
];

fn has_prefix(path_str: &str, prefix: &str) -> bool {
    path_str == prefix || path_str.starts_with(&format!("{}/", prefix))
}

/// Check if a path is SIP-protected.
pub fn is_sip_protected(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    if has_prefix(&path_str, "/usr/local") {
        return false;
    }
    SIP_PATHS.iter().any(|p| has_prefix(&path_str, p))
}

/// Check if a path likely requires Full Disk Access.
pub fn requires_full_disk_access(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    FDA_PATHS.iter().any(|p| path_str.contains(p))
}

/// True for app sandbox containers, which macOS protects with its app-data
/// privacy controls (TCC "App Management / App Data"), *not* SIP.
pub fn is_app_container(path: &Path) -> bool {
    let s = path.to_string_lossy();
    s.contains("/Library/Containers/") || s.contains("/Library/Group Containers/")
}

/// Check if we can read a path
pub fn can_read(path: &Path) -> bool {
    path.exists() && std::fs::metadata(path).is_ok()
}

/// Check if we can write to a path
pub fn can_write(path: &Path) -> bool {
    if let Some(parent) = path.parent() {
        std::fs::metadata(parent)
            .map(|m| !m.permissions().readonly())
            .unwrap_or(false)
    } else {
        false
    }
}

/// Get a helpful message for permission issues
pub fn permission_hint(path: &Path) -> String {
    if is_sip_protected(path) {
        "This path is protected by System Integrity Protection (SIP) and cannot be modified."
            .to_string()
    } else if is_app_container(path) {
        "This is an app's sandbox container. macOS privacy controls block access to other apps' data \
         unless TidyMac has Full Disk Access (System Settings > Privacy & Security > Full Disk Access)."
            .to_string()
    } else if requires_full_disk_access(path) {
        "This path requires Full Disk Access. Grant it in System Settings > Privacy & Security > Full Disk Access."
            .to_string()
    } else {
        format!(
            "Check file permissions for '{}'. You may need to run with sudo for system paths.",
            path.display()
        )
    }
}

/// Whether the current process has Full Disk Access.
///
/// The system TCC database always exists and can only be opened by processes
/// with Full Disk Access, so it is a reliable probe. If it can't be found, fall
/// back to user folders that are TCC-protected and almost always present.
pub fn has_full_disk_access() -> bool {
    let system_tcc = Path::new("/Library/Application Support/com.apple.TCC/TCC.db");
    if system_tcc.exists() {
        return std::fs::File::open(system_tcc).is_ok();
    }
    if let Some(home) = dirs::home_dir() {
        for probe in ["Library/Safari", "Library/Mail", "Library/Messages"] {
            let p = home.join(probe);
            if p.exists() {
                return std::fs::read_dir(&p).is_ok();
            }
        }
    }
    // Nothing to probe: report "no" so the UI keeps nudging the user.
    false
}
