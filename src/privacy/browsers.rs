use std::path::{Path, PathBuf};

/// Information about a browser's privacy-relevant data
#[derive(Debug, Clone)]
pub struct BrowserProfile {
    pub browser: BrowserType,
    pub profile_path: PathBuf,
    pub cookies_path: Option<PathBuf>,
    pub cookies_size: u64,
    pub history_path: Option<PathBuf>,
    pub history_size: u64,
    pub local_storage_path: Option<PathBuf>,
    pub local_storage_size: u64,
    pub cache_path: Option<PathBuf>,
    pub cache_size: u64,
    pub extensions_path: Option<PathBuf>,
    pub extensions_size: u64,
    pub total_size: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BrowserType {
    Chrome,
    ChromeCanary,
    Chromium,
    Brave,
    Edge,
    Firefox,
    Safari,
    Arc,
    Vivaldi,
    Opera,
}

impl std::fmt::Display for BrowserType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrowserType::Chrome => write!(f, "Google Chrome"),
            BrowserType::ChromeCanary => write!(f, "Chrome Canary"),
            BrowserType::Chromium => write!(f, "Chromium"),
            BrowserType::Brave => write!(f, "Brave"),
            BrowserType::Edge => write!(f, "Microsoft Edge"),
            BrowserType::Firefox => write!(f, "Firefox"),
            BrowserType::Safari => write!(f, "Safari"),
            BrowserType::Arc => write!(f, "Arc"),
            BrowserType::Vivaldi => write!(f, "Vivaldi"),
            BrowserType::Opera => write!(f, "Opera"),
        }
    }
}

/// Chromium-based browser locations relative to ~/Library/Application Support/
const CHROMIUM_BROWSERS: &[(&str, BrowserType)] = &[
    ("Google/Chrome", BrowserType::Chrome),
    ("Google/Chrome Canary", BrowserType::ChromeCanary),
    ("Chromium", BrowserType::Chromium),
    ("BraveSoftware/Brave-Browser", BrowserType::Brave),
    ("Microsoft Edge", BrowserType::Edge),
    ("Arc/User Data", BrowserType::Arc),
    ("Vivaldi", BrowserType::Vivaldi),
    ("com.operasoftware.Opera", BrowserType::Opera),
];

/// Discover all browser profiles and their privacy data
pub fn discover_browsers() -> Vec<BrowserProfile> {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return Vec::new(),
    };
    let mut profiles = Vec::new();

    // Chromium-based browsers
    let app_support = home.join("Library/Application Support");
    for (rel_path, browser_type) in CHROMIUM_BROWSERS {
        let base = app_support.join(rel_path);
        if !base.exists() {
            continue;
        }

        // Check Default profile and any numbered profiles
        let mut profile_dirs = vec![base.join("Default")];
        if let Ok(entries) = std::fs::read_dir(&base) {
            for entry in entries.filter_map(|e| e.ok()) {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("Profile ") {
                    profile_dirs.push(entry.path());
                }
            }
        }

        for profile_dir in profile_dirs {
            if !profile_dir.exists() {
                continue;
            }
            if let Some(profile) = scan_chromium_profile(&profile_dir, browser_type.clone()) {
                profiles.push(profile);
            }
        }
    }

    // Firefox
    let firefox_base = app_support.join("Firefox/Profiles");
    if firefox_base.exists() {
        if let Ok(entries) = std::fs::read_dir(&firefox_base) {
            for entry in entries.filter_map(|e| e.ok()) {
                if entry.path().is_dir() {
                    if let Some(profile) = scan_firefox_profile(&entry.path()) {
                        profiles.push(profile);
                    }
                }
            }
        }
    }

    // Safari
    let safari_dir = home.join("Library/Safari");
    if safari_dir.exists() {
        if let Some(profile) = scan_safari(&home) {
            profiles.push(profile);
        }
    }

    profiles
}

/// Scan a Chromium-based browser profile
fn scan_chromium_profile(profile_dir: &Path, browser: BrowserType) -> Option<BrowserProfile> {
    let cookies = profile_dir.join("Cookies");
    let history = profile_dir.join("History");
    let local_storage = profile_dir.join("Local Storage");
    let cache = profile_dir.join("Cache");
    let extensions = profile_dir.join("Extensions");

    let cookies_size = file_or_dir_size(&cookies);
    let history_size = file_or_dir_size(&history);
    let local_storage_size = file_or_dir_size(&local_storage);
    let cache_size = file_or_dir_size(&cache);
    let extensions_size = file_or_dir_size(&extensions);

    let total = cookies_size + history_size + local_storage_size + cache_size + extensions_size;

    if total == 0 {
        return None;
    }

    Some(BrowserProfile {
        browser,
        profile_path: profile_dir.to_path_buf(),
        cookies_path: exists_opt(&cookies),
        cookies_size,
        history_path: exists_opt(&history),
        history_size,
        local_storage_path: exists_opt(&local_storage),
        local_storage_size,
        cache_path: exists_opt(&cache),
        cache_size,
        extensions_path: exists_opt(&extensions),
        extensions_size,
        total_size: total,
    })
}

/// Scan a Firefox profile directory
fn scan_firefox_profile(profile_dir: &Path) -> Option<BrowserProfile> {
    let cookies = profile_dir.join("cookies.sqlite");
    let history = profile_dir.join("places.sqlite");
    let local_storage = profile_dir.join("webappsstore.sqlite");
    let cache = profile_dir.join("cache2");
    let extensions = profile_dir.join("extensions");

    let cookies_size = file_or_dir_size(&cookies);
    let history_size = file_or_dir_size(&history);
    let local_storage_size = file_or_dir_size(&local_storage);
    let cache_size = file_or_dir_size(&cache);
    let extensions_size = file_or_dir_size(&extensions);

    let total = cookies_size + history_size + local_storage_size + cache_size + extensions_size;

    if total == 0 {
        return None;
    }

    Some(BrowserProfile {
        browser: BrowserType::Firefox,
        profile_path: profile_dir.to_path_buf(),
        cookies_path: exists_opt(&cookies),
        cookies_size,
        history_path: exists_opt(&history),
        history_size,
        local_storage_path: exists_opt(&local_storage),
        local_storage_size,
        cache_path: exists_opt(&cache),
        cache_size,
        extensions_path: exists_opt(&extensions),
        extensions_size,
        total_size: total,
    })
}

/// Scan Safari data
fn scan_safari(home: &Path) -> Option<BrowserProfile> {
    let safari_dir = home.join("Library/Safari");
    let history = safari_dir.join("History.db");
    let local_storage = safari_dir.join("LocalStorage");
    let cache = home.join("Library/Caches/com.apple.Safari");
    let cookies = home.join("Library/Cookies/Cookies.binarycookies");

    let cookies_size = file_or_dir_size(&cookies);
    let history_size = file_or_dir_size(&history);
    let local_storage_size = file_or_dir_size(&local_storage);
    let cache_size = file_or_dir_size(&cache);

    let total = cookies_size + history_size + local_storage_size + cache_size;

    if total == 0 {
        return None;
    }

    Some(BrowserProfile {
        browser: BrowserType::Safari,
        profile_path: safari_dir,
        cookies_path: exists_opt(&cookies),
        cookies_size,
        history_path: exists_opt(&history),
        history_size,
        local_storage_path: exists_opt(&local_storage),
        local_storage_size,
        cache_path: exists_opt(&cache),
        cache_size,
        extensions_path: None,
        extensions_size: 0,
        total_size: total,
    })
}

fn file_or_dir_size(path: &Path) -> u64 {
    if !path.exists() {
        return 0;
    }
    if path.is_dir() {
        crate::scanner::walker::dir_size(path)
    } else {
        std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
    }
}

fn exists_opt(path: &Path) -> Option<PathBuf> {
    if path.exists() {
        Some(path.to_path_buf())
    } else {
        None
    }
}
