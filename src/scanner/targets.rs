use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;

// ─── Core types ───────────────────────────────────────────────────────────────

/// Safety level for a file deletion
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyLevel {
    /// Caches, temp files — always safe to remove
    Safe,
    /// Old logs, downloads — review recommended
    #[default]
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
    CargoTarget,
    Npm,
    Sbt,
    Yarn,
    YarnBerry,
    Pnpm,
    Go,
    RubyGems,
    Terraform,
    Ivy2,
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
            DevTool::CargoTarget => write!(f, "Cargo target/ dirs"),
            DevTool::Npm => write!(f, "npm"),
            DevTool::Sbt => write!(f, "sbt"),
            DevTool::Yarn => write!(f, "Yarn Classic"),
            DevTool::YarnBerry => write!(f, "Yarn Berry"),
            DevTool::Pnpm => write!(f, "pnpm"),
            DevTool::Go => write!(f, "Go module cache"),
            DevTool::RubyGems => write!(f, "Ruby Gems"),
            DevTool::Terraform => write!(f, "Terraform plugins"),
            DevTool::Ivy2 => write!(f, "JVM / Ivy2 cache"),
        }
    }
}

/// File category classification
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    SystemCache,
    #[default]
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

/// A command-line tool that performs a cleanup safely on our behalf.
///
/// Some data must be removed through its owning tool so the tool's own
/// bookkeeping stays consistent (simulator device sets, Docker's VM disk,
/// Go's read-only module cache). These actions can't be staged for undo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolAction {
    /// `docker container/image/volume/builder prune -f`
    DockerPrune,
    /// `xcrun simctl delete unavailable`
    SimctlDeleteUnavailable,
    /// `xcrun simctl --set previews delete all`
    SimctlDeletePreviews,
    /// `go clean -modcache`
    GoCleanModcache,
}

impl ToolAction {
    pub fn command_line(&self) -> &'static str {
        match self {
            ToolAction::DockerPrune => "docker container/image/volume/builder prune -f",
            ToolAction::SimctlDeleteUnavailable => "xcrun simctl delete unavailable",
            ToolAction::SimctlDeletePreviews => "xcrun simctl --set previews delete all",
            ToolAction::GoCleanModcache => "go clean -modcache",
        }
    }
}

/// A single scan result entry
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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

    /// Entries that would be removed. Each entry is a direct child of one of
    /// `roots` (a cache folder, a trashed item, a log file…) — never a root itself.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<FileEntry>,

    /// The scanned locations. Every entry must lie strictly inside one of them.
    #[serde(default)]
    pub roots: Vec<PathBuf>,

    /// Bytes deleting the entries would actually free (APFS clones counted once).
    #[serde(default)]
    pub reclaimable_bytes: u64,

    /// If set, cleaning runs this tool instead of deleting files.
    #[serde(default)]
    pub tool_action: Option<ToolAction>,

    /// Shown for review but never removed by a bulk clean (e.g. large files).
    #[serde(default)]
    pub review_only: bool,

    /// Deletion policy the safety guard applies to this item's entries.
    #[serde(default)]
    pub policy: crate::common::safety::DeletePolicy,

    /// Items rebuilt from the scan cache have no entry list and can't be cleaned
    /// until a fresh scan.
    #[serde(default)]
    pub from_cache: bool,
}

impl ScanItem {
    /// Is there anything this item could actually clean?
    pub fn is_cleanable(&self) -> bool {
        !self.review_only && !self.from_cache && (self.tool_action.is_some() || !self.files.is_empty())
    }
}

/// Individual file entry within a scan item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub modified: Option<SystemTime>,
}

/// Complete scan results
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
            ..Default::default()
        }
    }

    /// Recalculate totals from items. Review-only items (large files) are
    /// listed but not counted as reclaimable.
    pub fn recalculate(&mut self) {
        self.total_reclaimable = self
            .items
            .iter()
            .filter(|i| !i.review_only)
            .map(|i| i.size_bytes)
            .sum();
        self.total_files = self
            .items
            .iter()
            .filter(|i| !i.review_only)
            .map(|i| i.file_count)
            .sum();
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

/// A scan target defines where to look and how to classify what's found.
///
/// The scanner lists the **direct children** of each path and reports each
/// child (file or folder) as one cleanable entry, with its total size and the
/// newest modification time found inside it. Roots themselves are never removed.
#[derive(Debug, Clone, Default)]
pub struct ScanTarget {
    pub name: String,
    pub category: Category,
    /// Paths with `~` expansion, globs, and `{uid}` substitution.
    pub paths: Vec<String>,
    pub safety: SafetyLevel,
    pub reason: String,
    /// `false` → only direct child *files* are considered (no folders).
    pub recursive: bool,
    /// Only flag entries whose newest content is older than N days.
    pub min_age_days: Option<u32>,
    /// Only flag entries at least this large.
    pub min_size_bytes: Option<u64>,
    /// Child names to skip (case-insensitive exact match, `prefix*` allowed).
    pub exclude: Vec<String>,
    /// If non-empty, only files with these extensions (lower-case, no dot).
    pub extensions: Vec<String>,
    /// Run a tool instead of deleting files.
    pub tool_action: Option<ToolAction>,
    /// Deletion policy for this target's entries.
    pub policy: crate::common::safety::DeletePolicy,
}

impl ScanTarget {
    fn simple(
        name: &str,
        category: Category,
        paths: &[&str],
        safety: SafetyLevel,
        reason: &str,
    ) -> Self {
        Self {
            name: name.into(),
            category,
            paths: paths.iter().map(|p| p.to_string()).collect(),
            safety,
            reason: reason.into(),
            recursive: true,
            ..Default::default()
        }
    }

    fn min_age(mut self, days: u32) -> Self {
        self.min_age_days = Some(days);
        self
    }

    fn exclude(mut self, names: &[&str]) -> Self {
        self.exclude = names.iter().map(|n| n.to_string()).collect();
        self
    }

    fn tool(mut self, action: ToolAction) -> Self {
        self.tool_action = Some(action);
        self
    }
}

/// Cache folders that are either in active use by macOS services or are
/// expensive to rebuild. We never offer to clear these.
const USER_CACHE_EXCLUDES: &[&str] = &[
    "com.apple.bird",
    "com.apple.nsurlsessiond",
    "com.apple.containermanagerd",
    "com.apple.ap.adprivacyd",
    "com.apple.homed",
    "com.apple.HomeKit",
    "CloudKit",
    "FamilyCircle",
    "com.apple.amsengagementd",
    "com.apple.akd",
    "com.apple.Safari.SafeBrowsing",
    "com.apple.iCloudHelper",
    "TidyMac",
];

/// Get system junk scan targets
pub fn system_junk_targets() -> Vec<ScanTarget> {
    vec![
        ScanTarget::simple(
            "User Cache Files",
            Category::UserCache,
            &["~/Library/Caches"],
            SafetyLevel::Safe,
            "App caches — rebuilt automatically when the app needs them",
        )
        .exclude(USER_CACHE_EXCLUDES),
        ScanTarget::simple(
            "User Log Files",
            Category::Logs,
            &["~/Library/Logs"],
            SafetyLevel::Safe,
            "Application logs older than 7 days",
        )
        .min_age(7)
        .exclude(&["DiagnosticReports", "TidyMac"]),
        ScanTarget::simple(
            "Temporary Files",
            Category::TempFiles,
            &["{user_tmp}"],
            SafetyLevel::Caution,
            "Your temporary folder ($TMPDIR) — only items untouched for 3+ days",
        )
        .min_age(3)
        .exclude(&["TemporaryItems", "com.apple.*", "Cleanup At Startup"]),
        ScanTarget::simple(
            "Crash Reports",
            Category::CrashReports,
            &["~/Library/Logs/DiagnosticReports"],
            SafetyLevel::Safe,
            "Crash and diagnostic reports",
        ),
        ScanTarget::simple(
            "QuickLook Thumbnails",
            Category::SystemCache,
            &["~/Library/Caches/com.apple.QuickLook.thumbnailcache"],
            SafetyLevel::Safe,
            "Thumbnail previews — regenerated on demand",
        ),
        ScanTarget {
            name: "Downloaded Installers".into(),
            category: Category::DownloadedDmg,
            paths: vec!["~/Downloads".into()],
            safety: SafetyLevel::Caution,
            reason: "Disk images and installer packages older than 7 days".into(),
            recursive: false,
            min_age_days: Some(7),
            extensions: vec!["dmg".into(), "pkg".into(), "mpkg".into(), "iso".into()],
            ..Default::default()
        },
    ]
}

/// Get developer tool cache targets
pub fn developer_targets() -> Vec<ScanTarget> {
    vec![
        ScanTarget::simple(
            "Xcode DerivedData",
            Category::DevCache(DevTool::Xcode),
            &["~/Library/Developer/Xcode/DerivedData"],
            SafetyLevel::Safe,
            "Build products and indexes — rebuilt on next build",
        ),
        ScanTarget::simple(
            "Xcode Archives",
            Category::DevCache(DevTool::XcodeArchives),
            &["~/Library/Developer/Xcode/Archives"],
            SafetyLevel::Caution,
            "App archives older than 90 days — needed to symbolicate old releases",
        )
        .min_age(90),
        ScanTarget::simple(
            "Xcode Device Support",
            Category::DevCache(DevTool::Xcode),
            &[
                "~/Library/Developer/Xcode/iOS DeviceSupport",
                "~/Library/Developer/Xcode/watchOS DeviceSupport",
                "~/Library/Developer/Xcode/tvOS DeviceSupport",
            ],
            SafetyLevel::Safe,
            "Debug symbols for device OS versions not used in 90 days — re-copied when a device connects",
        )
        .min_age(90),
        ScanTarget::simple(
            "Unavailable Simulators",
            Category::DevCache(DevTool::XcodeSimulators),
            &["~/Library/Developer/CoreSimulator/Devices"],
            SafetyLevel::Safe,
            "Simulators for runtimes no longer installed (removed with simctl)",
        )
        .tool(ToolAction::SimctlDeleteUnavailable),
        ScanTarget::simple(
            "SwiftUI Preview Simulators",
            Category::DevCache(DevTool::XcodeSimulators),
            &["~/Library/Developer/Xcode/UserData/Previews/Simulator Devices"],
            SafetyLevel::Safe,
            "Preview devices — recreated when you open a preview (removed with simctl)",
        )
        .tool(ToolAction::SimctlDeletePreviews),
        ScanTarget::simple(
            "Docker (dangling data)",
            Category::DevCache(DevTool::Docker),
            &[],
            SafetyLevel::Caution,
            "Stopped containers, dangling images, unused anonymous volumes and build cache (via docker prune)",
        )
        .tool(ToolAction::DockerPrune),
        ScanTarget::simple(
            "Homebrew Cache",
            Category::DevCache(DevTool::Homebrew),
            &["~/Library/Caches/Homebrew"],
            SafetyLevel::Safe,
            "Downloaded bottles and source archives",
        ),
        ScanTarget::simple(
            "pip Cache",
            Category::DevCache(DevTool::Pip),
            &["~/Library/Caches/pip"],
            SafetyLevel::Safe,
            "Python package download cache",
        ),
        ScanTarget::simple(
            "npm Cache",
            Category::DevCache(DevTool::Npm),
            &["~/.npm/_cacache"],
            SafetyLevel::Safe,
            "npm package cache — re-downloaded on install",
        ),
        ScanTarget::simple(
            "Yarn Cache",
            Category::DevCache(DevTool::Yarn),
            &["~/Library/Caches/Yarn"],
            SafetyLevel::Safe,
            "Yarn Classic package cache",
        ),
        ScanTarget::simple(
            "CocoaPods Cache",
            Category::DevCache(DevTool::CocoaPods),
            &["~/Library/Caches/CocoaPods"],
            SafetyLevel::Safe,
            "CocoaPods spec and pod cache",
        ),
        ScanTarget::simple(
            "Cargo Registry Cache",
            Category::DevCache(DevTool::Cargo),
            &["~/.cargo/registry/cache", "~/.cargo/registry/src"],
            SafetyLevel::Safe,
            "Downloaded and extracted crates — re-fetched on build",
        ),
        ScanTarget::simple(
            "Gradle Cache",
            Category::DevCache(DevTool::Gradle),
            &["~/.gradle/caches"],
            SafetyLevel::Safe,
            "Gradle dependency and build cache",
        ),
        ScanTarget::simple(
            "Maven Local Repository",
            Category::DevCache(DevTool::Maven),
            &["~/.m2/repository"],
            SafetyLevel::Caution,
            "Maven artifacts — may include locally installed snapshots",
        ),
        ScanTarget::simple(
            "Conda Package Cache",
            Category::DevCache(DevTool::Conda),
            &["~/.conda/pkgs"],
            SafetyLevel::Safe,
            "Conda package tarballs",
        ),
        ScanTarget::simple(
            "pnpm Store",
            Category::DevCache(DevTool::Pnpm),
            &["~/Library/pnpm/store", "~/.local/share/pnpm/store"],
            SafetyLevel::Safe,
            "pnpm content-addressable store",
        ),
        ScanTarget::simple(
            "Yarn Berry Cache",
            Category::DevCache(DevTool::YarnBerry),
            &["~/.yarn/berry/cache"],
            SafetyLevel::Safe,
            "Yarn 2+ global cache",
        ),
        ScanTarget::simple(
            "Go Module Cache",
            Category::DevCache(DevTool::Go),
            &["~/go/pkg/mod"],
            SafetyLevel::Safe,
            "Go module cache (read-only files; removed with go clean -modcache)",
        )
        .tool(ToolAction::GoCleanModcache),
        ScanTarget::simple(
            "Ruby Gems Cache",
            Category::DevCache(DevTool::RubyGems),
            &["~/.gem/ruby/*/cache", "~/.local/share/gem/ruby/*/cache"],
            SafetyLevel::Safe,
            "Downloaded .gem files (installed gems are kept)",
        ),
        ScanTarget::simple(
            "Terraform Plugin Cache",
            Category::DevCache(DevTool::Terraform),
            &["~/.terraform.d/plugin-cache"],
            SafetyLevel::Safe,
            "Cached provider plugins",
        ),
        ScanTarget::simple(
            "Ivy / Coursier Cache",
            Category::DevCache(DevTool::Ivy2),
            &["~/.ivy2/cache", "~/Library/Caches/Coursier"],
            SafetyLevel::Safe,
            "JVM dependency caches",
        ),
        ScanTarget::simple(
            "sbt Boot Cache",
            Category::DevCache(DevTool::Sbt),
            &["~/.sbt/boot"],
            SafetyLevel::Safe,
            "sbt launcher cache (your ~/.sbt settings and plugins are kept)",
        ),
    ]
}

/// Get trash scan targets
pub fn trash_targets() -> Vec<ScanTarget> {
    vec![
        ScanTarget::simple(
            "User Trash",
            Category::Trash,
            &["~/.Trash"],
            SafetyLevel::Safe,
            "Items in your Trash",
        ),
        ScanTarget::simple(
            "External Drive Trash",
            Category::Trash,
            &["/Volumes/*/.Trashes/{uid}"],
            SafetyLevel::Safe,
            "Your trashed items on external drives (permanent delete only)",
        ),
    ]
}

/// Get mail attachment targets
pub fn mail_targets() -> Vec<ScanTarget> {
    vec![
        ScanTarget::simple(
            "Mail Downloads",
            Category::MailAttachment,
            &["~/Library/Mail Downloads"],
            SafetyLevel::Safe,
            "Opened mail attachments — re-downloaded from the mail server",
        ),
        ScanTarget::simple(
            "Mail Container Data",
            Category::MailAttachment,
            &["~/Library/Containers/com.apple.mail/Data/Library/Mail Downloads"],
            SafetyLevel::Safe,
            "Sandboxed mail attachment cache",
        ),
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
