use std::path::{Path, PathBuf};

/// Well-known tracking/analytics domains
const KNOWN_TRACKERS: &[&str] = &[
    "doubleclick.net",
    "google-analytics.com",
    "googleadservices.com",
    "googlesyndication.com",
    "facebook.com",
    "facebook.net",
    "fbcdn.net",
    "analytics.twitter.com",
    "ads.twitter.com",
    "amazon-adsystem.com",
    "advertising.com",
    "adnxs.com",
    "adsrvr.org",
    "criteo.com",
    "criteo.net",
    "outbrain.com",
    "taboola.com",
    "hotjar.com",
    "mixpanel.com",
    "amplitude.com",
    "segment.com",
    "segment.io",
    "optimizely.com",
    "quantserve.com",
    "scorecardresearch.com",
    "chartbeat.com",
    "newrelic.com",
    "nr-data.net",
    "rubiconproject.com",
    "pubmatic.com",
    "openx.net",
    "casalemedia.com",
    "demdex.net",
    "bluekai.com",
    "krxd.net",
    "exelator.com",
    "turn.com",
    "mathtag.com",
    "rlcdn.com",
    "sharethis.com",
    "addthis.com",
    "appsflyer.com",
    "branch.io",
    "adjust.com",
    "mparticle.com",
    "braze.com",
    "tiktok.com",
    "bytedance.com",
    "snap.com",
    "snapchat.com",
];

/// Result of a privacy audit
#[derive(Debug, Clone)]
pub struct PrivacyReport {
    pub browser_profiles: Vec<super::browsers::BrowserProfile>,
    pub tracking_apps: Vec<TrackingApp>,
    pub total_privacy_data_size: u64,
    pub total_tracking_files: usize,
    pub cookie_locations: Vec<CookieLocation>,
}

/// An app that stores tracking/analytics data
#[derive(Debug, Clone)]
pub struct TrackingApp {
    pub name: String,
    pub path: PathBuf,
    pub data_size: u64,
    pub kind: TrackingKind,
}

#[derive(Debug, Clone)]
pub enum TrackingKind {
    Cookies,
    HttpStorage,
    WebData,
    LocalDatabase,
    AnalyticsCache,
}

impl std::fmt::Display for TrackingKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrackingKind::Cookies => write!(f, "Cookies"),
            TrackingKind::HttpStorage => write!(f, "HTTP Storage"),
            TrackingKind::WebData => write!(f, "Web Data"),
            TrackingKind::LocalDatabase => write!(f, "Local Database"),
            TrackingKind::AnalyticsCache => write!(f, "Analytics Cache"),
        }
    }
}

/// A cookie storage location found on the system
#[derive(Debug, Clone)]
pub struct CookieLocation {
    pub path: PathBuf,
    pub app_name: String,
    pub size: u64,
}

/// Run a full privacy audit
pub fn run_privacy_audit(scan_browsers: bool, scan_cookies: bool) -> PrivacyReport {
    let home = dirs::home_dir().unwrap_or_default();
    let mut report = PrivacyReport {
        browser_profiles: Vec::new(),
        tracking_apps: Vec::new(),
        total_privacy_data_size: 0,
        total_tracking_files: 0,
        cookie_locations: Vec::new(),
    };

    // Browser scan
    if scan_browsers {
        report.browser_profiles = super::browsers::discover_browsers();
        for profile in &report.browser_profiles {
            report.total_privacy_data_size += profile.total_size;
        }
    }

    // Cookie scan across all apps
    if scan_cookies {
        report.cookie_locations = scan_cookie_locations(&home);
        report.tracking_apps = scan_tracking_data(&home);
        for loc in &report.cookie_locations {
            report.total_tracking_files += 1;
            report.total_privacy_data_size += loc.size;
        }
        for app in &report.tracking_apps {
            report.total_privacy_data_size += app.data_size;
        }
    }

    report
}

/// Scan for cookie files across the system
fn scan_cookie_locations(home: &Path) -> Vec<CookieLocation> {
    let mut locations = Vec::new();

    // ~/Library/Cookies/
    let cookies_dir = home.join("Library/Cookies");
    if cookies_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&cookies_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_file() {
                    let name = path
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                    if size > 0 {
                        locations.push(CookieLocation {
                            path,
                            app_name: name,
                            size,
                        });
                    }
                }
            }
        }
    }

    // ~/Library/HTTPStorages/
    let http_dir = home.join("Library/HTTPStorages");
    if http_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&http_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_dir() {
                    let name = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    let size = crate::scanner::walker::dir_size(&path);
                    if size > 0 {
                        locations.push(CookieLocation {
                            path,
                            app_name: name,
                            size,
                        });
                    }
                }
            }
        }
    }

    locations.sort_by(|a, b| b.size.cmp(&a.size));
    locations
}

/// Scan for app-level tracking data
fn scan_tracking_data(home: &Path) -> Vec<TrackingApp> {
    let mut apps = Vec::new();

    // ~/Library/WebKit/
    let webkit_dir = home.join("Library/WebKit");
    if webkit_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&webkit_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                    let size = crate::scanner::walker::dir_size(&path);
                    if size > 0 {
                        apps.push(TrackingApp {
                            name,
                            path,
                            data_size: size,
                            kind: TrackingKind::WebData,
                        });
                    }
                }
            }
        }
    }

    // ~/Library/Saved Application State/ (tracks window positions, recent docs)
    let saved_state = home.join("Library/Saved Application State");
    if saved_state.exists() {
        let total_size = crate::scanner::walker::dir_size(&saved_state);
        let count = std::fs::read_dir(&saved_state)
            .map(|e| e.count())
            .unwrap_or(0);
        if total_size > 0 {
            apps.push(TrackingApp {
                name: format!("Saved App State ({} apps)", count),
                path: saved_state,
                data_size: total_size,
                kind: TrackingKind::AnalyticsCache,
            });
        }
    }

    apps.sort_by(|a, b| b.data_size.cmp(&a.data_size));
    apps
}

/// Check if a domain is a known tracker
pub fn is_known_tracker(domain: &str) -> bool {
    let lower = domain.to_lowercase();
    KNOWN_TRACKERS
        .iter()
        .any(|tracker| lower.contains(tracker))
}

/// Get the count of known trackers in the database
pub fn tracker_database_size() -> usize {
    KNOWN_TRACKERS.len()
}
