use tempfile::TempDir;

use tidymac::common::config::Config;
use tidymac::common::format;
use tidymac::common::permissions;
use tidymac::profiles::loader::Profile;
use tidymac::scanner::targets;
use tidymac::scanner::walker;

// ─── Format tests ─────────────────────────────────────────────────────────────

#[test]
fn test_format_size_boundaries() {
    assert_eq!(format::format_size(0), "0 B");
    assert_eq!(format::format_size(1), "1 B");
    assert_eq!(format::format_size(1023), "1023 B");
    assert_eq!(format::format_size(1024), "1.0 KB");
    assert_eq!(format::format_size(1024 * 1024 - 1), "1024.0 KB");
    assert_eq!(format::format_size(1024 * 1024), "1.00 MB");
    // Just verify u64::MAX doesn't panic
    let result = format::format_size(u64::MAX);
    assert!(result.contains("TB"));
}

#[test]
fn test_format_path_with_home() {
    // format_path should replace home dir with ~
    if let Some(home) = dirs::home_dir() {
        let test_path = home.join("Documents/test.txt");
        let formatted = format::format_path(&test_path);
        assert!(formatted.starts_with("~/"), "Path should start with ~/, got: {}", formatted);
        assert!(formatted.contains("Documents/test.txt"));
    }
}

#[test]
fn test_format_path_without_home() {
    let path = std::path::Path::new("/tmp/test.txt");
    let formatted = format::format_path(path);
    assert_eq!(formatted, "/tmp/test.txt");
}

#[test]
fn test_truncate_edge_cases() {
    assert_eq!(format::truncate("", 5), "");
    assert_eq!(format::truncate("ab", 2), "ab");
    assert_eq!(format::truncate("abc", 3), "abc");
    assert_eq!(format::truncate("abcd", 3), "...");
    assert_eq!(format::truncate("abcde", 4), "a...");
}

// ─── Permissions tests ────────────────────────────────────────────────────────

#[test]
fn test_sip_protected_paths() {
    assert!(permissions::is_sip_protected(std::path::Path::new("/System/Library")));
    assert!(permissions::is_sip_protected(std::path::Path::new("/usr/bin/ls")));
    assert!(!permissions::is_sip_protected(std::path::Path::new("/Applications/Safari.app")));
    assert!(!permissions::is_sip_protected(std::path::Path::new("/tmp/test")));
}

#[test]
fn test_fda_paths() {
    assert!(permissions::requires_full_disk_access(
        std::path::Path::new("/Users/test/Library/Mail/V9")
    ));
    assert!(permissions::requires_full_disk_access(
        std::path::Path::new("/Users/test/Library/Safari/History.db")
    ));
    assert!(!permissions::requires_full_disk_access(
        std::path::Path::new("/Users/test/Library/Caches/foo")
    ));
}

// ─── Config tests ─────────────────────────────────────────────────────────────

#[test]
fn test_config_defaults() {
    let config = Config::default();
    assert_eq!(config.staging_retention_days, 7);
    assert_eq!(config.large_file_threshold_mb, 500);
    assert_eq!(config.stale_days, 30);
    assert_eq!(config.default_profile, "quick_sweep");
    assert!(config.exclude_paths.is_empty());
}

#[test]
fn test_config_large_file_threshold_bytes() {
    let config = Config::default();
    assert_eq!(config.large_file_threshold_bytes(), 500 * 1024 * 1024);
}

#[test]
fn test_config_is_excluded() {
    let mut config = Config::default();
    config.exclude_paths = vec![
        "node_modules".to_string(),
        ".git".to_string(),
    ];

    assert!(config.is_excluded(std::path::Path::new("/Users/test/projects/app/node_modules")));
    assert!(config.is_excluded(std::path::Path::new("/Users/test/repos/tidymac/.git")));
    assert!(!config.is_excluded(std::path::Path::new("/Users/test/Documents/report.pdf")));
}

#[test]
fn test_config_serialization_roundtrip() {
    let config = Config::default();
    let toml_str = toml::to_string_pretty(&config).unwrap();
    let loaded: Config = toml::from_str(&toml_str).unwrap();

    assert_eq!(loaded.staging_retention_days, config.staging_retention_days);
    assert_eq!(loaded.large_file_threshold_mb, config.large_file_threshold_mb);
    assert_eq!(loaded.stale_days, config.stale_days);
}

// ─── Profile tests ────────────────────────────────────────────────────────────

#[test]
fn test_builtin_profiles_load() {
    let names = ["quick", "quick_sweep", "developer", "dev", "creative", "deep", "deep_clean"];
    for name in &names {
        let profile = Profile::load(name);
        assert!(profile.is_ok(), "Profile '{}' should load successfully", name);
    }
}

#[test]
fn test_unknown_profile_fails() {
    let result = Profile::load("nonexistent_profile_xyz");
    assert!(result.is_err());
}

#[test]
fn test_developer_profile_enables_dev_targets() {
    let profile = Profile::load("developer").unwrap();
    assert!(profile.targets.dev.xcode_derived_data);
    assert!(profile.targets.dev.homebrew_cache);
    assert!(profile.targets.dev.npm_cache);
    assert!(profile.targets.dev.pip_cache);
    assert!(profile.targets.dev.cargo_cache);
    assert!(profile.targets.dev.node_modules_stale);
    assert!(profile.targets.dev.venv);
    assert!(profile.includes_dev_projects());
}

#[test]
fn test_quick_profile_disables_dev_targets() {
    let profile = Profile::load("quick").unwrap();
    assert!(!profile.targets.dev.xcode_derived_data);
    assert!(!profile.targets.dev.homebrew_cache);
    assert!(!profile.includes_dev_projects());
}

#[test]
fn test_deep_profile_enables_everything() {
    let profile = Profile::load("deep").unwrap();
    assert!(profile.targets.system_caches);
    assert!(profile.targets.trash);
    assert!(profile.targets.large_files);
    assert!(profile.targets.mail_attachments);
    assert!(profile.targets.dev.xcode_derived_data);
    assert!(profile.targets.dev.docker_dangling);
}

#[test]
fn test_profile_enabled_targets_returns_correct_subset() {
    let quick = Profile::load("quick").unwrap();
    let deep = Profile::load("deep").unwrap();

    let quick_targets = quick.enabled_targets();
    let deep_targets = deep.enabled_targets();

    assert!(
        deep_targets.len() >= quick_targets.len(),
        "Deep profile should enable at least as many targets as quick"
    );
}

#[test]
fn test_available_profiles_includes_builtins() {
    let profiles = Profile::available_profiles();
    assert!(profiles.contains(&"quick".to_string()));
    assert!(profiles.contains(&"developer".to_string()));
    assert!(profiles.contains(&"creative".to_string()));
    assert!(profiles.contains(&"deep".to_string()));
}

// ─── Scanner targets tests ────────────────────────────────────────────────────

#[test]
fn test_all_targets_non_empty() {
    let all = targets::all_targets();
    assert!(!all.is_empty(), "Should have at least some scan targets");
}

#[test]
fn test_system_junk_targets() {
    let targets = targets::system_junk_targets();
    assert!(targets.len() >= 5, "Should have at least 5 system junk targets");

    // Verify each target has required fields
    for t in &targets {
        assert!(!t.name.is_empty());
        assert!(!t.paths.is_empty());
        assert!(!t.reason.is_empty());
    }
}

#[test]
fn test_developer_targets() {
    let targets = targets::developer_targets();
    assert!(targets.len() >= 10, "Should have at least 10 developer targets");

    let names: Vec<&str> = targets.iter().map(|t| t.name.as_str()).collect();
    assert!(names.iter().any(|n| n.contains("Xcode")));
    assert!(names.iter().any(|n| n.contains("Homebrew")));
    assert!(names.iter().any(|n| n.contains("npm")));
    assert!(names.iter().any(|n| n.contains("pip")));
    assert!(names.iter().any(|n| n.contains("Cargo")));
}

// ─── Walker tests ─────────────────────────────────────────────────────────────

#[test]
fn test_expand_paths_tilde() {
    let paths = vec!["~/Documents".to_string()];
    let expanded = walker::expand_paths(&paths);

    assert_eq!(expanded.len(), 1);
    assert!(!expanded[0].to_string_lossy().contains('~'), "Tilde should be expanded");

    if let Some(home) = dirs::home_dir() {
        assert!(
            expanded[0].starts_with(&home),
            "Expanded path should start with home directory"
        );
    }
}

#[test]
fn test_dir_size_empty_dir() {
    let dir = TempDir::new().unwrap();
    let size = walker::dir_size(dir.path());
    assert_eq!(size, 0, "Empty directory should have zero size");
}

#[test]
fn test_dir_size_with_files() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("a.txt"), "hello").unwrap(); // 5 bytes
    std::fs::write(dir.path().join("b.txt"), "world!").unwrap(); // 6 bytes

    let size = walker::dir_size(dir.path());
    // Physical disk usage (st_blocks * 512) is block-aligned, so >= logical size
    assert!(size >= 11, "Dir size should be at least sum of file sizes, got {}", size);
    assert!(size <= 16384, "Dir size should be reasonable, got {}", size);
}

#[test]
fn test_dir_size_nested() {
    let dir = TempDir::new().unwrap();
    let sub = dir.path().join("subdir");
    std::fs::create_dir_all(&sub).unwrap();

    std::fs::write(dir.path().join("root.txt"), "abc").unwrap(); // 3
    std::fs::write(sub.join("nested.txt"), "defgh").unwrap(); // 5

    let size = walker::dir_size(dir.path());
    // Physical disk usage is block-aligned
    assert!(size >= 8, "Should include nested files, got {}", size);
    assert!(size <= 16384, "Dir size should be reasonable, got {}", size);
}

#[test]
fn test_dir_size_nonexistent() {
    let size = walker::dir_size(std::path::Path::new("/nonexistent/path/xyz"));
    assert_eq!(size, 0);
}

#[test]
fn test_find_large_files() {
    let dir = TempDir::new().unwrap();
    // Create a non-hidden subdir since find_large_files skips dotfiles
    let scan_dir = dir.path().join("testdir");
    std::fs::create_dir_all(&scan_dir).unwrap();

    std::fs::write(scan_dir.join("small.txt"), "tiny").unwrap();
    std::fs::write(scan_dir.join("big.txt"), "x".repeat(2000)).unwrap();

    let large = walker::find_large_files(&scan_dir, 1000);
    // Both files may exceed 1000 bytes in physical blocks, so filter by name
    let big_files: Vec<_> = large.iter().filter(|f| f.path.to_string_lossy().contains("big.txt")).collect();
    assert_eq!(big_files.len(), 1, "Should find big.txt as large file");
    assert!(big_files[0].size_bytes >= 2000, "big.txt physical size should be >= 2000");
}

#[test]
fn test_find_large_files_empty_dir() {
    let dir = TempDir::new().unwrap();
    let scan_dir = dir.path().join("testdir");
    std::fs::create_dir_all(&scan_dir).unwrap();
    let large = walker::find_large_files(&scan_dir, 1000);
    assert!(large.is_empty());
}

#[test]
fn test_find_large_files_sorted_descending() {
    let dir = TempDir::new().unwrap();
    let scan_dir = dir.path().join("testdir");
    std::fs::create_dir_all(&scan_dir).unwrap();

    std::fs::write(scan_dir.join("medium.txt"), "x".repeat(2000)).unwrap();
    std::fs::write(scan_dir.join("large.txt"), "x".repeat(5000)).unwrap();
    std::fs::write(scan_dir.join("xlarge.txt"), "x".repeat(9000)).unwrap();

    let large = walker::find_large_files(&scan_dir, 1000);
    assert_eq!(large.len(), 3);
    assert!(
        large[0].size_bytes >= large[1].size_bytes && large[1].size_bytes >= large[2].size_bytes,
        "Results should be sorted by size descending"
    );
}

// ─── Scan results tests ──────────────────────────────────────────────────────

#[test]
fn test_scan_results_recalculate() {
    let mut results = targets::ScanResults::new();

    results.items.push(targets::ScanItem {
        name: "Test 1".to_string(),
        category: targets::Category::UserCache,
        path: std::path::PathBuf::from("/tmp/test1"),
        size_bytes: 1000,
        file_count: 5,
        safety: targets::SafetyLevel::Safe,
        reason: "test".to_string(),
        files: Vec::new(),
    });

    results.items.push(targets::ScanItem {
        name: "Test 2".to_string(),
        category: targets::Category::Logs,
        path: std::path::PathBuf::from("/tmp/test2"),
        size_bytes: 2000,
        file_count: 3,
        safety: targets::SafetyLevel::Caution,
        reason: "test".to_string(),
        files: Vec::new(),
    });

    results.recalculate();

    assert_eq!(results.total_reclaimable, 3000);
    assert_eq!(results.total_files, 8);
}

#[test]
fn test_scan_results_filter_by_safety() {
    let mut results = targets::ScanResults::new();

    results.items.push(targets::ScanItem {
        name: "Safe item".to_string(),
        category: targets::Category::UserCache,
        path: std::path::PathBuf::from("/tmp/safe"),
        size_bytes: 1000,
        file_count: 1,
        safety: targets::SafetyLevel::Safe,
        reason: "test".to_string(),
        files: Vec::new(),
    });

    results.items.push(targets::ScanItem {
        name: "Caution item".to_string(),
        category: targets::Category::Logs,
        path: std::path::PathBuf::from("/tmp/caution"),
        size_bytes: 2000,
        file_count: 1,
        safety: targets::SafetyLevel::Caution,
        reason: "test".to_string(),
        files: Vec::new(),
    });

    let safe = results.filter_by_safety(&targets::SafetyLevel::Safe);
    assert_eq!(safe.len(), 1);
    assert_eq!(safe[0].name, "Safe item");

    let caution = results.filter_by_safety(&targets::SafetyLevel::Caution);
    assert_eq!(caution.len(), 1);
    assert_eq!(caution[0].name, "Caution item");
}
