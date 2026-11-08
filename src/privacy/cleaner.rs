//! Privacy cleanup through the unified deleter.
//!
//! * Browser data is skipped while that browser is running (deleting SQLite
//!   cookie databases under a live browser can corrupt the profile).
//! * Browsing history is never deleted by TidyMac.
//! * Everything is staged for undo by default.

use anyhow::Result;

use super::trackers::PrivacyReport;
use crate::cleaner::{self, CleanMode, CleanReport, DeleteOptions, DeleteRequest, SkippedEntry};
use crate::common::running::RunningApps;
use crate::common::safety::DeletePolicy;

/// What to clean.
#[derive(Debug, Clone)]
pub struct PrivacyCleanOptions {
    pub mode: CleanMode,
    /// Browser caches.
    pub caches: bool,
    /// Browser cookie databases (signs you out of websites).
    pub browser_cookies: bool,
    /// Per-app cookie files and HTTP storage in ~/Library (signs you out of apps).
    pub app_cookies: bool,
}

impl Default for PrivacyCleanOptions {
    fn default() -> Self {
        Self {
            mode: CleanMode::SoftDelete,
            caches: true,
            browser_cookies: false,
            app_cookies: false,
        }
    }
}

/// Clean the data found by a privacy audit.
pub fn clean_privacy(report: &PrivacyReport, opts: &PrivacyCleanOptions) -> Result<CleanReport> {
    let running = RunningApps::snapshot();
    let library = dirs::home_dir().unwrap_or_default().join("Library");
    let mut requests = Vec::new();
    let mut skipped = Vec::new();

    for profile in &report.browser_profiles {
        let browser_running = running.is_bundle_running(profile.browser.bundle_id());
        let mut add = |path: &Option<std::path::PathBuf>, size: u64, what: &str| {
            if let Some(p) = path {
                if browser_running {
                    skipped.push(SkippedEntry {
                        path: p.clone(),
                        reason: format!("{} is running — quit it to clean its {}", profile.browser, what),
                    });
                } else {
                    requests.push(DeleteRequest {
                        path: p.clone(),
                        size_bytes: size,
                        category: format!("Browser {}: {}", what, profile.browser),
                        safety: "Caution".into(),
                        roots: vec![library.clone()],
                        policy: DeletePolicy::AppData,
                    });
                }
            }
        };
        if opts.caches {
            add(&profile.cache_path, profile.cache_size, "cache");
        }
        if opts.browser_cookies {
            add(&profile.cookies_path, profile.cookies_size, "cookies");
        }
    }

    if opts.app_cookies {
        for loc in &report.cookie_locations {
            requests.push(DeleteRequest {
                path: loc.path.clone(),
                size_bytes: loc.size,
                category: format!("App cookies: {}", loc.app_name),
                safety: "Caution".into(),
                roots: vec![library.clone()],
                policy: DeletePolicy::AppData,
            });
        }
    }

    let options = DeleteOptions::new(opts.mode.clone(), "privacy");
    let mut result = cleaner::execute(requests, Vec::new(), &options)?;
    skipped.append(&mut result.skipped);
    result.skipped = skipped;
    Ok(result)
}
