use std::path::Path;

/// Home subdirectories that must never be deleted **entirely**.
/// Using prefix matching would also protect Caches/Logs/AppSupport inside Library,
/// which are the primary targets for cleaning. So Library is exact-match only.
///
/// Format: (dir_name, use_prefix_match)
/// - `true`  → protect the dir AND anything nested underneath it
/// - `false` → protect only the dir itself (exact match)
const PROTECTED_HOME_DIRS: &[(&str, bool)] = &[
    ("", false), // home dir itself
    ("Desktop", true),
    ("Documents", true),
    ("Downloads", false), // exact: the Downloads folder itself, but its contents are fair game via scan targets
    ("Pictures", true),
    ("Music", true),
    ("Movies", true),
    ("Library", false), // exact only: Library/Caches, Library/Logs etc. are valid clean targets
    ("Applications", true),
    (".ssh", true),
    (".gnupg", true),
];

/// Check if a path is protected and should NEVER be deleted.
///
/// System paths use prefix matching (e.g. `/System/Volumes` is caught).
/// Home directory paths use a mix: critical personal dirs like `.ssh`, `Desktop`,
/// `Documents` use prefix matching, while `Library` uses exact-only since its
/// subdirectories (Caches, Logs, Application Support) are valid clean targets.
pub fn is_protected(path: &Path) -> bool {
    let path_str = path.to_string_lossy();

    // Never delete root-level system paths.
    // /Users and /Library use EXACT match only — their subdirectories are handled
    // differently: /Users subdirs are governed by the home-dir block below,
    // and ~/Library/Caches etc. are valid clean targets.
    const EXACT_ONLY: &[&str] = &["/", "/Users", "/Library"];
    for protected in EXACT_ONLY {
        if path_str == *protected {
            return true;
        }
    }

    // Other system roots use prefix matching (e.g. /System/Volumes/Data is caught)
    const PREFIX_ROOTS: &[&str] = &[
        "/System",
        "/Applications",
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
    for protected in PREFIX_ROOTS {
        if path_str == *protected || path_str.starts_with(&format!("{}/", protected)) {
            return true;
        }
    }

    // Never delete home directory or critical subdirectories
    if let Some(home) = dirs::home_dir() {
        let home_str = home.to_string_lossy().to_string();

        // Home directory itself (always exact)
        if path_str == home_str {
            return true;
        }

        for (dir, use_prefix) in PROTECTED_HOME_DIRS {
            let protected_path = if dir.is_empty() {
                home_str.clone()
            } else {
                format!("{}/{}", home_str, dir)
            };

            if *use_prefix {
                // Block the dir and everything nested under it
                if path_str == protected_path
                    || path_str.starts_with(&format!("{}/", protected_path))
                {
                    return true;
                }
            } else {
                // Block only the dir itself
                if path_str == protected_path {
                    return true;
                }
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
pub const MAX_FILES_PER_OPERATION: usize = 100_000;

/// Maximum total bytes to delete in a single operation (50 GB).
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
/// Returns `Err` only for hard blocks (e.g. too many files).
/// Returns `Ok(ValidationResult::Warning)` for soft thresholds the user
/// can acknowledge with `--yes`, instead of abusing `Err` for non-fatal cases.
pub fn validate_clean_operation(
    file_count: usize,
    total_bytes: u64,
) -> Result<ValidationResult, String> {
    if file_count > MAX_FILES_PER_OPERATION {
        return Err(format!(
            "Operation would affect {} files (limit: {}). Use --yes to override.",
            file_count, MAX_FILES_PER_OPERATION
        ));
    }

    if total_bytes > MAX_BYTES_WARNING_THRESHOLD {
        return Ok(ValidationResult::Warning(CleanWarning {
            message: format!(
                "Operation would delete {} (>{} threshold). Use --yes to override.",
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
    fn test_system_subdirs_protected_by_prefix() {
        // Key fix: subdirectories of protected system paths are also protected
        assert!(is_protected(Path::new("/System/Volumes/Data")));
        assert!(is_protected(Path::new("/System/Library/CoreServices")));
        assert!(is_protected(Path::new("/usr/local/bin")));
        assert!(is_protected(Path::new("/private/var/run")));
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
    fn test_home_prefix_dirs_protected_recursively() {
        if let Some(home) = dirs::home_dir() {
            // Prefix-protected dirs: their contents are also protected
            assert!(is_protected(&home.join("Desktop/myfile.txt")));
            assert!(is_protected(&home.join("Documents/work/report.pdf")));
            assert!(is_protected(&home.join(".ssh/id_rsa")));
        }
    }

    #[test]
    fn test_cache_dir_not_protected() {
        if let Some(home) = dirs::home_dir() {
            // Library itself IS protected (exact), but subdirs are NOT (they are clean targets)
            assert!(is_protected(&home.join("Library")));
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
        // Should be Ok(Warning), not Err, for the soft threshold
        assert!(matches!(result, Ok(ValidationResult::Warning(_))));
    }
}
