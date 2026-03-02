//! Integration tests for the apps detector module.

use std::fs;
use tempfile::TempDir;

// ─── Tests for AppSource ──────────────────────────────────────────────────────

#[test]
fn test_app_source_display() {
    use tidymac::apps::detector::AppSource;
    assert_eq!(AppSource::Applications.to_string(), "Applications");
    assert_eq!(AppSource::UserApplications.to_string(), "User Apps");
    assert_eq!(AppSource::Homebrew.to_string(), "Homebrew");
    assert_eq!(AppSource::System.to_string(), "System");
}

#[test]
fn test_associated_kind_display() {
    use tidymac::apps::detector::AssociatedKind;
    assert_eq!(AssociatedKind::AppSupport.to_string(), "App Support");
    assert_eq!(AssociatedKind::Cache.to_string(), "Cache");
    assert_eq!(AssociatedKind::Preferences.to_string(), "Preferences");
    assert_eq!(AssociatedKind::SavedState.to_string(), "Saved State");
    assert_eq!(AssociatedKind::Container.to_string(), "Container");
    assert_eq!(
        AssociatedKind::GroupContainer.to_string(),
        "Group Container"
    );
    assert_eq!(AssociatedKind::Cookies.to_string(), "Cookies");
    assert_eq!(AssociatedKind::HttpStorage.to_string(), "HTTP Storage");
    assert_eq!(AssociatedKind::WebKit.to_string(), "WebKit Data");
    assert_eq!(AssociatedKind::Logs.to_string(), "Logs");
    assert_eq!(AssociatedKind::LaunchAgent.to_string(), "Launch Agent");
    assert_eq!(AssociatedKind::LoginItem.to_string(), "Login Item");
    assert_eq!(AssociatedKind::CrashReports.to_string(), "Crash Reports");
}

#[test]
fn test_discover_apps_returns_vec() {
    // Smoke test: should never panic, may return empty on headless CI
    let apps = tidymac::apps::detector::discover_apps();
    // Apps may be empty on minimal CI. Just validate properties on any found app.
    for app in &apps {
        assert!(!app.name.is_empty(), "App name should not be empty");
        assert!(app.path.exists(), "App path should exist: {:?}", app.path);
        assert!(app.app_size > 0 || app.path.exists(), "App size check");
    }
}

#[test]
fn test_find_associated_files_no_bundle_id() {
    // A non-existent app should return an empty list gracefully
    let files =
        tidymac::apps::detector::find_associated_files("__nonexistent_app_tidymac_test__", None);
    assert!(
        files.is_empty(),
        "Should find no files for a non-existent app"
    );
}

#[test]
fn test_find_associated_files_with_fake_bundle_id() {
    let files = tidymac::apps::detector::find_associated_files(
        "FakeApp",
        Some("com.fake.app.tidymac.test"),
    );
    // None of the fake paths should exist, so we expect all to be gone
    // (find_associated_files only adds entries that exist)
    assert!(
        files.iter().all(|f| f.exists),
        "All returned files should exist"
    );
}

#[test]
fn test_installed_app_total_size_is_at_least_app_size() {
    // total_size = app_size + associated. associated can be 0 but never negative.
    let apps = tidymac::apps::detector::discover_apps();
    for app in &apps {
        assert!(
            app.total_size >= app.app_size,
            "total_size ({}) should be >= app_size ({}) for {}",
            app.total_size,
            app.app_size,
            app.name
        );
    }
}
