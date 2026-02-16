use std::path::Path;

/// Known SIP-protected paths that cannot be modified
const SIP_PATHS: &[&str] = &[
    "/System",
    "/usr",
    "/bin",
    "/sbin",
    "/var",
    "/Applications/Utilities",
];

/// Paths requiring Full Disk Access
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
];

/// Check if a path is SIP-protected
pub fn is_sip_protected(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    SIP_PATHS.iter().any(|p| path_str.starts_with(p))
}

/// Check if a path likely requires Full Disk Access
pub fn requires_full_disk_access(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    FDA_PATHS.iter().any(|p| path_str.contains(p))
}

/// Check if we can read a path
pub fn can_read(path: &Path) -> bool {
    path.exists() && std::fs::metadata(path).is_ok()
}

/// Check if we can write to a path
pub fn can_write(path: &Path) -> bool {
    if let Some(parent) = path.parent() {
        // Try to check write permission on parent directory
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

/// Check if the current process has Full Disk Access
/// This is a heuristic — we try to read a known FDA-protected path
pub fn has_full_disk_access() -> bool {
    if let Some(home) = dirs::home_dir() {
        let test_path = home.join("Library/Mail");
        if test_path.exists() {
            return std::fs::read_dir(&test_path).is_ok();
        }
    }
    // If Mail folder doesn't exist, we can't test — assume yes
    true
}
