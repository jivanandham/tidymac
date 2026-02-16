use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;

// ─── Core types ───────────────────────────────────────────────────────────────

/// Safety level for a file deletion
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyLevel {
    /// Caches, temp files — always safe to remove
    Safe,
    /// Old logs, downloads — review recommended
    Caution,
    /// App support files — may break applications
    Dangerous,
}

/// Developer tool identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DevTool {
    Xcode,
    XcodeArchives,
    XcodeSimulators,
    Docker,
    NodeModules,
    Venv,
    Conda,
    Homebrew,
    Pip,
    CocoaPods,
    Gradle,
    Maven,
    Cargo,
    Npm,
    Yarn,
    Pnpm,
}

impl std::fmt::Display for DevTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DevTool::Xcode => write!(f, "Xcode DerivedData"),
            DevTool::XcodeArchives => write!(f, "Xcode Archives"),
            DevTool::XcodeSimulators => write!(f, "iOS Simulators"),
            DevTool::Docker => write!(f, "Docker"),
            DevTool::NodeModules => write!(f, "node_modules"),
            DevTool::Venv => write!(f, "Python virtualenv"),
            DevTool::Conda => write!(f, "Conda"),
            DevTool::Homebrew => write!(f, "Homebrew"),
            DevTool::Pip => write!(f, "pip"),
            DevTool::CocoaPods => write!(f, "CocoaPods"),
            DevTool::Gradle => write!(f, "Gradle"),
            DevTool::Maven => write!(f, "Maven"),
            DevTool::Cargo => write!(f, "Cargo"),
            DevTool::Npm => write!(f, "npm"),
            DevTool::Yarn => write!(f, "Yarn"),
            DevTool::Pnpm => write!(f, "pnpm"),
        }
    }
}

/// File category classification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    SystemCache,
    UserCache,
    Logs,
    TempFiles,
    CrashReports,
    DevCache(DevTool),
    LargeFile,
    Duplicate,
    MailAttachment,
    Trash,
    BrowserData,
    AppLeftover,
    StartupItem,
    DownloadedDmg,
    OldDownload,
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Category::SystemCache => write!(f, "System Cache"),
            Category::UserCache => write!(f, "User Cache"),
            Category::Logs => write!(f, "Logs"),
            Category::TempFiles => write!(f, "Temporary Files"),
            Category::CrashReports => write!(f, "Crash Reports"),
            Category::DevCache(tool) => write!(f, "Dev: {}", tool),
            Category::LargeFile => write!(f, "Large File"),
            Category::Duplicate => write!(f, "Duplicate"),
            Category::MailAttachment => write!(f, "Mail Attachment"),
            Category::Trash => write!(f, "Trash"),
            Category::BrowserData => write!(f, "Browser Data"),
            Category::AppLeftover => write!(f, "App Leftover"),
            Category::StartupItem => write!(f, "Startup Item"),
            Category::DownloadedDmg => write!(f, "Downloaded DMG"),
            Category::OldDownload => write!(f, "Old Download"),
        }
    }
}

/// A single scan result entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanItem {
    /// Display name for this item group
    pub name: String,

    /// Category of the item
    pub category: Category,

    /// Base path of the scan target
    pub path: PathBuf,

    /// Total size in bytes
    pub size_bytes: u64,

    /// Number of files found
    pub file_count: usize,

    /// Safety level for deletion
    pub safety: SafetyLevel,

    /// Human-readable reason this is flagged
    pub reason: String,

    /// Individual file paths (for detailed view)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<FileEntry>,
}

/// Individual file entry within a scan item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub modified: Option<SystemTime>,
}

/// Complete scan results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResults {
    /// When the scan was performed
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// How long the scan took in seconds
    pub duration_secs: f64,

    /// All scan items grouped by category
    pub items: Vec<ScanItem>,

    /// Total reclaimable space in bytes
    pub total_reclaimable: u64,

    /// Total files found
    pub total_files: usize,

    /// Errors encountered during scan
    pub errors: Vec<String>,
}

impl ScanResults {
    pub fn new() -> Self {
        Self {
            timestamp: chrono::Utc::now(),
            duration_secs: 0.0,
            items: Vec::new(),
            total_reclaimable: 0,
            total_files: 0,
            errors: Vec::new(),
        }
    }

    /// Recalculate totals from items
    pub fn recalculate(&mut self) {
        self.total_reclaimable = self.items.iter().map(|i| i.size_bytes).sum();
        self.total_files = self.items.iter().map(|i| i.file_count).sum();
    }

    /// Filter items by safety level
    pub fn filter_by_safety(&self, level: &SafetyLevel) -> Vec<&ScanItem> {
        self.items.iter().filter(|i| &i.safety == level).collect()
    }

    /// Get items grouped by category
    pub fn by_category(&self) -> std::collections::HashMap<String, Vec<&ScanItem>> {
        let mut map: std::collections::HashMap<String, Vec<&ScanItem>> =
            std::collections::HashMap::new();
        for item in &self.items {
            let key = format!("{}", item.category);
            map.entry(key).or_default().push(item);
        }
        map
    }
}

// ─── Scan target definitions ──────────────────────────────────────────────────

/// A scan target defines where to look and how to classify what's found
#[derive(Debug, Clone)]
pub struct ScanTarget {
    pub name: String,
    pub category: Category,
    pub paths: Vec<String>, // paths with ~ expansion
    pub safety: SafetyLevel,
    pub reason: String,
    pub recursive: bool,
    pub min_age_days: Option<u32>, // only flag if older than N days
}

/// Get all system junk scan targets
pub fn system_junk_targets() -> Vec<ScanTarget> {
    vec![
        ScanTarget {
            name: "User Cache Files".into(),
            category: Category::UserCache,
            paths: vec!["~/Library/Caches".into()],
            safety: SafetyLevel::Safe,
            reason: "Application caches that will be regenerated automatically".into(),
            recursive: true,
            min_age_days: None,
        },
        ScanTarget {
            name: "System Log Files".into(),
            category: Category::Logs,
            paths: vec!["/var/log".into()],
            safety: SafetyLevel::Caution,
            reason: "System logs — old entries are safe to remove".into(),
            recursive: true,
            min_age_days: Some(7),
        },
        ScanTarget {
            name: "User Log Files".into(),
            category: Category::Logs,
            paths: vec!["~/Library/Logs".into()],
            safety: SafetyLevel::Safe,
            reason: "Application logs that can be safely removed".into(),
            recursive: true,
            min_age_days: None,
        },
        ScanTarget {
            name: "Temporary Files".into(),
            category: Category::TempFiles,
            paths: vec!["/tmp".into(), "/var/folders".into()],
            safety: SafetyLevel::Safe,
            reason: "Temporary files created by the system and apps".into(),
            recursive: true,
            min_age_days: Some(1),
        },
        ScanTarget {
            name: "Crash Reports".into(),
            category: Category::CrashReports,
            paths: vec!["~/Library/Logs/DiagnosticReports".into()],
            safety: SafetyLevel::Safe,
            reason: "Application crash reports — safe to remove unless debugging".into(),
            recursive: true,
            min_age_days: None,
        },
        ScanTarget {
            name: "QuickLook Thumbnails".into(),
            category: Category::SystemCache,
            paths: vec![
                "~/Library/Caches/com.apple.QuickLook.thumbnailcache".into(),
            ],
            safety: SafetyLevel::Safe,
            reason: "Thumbnail preview caches — regenerated on demand".into(),
            recursive: true,
            min_age_days: None,
        },
        ScanTarget {
            name: "Downloaded DMG Files".into(),
            category: Category::DownloadedDmg,
            paths: vec!["~/Downloads".into()],
            safety: SafetyLevel::Caution,
            reason: "Installer disk images — usually safe to remove after installation".into(),
            recursive: false,
            min_age_days: Some(7),
        },
    ]
}

/// Get all developer tool scan targets
pub fn developer_targets() -> Vec<ScanTarget> {
    vec![
        ScanTarget {
            name: "Xcode DerivedData".into(),
            category: Category::DevCache(DevTool::Xcode),
            paths: vec!["~/Library/Developer/Xcode/DerivedData".into()],
            safety: SafetyLevel::Safe,
            reason: "Build artifacts that Xcode regenerates on next build".into(),
            recursive: true,
            min_age_days: None,
        },
        ScanTarget {
            name: "Xcode Archives".into(),
            category: Category::DevCache(DevTool::XcodeArchives),
            paths: vec!["~/Library/Developer/Xcode/Archives".into()],
            safety: SafetyLevel::Caution,
            reason: "App Store submission archives — keep if you need to debug shipped versions"
                .into(),
            recursive: true,
            min_age_days: Some(90),
        },
        ScanTarget {
            name: "iOS Simulators".into(),
            category: Category::DevCache(DevTool::XcodeSimulators),
            paths: vec!["~/Library/Developer/CoreSimulator/Devices".into()],
            safety: SafetyLevel::Caution,
            reason: "iOS simulator data — can be re-downloaded".into(),
            recursive: true,
            min_age_days: None,
        },
        ScanTarget {
            name: "Docker Data".into(),
            category: Category::DevCache(DevTool::Docker),
            paths: vec![
                "~/Library/Containers/com.docker.docker/Data".into(),
                "~/.docker".into(),
            ],
            safety: SafetyLevel::Caution,
            reason: "Docker images and volumes — use 'docker system prune' for granular control"
                .into(),
            recursive: true,
            min_age_days: None,
        },
        ScanTarget {
            name: "Homebrew Cache".into(),
            category: Category::DevCache(DevTool::Homebrew),
            paths: vec!["~/Library/Caches/Homebrew".into()],
            safety: SafetyLevel::Safe,
            reason: "Downloaded package archives — re-downloaded on demand".into(),
            recursive: true,
            min_age_days: None,
        },
        ScanTarget {
            name: "pip Cache".into(),
            category: Category::DevCache(DevTool::Pip),
            paths: vec!["~/Library/Caches/pip".into()],
            safety: SafetyLevel::Safe,
            reason: "Python package download cache — re-downloaded on demand".into(),
            recursive: true,
            min_age_days: None,
        },
        ScanTarget {
            name: "npm Cache".into(),
            category: Category::DevCache(DevTool::Npm),
            paths: vec!["~/.npm/_cacache".into()],
            safety: SafetyLevel::Safe,
            reason: "npm package cache — re-downloaded on demand".into(),
            recursive: true,
            min_age_days: None,
        },
        ScanTarget {
            name: "Yarn Cache".into(),
            category: Category::DevCache(DevTool::Yarn),
            paths: vec!["~/Library/Caches/Yarn".into()],
            safety: SafetyLevel::Safe,
            reason: "Yarn package cache — re-downloaded on demand".into(),
            recursive: true,
            min_age_days: None,
        },
        ScanTarget {
            name: "CocoaPods Cache".into(),
            category: Category::DevCache(DevTool::CocoaPods),
            paths: vec!["~/Library/Caches/CocoaPods".into()],
            safety: SafetyLevel::Safe,
            reason: "CocoaPods spec and download cache".into(),
            recursive: true,
            min_age_days: None,
        },
        ScanTarget {
            name: "Cargo Registry Cache".into(),
            category: Category::DevCache(DevTool::Cargo),
            paths: vec![
                "~/.cargo/registry/cache".into(),
                "~/.cargo/registry/src".into(),
            ],
            safety: SafetyLevel::Safe,
            reason: "Rust crate download cache — re-downloaded on demand".into(),
            recursive: true,
            min_age_days: None,
        },
        ScanTarget {
            name: "Gradle Cache".into(),
            category: Category::DevCache(DevTool::Gradle),
            paths: vec!["~/.gradle/caches".into()],
            safety: SafetyLevel::Safe,
            reason: "Gradle build cache and dependency downloads".into(),
            recursive: true,
            min_age_days: None,
        },
        ScanTarget {
            name: "Maven Local Repository".into(),
            category: Category::DevCache(DevTool::Maven),
            paths: vec!["~/.m2/repository".into()],
            safety: SafetyLevel::Caution,
            reason: "Maven dependency cache — may include locally installed artifacts".into(),
            recursive: true,
            min_age_days: None,
        },
        ScanTarget {
            name: "Conda Package Cache".into(),
            category: Category::DevCache(DevTool::Conda),
            paths: vec!["~/.conda/pkgs".into()],
            safety: SafetyLevel::Safe,
            reason: "Conda downloaded packages — re-downloaded on demand".into(),
            recursive: true,
            min_age_days: None,
        },
    ]
}

/// Get trash scan targets
pub fn trash_targets() -> Vec<ScanTarget> {
    vec![
        ScanTarget {
            name: "User Trash".into(),
            category: Category::Trash,
            paths: vec!["~/.Trash".into()],
            safety: SafetyLevel::Safe,
            reason: "Files in your trash bin".into(),
            recursive: true,
            min_age_days: None,
        },
        ScanTarget {
            name: "External Drive Trash".into(),
            category: Category::Trash,
            paths: vec!["/Volumes/*/.Trashes".into()],
            safety: SafetyLevel::Safe,
            reason: "Trash from external drives".into(),
            recursive: true,
            min_age_days: None,
        },
    ]
}

/// Get mail attachment targets
pub fn mail_targets() -> Vec<ScanTarget> {
    vec![
        ScanTarget {
            name: "Mail Downloads".into(),
            category: Category::MailAttachment,
            paths: vec!["~/Library/Mail Downloads".into()],
            safety: SafetyLevel::Safe,
            reason: "Cached mail attachments — re-downloaded from mail server".into(),
            recursive: true,
            min_age_days: None,
        },
        ScanTarget {
            name: "Mail Container Data".into(),
            category: Category::MailAttachment,
            paths: vec![
                "~/Library/Containers/com.apple.mail/Data/Library/Mail Downloads".into(),
            ],
            safety: SafetyLevel::Safe,
            reason: "Sandboxed mail attachment cache".into(),
            recursive: true,
            min_age_days: None,
        },
    ]
}

/// Get all default scan targets
pub fn all_targets() -> Vec<ScanTarget> {
    let mut targets = Vec::new();
    targets.extend(system_junk_targets());
    targets.extend(developer_targets());
    targets.extend(trash_targets());
    targets.extend(mail_targets());
    targets
}
