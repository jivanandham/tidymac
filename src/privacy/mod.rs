pub mod browsers;
pub mod cleaner;
pub mod trackers;

pub use browsers::{discover_browsers, BrowserProfile, BrowserType};
pub use cleaner::{clean_privacy, PrivacyCleanOptions};
pub use trackers::{
    is_known_tracker, run_privacy_audit, tracker_database_size, CookieLocation, PrivacyReport,
    TrackingApp, TrackingKind,
};
