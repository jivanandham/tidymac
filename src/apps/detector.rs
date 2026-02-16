use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Source of application installation
#[derive(Debug, Clone, PartialEq)]
pub enum AppSource {
    Applications,      // /Applications
    UserApplications,  // ~/Applications
    Homebrew,          // Homebrew cask
    System,            // System apps (not removable)
}

impl std::fmt::Display for AppSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppSource::Applications => write!(f, "Applications"),
            AppSource::UserApplications => write!(f, "User Apps"),
            AppSource::Homebrew => write!(f, "Homebrew"),
            AppSource::System => write!(f, "System"),
        }
    }
}

/// Information about an installed application
#[derive(Debug, Clone)]
pub struct InstalledApp {
    pub name: String,
    pub bundle_id: Option<String>,
    pub version: Option<String>,
    pub path: PathBuf,
    pub app_size: u64,
    pub associated_files: Vec<AssociatedFile>,
    pub total_size: u64,
    pub last_opened: Option<SystemTime>,
    pub source: AppSource,
}

/// A file or directory associated with an application
#[derive(Debug, Clone)]
pub struct AssociatedFile {
    pub path: PathBuf,
    pub size: u64,
    pub kind: AssociatedKind,
    pub exists: bool,
}

#[derive(Debug, Clone)]
pub enum AssociatedKind {
    AppSupport,
    Cache,
    Preferences,
    SavedState,
    Container,
    GroupContainer,
    Cookies,
    HttpStorage,
    WebKit,
    Logs,
    LaunchAgent,
    LoginItem,
    CrashReports,
}

impl std::fmt::Display for AssociatedKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssociatedKind::AppSupport => write!(f, "App Support"),
            AssociatedKind::Cache => write!(f, "Cache"),
            AssociatedKind::Preferences => write!(f, "Preferences"),
            AssociatedKind::SavedState => write!(f, "Saved State"),
            AssociatedKind::Container => write!(f, "Container"),
            AssociatedKind::GroupContainer => write!(f, "Group Container"),
            AssociatedKind::Cookies => write!(f, "Cookies"),
            AssociatedKind::HttpStorage => write!(f, "HTTP Storage"),
            AssociatedKind::WebKit => write!(f, "WebKit Data"),
            AssociatedKind::Logs => write!(f, "Logs"),
            AssociatedKind::LaunchAgent => write!(f, "Launch Agent"),
            AssociatedKind::LoginItem => write!(f, "Login Item"),
            AssociatedKind::CrashReports => write!(f, "Crash Reports"),
        }
    }
}

/// Discover all installed applications
pub fn discover_apps() -> Vec<InstalledApp> {
    let mut apps = Vec::new();

    // Scan /Applications
    scan_app_directory(Path::new("/Applications"), AppSource::Applications, &mut apps);

    // Scan ~/Applications
    if let Some(home) = dirs::home_dir() {
        scan_app_directory(&home.join("Applications"), AppSource::UserApplications, &mut apps);
    }

    // Sort by name
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps
}

/// Scan a directory for .app bundles
fn scan_app_directory(dir: &Path, source: AppSource, apps: &mut Vec<InstalledApp>) {
    if !dir.exists() {
        return;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("app") {
            if let Some(app) = parse_app_bundle(&path, source.clone()) {
                apps.push(app);
            }
        }
    }
}

/// Parse an .app bundle to extract metadata
fn parse_app_bundle(app_path: &Path, source: AppSource) -> Option<InstalledApp> {
    let name = app_path
        .file_stem()?
        .to_string_lossy()
        .to_string();

    let info_plist = app_path.join("Contents/Info.plist");

    let (bundle_id, version) = if info_plist.exists() {
        parse_info_plist(&info_plist)
    } else {
        (None, None)
    };

    let app_size = crate::scanner::walker::dir_size(app_path);
    let last_opened = std::fs::metadata(app_path)
        .ok()
        .and_then(|m| m.accessed().ok());

    // Find associated files
    let associated = find_associated_files(&name, bundle_id.as_deref());
    let assoc_size: u64 = associated.iter().filter(|a| a.exists).map(|a| a.size).sum();

    Some(InstalledApp {
        name,
        bundle_id,
        version,
        path: app_path.to_path_buf(),
        app_size,
        associated_files: associated,
        total_size: app_size + assoc_size,
        last_opened,
        source,
    })
}

/// Parse Info.plist to extract bundle ID and version
fn parse_info_plist(path: &Path) -> (Option<String>, Option<String>) {
    let plist_val = match plist::Value::from_file(path) {
        Ok(v) => v,
        Err(_) => return (None, None),
    };

    let dict = match plist_val.as_dictionary() {
        Some(d) => d,
        None => return (None, None),
    };

    let bundle_id = dict
        .get("CFBundleIdentifier")
        .and_then(|v| v.as_string())
        .map(|s| s.to_string());

    let version = dict
        .get("CFBundleShortVersionString")
        .and_then(|v| v.as_string())
        .map(|s| s.to_string());

    (bundle_id, version)
}

/// Find all files associated with an application
pub fn find_associated_files(app_name: &str, bundle_id: Option<&str>) -> Vec<AssociatedFile> {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return Vec::new(),
    };

    let mut files = Vec::new();

    // Build search identifiers
    let identifiers: Vec<&str> = match bundle_id {
        Some(bid) => vec![bid, app_name],
        None => vec![app_name],
    };

    for id in &identifiers {
        // ~/Library/Application Support/
        check_associated(&home.join("Library/Application Support").join(id), AssociatedKind::AppSupport, &mut files);

        // ~/Library/Caches/
        check_associated(&home.join("Library/Caches").join(id), AssociatedKind::Cache, &mut files);

        // ~/Library/Preferences/
        check_associated(&home.join("Library/Preferences").join(format!("{}.plist", id)), AssociatedKind::Preferences, &mut files);

        // ~/Library/Preferences/ByHost/
        check_glob_associated(
            &home.join("Library/Preferences/ByHost"),
            &format!("{}.*", id),
            AssociatedKind::Preferences,
            &mut files,
        );

        // ~/Library/Saved Application State/
        check_associated(
            &home.join("Library/Saved Application State").join(format!("{}.savedState", id)),
            AssociatedKind::SavedState,
            &mut files,
        );

        // ~/Library/Containers/
        check_associated(&home.join("Library/Containers").join(id), AssociatedKind::Container, &mut files);

        // ~/Library/Group Containers/
        check_glob_associated(
            &home.join("Library/Group Containers"),
            &format!("*{}*", id),
            AssociatedKind::GroupContainer,
            &mut files,
        );

        // ~/Library/Cookies/
        check_associated(
            &home.join("Library/Cookies").join(format!("{}.binarycookies", id)),
            AssociatedKind::Cookies,
            &mut files,
        );

        // ~/Library/HTTPStorages/
        check_associated(&home.join("Library/HTTPStorages").join(id), AssociatedKind::HttpStorage, &mut files);

        // ~/Library/WebKit/
        check_associated(&home.join("Library/WebKit").join(id), AssociatedKind::WebKit, &mut files);

        // ~/Library/Logs/
        check_associated(&home.join("Library/Logs").join(id), AssociatedKind::Logs, &mut files);

        // Launch Agents
        check_glob_associated(
            &home.join("Library/LaunchAgents"),
            &format!("{}*.plist", id),
            AssociatedKind::LaunchAgent,
            &mut files,
        );
        check_glob_associated(
            Path::new("/Library/LaunchAgents"),
            &format!("{}*.plist", id),
            AssociatedKind::LaunchAgent,
            &mut files,
        );

        // Crash Reports
        check_glob_associated(
            &home.join("Library/Logs/DiagnosticReports"),
            &format!("{}*", app_name),
            AssociatedKind::CrashReports,
            &mut files,
        );
    }

    // Deduplicate by path
    files.sort_by(|a, b| a.path.cmp(&b.path));
    files.dedup_by(|a, b| a.path == b.path);

    files
}

/// Check if a specific path exists and add it as an associated file
fn check_associated(path: &Path, kind: AssociatedKind, files: &mut Vec<AssociatedFile>) {
    let exists = path.exists();
    let size = if exists {
        if path.is_dir() {
            crate::scanner::walker::dir_size(path)
        } else {
            std::fs::metadata(path).map(|m| {
                use std::os::darwin::fs::MetadataExt;
                m.st_blocks() * 512
            }).unwrap_or(0)
        }
    } else {
        0
    };

    if exists {
        files.push(AssociatedFile {
            path: path.to_path_buf(),
            size,
            kind,
            exists,
        });
    }
}

/// Check for matching files using glob pattern
fn check_glob_associated(dir: &Path, pattern: &str, kind: AssociatedKind, files: &mut Vec<AssociatedFile>) {
    if !dir.exists() {
        return;
    }

    let glob_pattern = format!("{}/{}", dir.display(), pattern);
    if let Ok(entries) = glob::glob(&glob_pattern) {
        for entry in entries.filter_map(|e| e.ok()) {
            check_associated(&entry, kind.clone(), files);
        }
    }
}
