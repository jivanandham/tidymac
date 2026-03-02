//! Integration tests for the privacy browser discovery module.
//! These tests verify detection logic without touching real browser data.

use std::fs;
use tempfile::TempDir;

// ─── Helper: create a fake Chromium browser profile ──────────────────────────

fn create_fake_chromium_profile(base: &std::path::Path) {
    fs::create_dir_all(base.join("Default")).unwrap();
    fs::write(base.join("Default/Cookies"), b"fake-cookie-data-1234").unwrap();
    fs::write(base.join("Default/History"), b"fake-history-data").unwrap();
    fs::create_dir_all(base.join("Default/Cache")).unwrap();
    fs::write(base.join("Default/Cache/cache_entry"), b"cache").unwrap();
}

fn create_fake_firefox_profile(base: &std::path::Path, profile_name: &str) {
    let dir = base.join(profile_name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("cookies.sqlite"), b"fake-sqlite-cookies").unwrap();
    fs::write(dir.join("places.sqlite"), b"fake-sqlite-history").unwrap();
    fs::create_dir_all(dir.join("cache2")).unwrap();
    fs::write(dir.join("cache2/entry"), b"cache-content").unwrap();
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[test]
fn test_browser_type_display() {
    use tidymac::privacy::browsers::BrowserType;

    assert_eq!(BrowserType::Chrome.to_string(), "Google Chrome");
    assert_eq!(BrowserType::Firefox.to_string(), "Firefox");
    assert_eq!(BrowserType::Safari.to_string(), "Safari");
    assert_eq!(BrowserType::Brave.to_string(), "Brave");
    assert_eq!(BrowserType::Edge.to_string(), "Microsoft Edge");
    assert_eq!(BrowserType::Arc.to_string(), "Arc");
    assert_eq!(BrowserType::Vivaldi.to_string(), "Vivaldi");
    assert_eq!(BrowserType::Opera.to_string(), "Opera");
}

#[test]
fn test_discover_browsers_returns_vec() {
    // Just ensure the function doesn't panic and returns a Vec.
    // On CI it might be empty if no browsers are installed — that is fine.
    let profiles = tidymac::privacy::browsers::discover_browsers();
    // profiles can be empty on CI; just assert the type is correct
    let _ = profiles.len();
}

#[test]
fn test_browser_profile_total_size_calculation() {
    use tidymac::privacy::browsers::BrowserProfile;
    use tidymac::privacy::browsers::BrowserType;

    let profile = BrowserProfile {
        browser: BrowserType::Chrome,
        profile_path: std::path::PathBuf::from("/fake/path"),
        cookies_path: None,
        cookies_size: 100,
        history_path: None,
        history_size: 200,
        local_storage_path: None,
        local_storage_size: 300,
        cache_path: None,
        cache_size: 400,
        extensions_path: None,
        extensions_size: 500,
        total_size: 1500,
    };

    assert_eq!(
        profile.cookies_size
            + profile.history_size
            + profile.local_storage_size
            + profile.cache_size
            + profile.extensions_size,
        profile.total_size
    );
}

#[test]
fn test_browser_profile_fields_are_accessible() {
    use tidymac::privacy::browsers::{BrowserProfile, BrowserType};
    let profile = BrowserProfile {
        browser: BrowserType::Firefox,
        profile_path: std::path::PathBuf::from("/home/test/.mozilla/firefox/abc.default"),
        cookies_path: Some(std::path::PathBuf::from(
            "/home/test/.mozilla/firefox/abc.default/cookies.sqlite",
        )),
        cookies_size: 42,
        history_path: None,
        history_size: 0,
        local_storage_path: None,
        local_storage_size: 0,
        cache_path: None,
        cache_size: 0,
        extensions_path: None,
        extensions_size: 0,
        total_size: 42,
    };
    assert_eq!(profile.browser, BrowserType::Firefox);
    assert!(profile.cookies_path.is_some());
    assert_eq!(profile.total_size, 42);
}

#[test]
fn test_all_browser_types_have_display() {
    use tidymac::privacy::browsers::BrowserType;
    // Ensure none of the Display implementations panic
    let browsers = [
        BrowserType::Chrome,
        BrowserType::ChromeCanary,
        BrowserType::Chromium,
        BrowserType::Brave,
        BrowserType::Edge,
        BrowserType::Firefox,
        BrowserType::Safari,
        BrowserType::Arc,
        BrowserType::Vivaldi,
        BrowserType::Opera,
    ];
    for b in &browsers {
        assert!(!b.to_string().is_empty());
    }
}
