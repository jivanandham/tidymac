use std::path::Path;

/// Paths that must NEVER be deleted under any circumstances.
/// This is a critical safety net against bugs in scan targets.
const PROTECTED_PATHS: &[&str] = &[
    "/",
    "/System",
    "/Applications",
    "/Users",
    "/Library",
    "/usr",
    "/bin",
    "/sbin",
    "/var",
    "/etc",
    "/opt",
    "/private",
    "/cores",
    "/Volumes",
];

/// Paths under home that must never be deleted entirely
const PROTECTED_HOME_DIRS: &[&str] = &[
    "", // home dir itself
    "Desktop",
    "Documents",
    "Downloads",
    "Pictures",
    "Music",
    "Movies",
    "Library",
    "Applications",
    ".ssh",
    ".gnupg",
];

/// Check if a path is protected and should NEVER be deleted
pub fn is_protected(path: &Path) -> bool {
    let path_str = path.to_string_lossy();

    // Never delete root-level system paths
    for protected in PROTECTED_PATHS {
        if path_str == *protected {
            return true;
        }
    }

    // Never delete home directory or critical subdirectories
    if let Some(home) = dirs::home_dir() {
        let home_str = home.to_string_lossy().to_string();

        // The home directory itself
        if path_str == home_str {
            return true;
        }

        // Critical home subdirectories
        for dir in PROTECTED_HOME_DIRS {
            let protected_path = if dir.is_empty() {
                home_str.clone()
            } else {
                format!("{}/{}", home_str, dir)
            };
            if path_str == protected_path {
                return true;
            }
        }
    }

    false
}

/// Validate that a list of paths are safe to delete.
/// Returns the paths that are NOT safe (protected).
pub fn check_paths<'a>(paths: &'a [&'a Path]) -> Vec<&'a Path> {
    paths.iter().filter(|p| is_protected(p)).copied().collect()
}

/// Maximum number of files to delete in a single operation.
/// A safety limit to prevent runaway deletion bugs.
pub const MAX_FILES_PER_OPERATION: usize = 100_000;

/// Maximum total bytes to delete in a single operation (50 GB).
/// User can override with --yes flag.
pub const MAX_BYTES_WARNING_THRESHOLD: u64 = 50 * 1024 * 1024 * 1024;

/// Validate a cleaning operation before execution
pub fn validate_clean_operation(
    file_count: usize,
    total_bytes: u64,
) -> Result<(), String> {
    if file_count > MAX_FILES_PER_OPERATION {
        return Err(format!(
            "Operation would affect {} files (limit: {}). Use --yes to override.",
            file_count, MAX_FILES_PER_OPERATION
        ));
    }

    // Warning threshold, not a hard block
    if total_bytes > MAX_BYTES_WARNING_THRESHOLD {
        return Err(format!(
            "Operation would delete {} (>{} threshold). Use --yes to override.",
            crate::common::format::format_size(total_bytes),
            crate::common::format::format_size(MAX_BYTES_WARNING_THRESHOLD),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
    }

    #[test]
    fn test_home_dir_protected() {
        if let Some(home) = dirs::home_dir() {
            assert!(is_protected(&home));
            assert!(is_protected(&home.join("Desktop")));
            assert!(is_protected(&home.join("Documents")));
            assert!(is_protected(&home.join("Downloads")));
            assert!(is_protected(&home.join(".ssh")));
        }
    }

    #[test]
    fn test_cache_dir_not_protected() {
        if let Some(home) = dirs::home_dir() {
            assert!(!is_protected(&home.join("Library/Caches/com.example.app")));
            assert!(!is_protected(&home.join("Library/Logs/old.log")));
            assert!(!is_protected(&home.join(".Trash/deleted.txt")));
        }
    }

    #[test]
    fn test_tmp_not_protected() {
        assert!(!is_protected(Path::new("/tmp/somefile")));
    }

    #[test]
    fn test_validate_clean_within_limits() {
        let result = validate_clean_operation(100, 1024 * 1024);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_clean_too_many_files() {
        let result = validate_clean_operation(MAX_FILES_PER_OPERATION + 1, 1024);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_clean_too_many_bytes() {
        let result = validate_clean_operation(10, MAX_BYTES_WARNING_THRESHOLD + 1);
        assert!(result.is_err());
    }
}
