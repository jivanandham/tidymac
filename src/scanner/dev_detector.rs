use std::path::PathBuf;

use super::targets::{Category, DevTool, SafetyLevel, ScanItem};
use super::walker;

/// Detect installed developer tools and their cache sizes
pub fn detect_dev_environment() -> Vec<DevToolInfo> {
    let home = dirs::home_dir().unwrap_or_default();
    let mut tools = Vec::new();

    // Xcode
    let xcode_path = home.join("Library/Developer/Xcode/DerivedData");
    if xcode_path.exists() {
        tools.push(DevToolInfo {
            tool: DevTool::Xcode,
            installed: true,
            cache_path: xcode_path,
        });
    }

    // Docker
    let docker_path = home.join("Library/Containers/com.docker.docker");
    if docker_path.exists() {
        tools.push(DevToolInfo {
            tool: DevTool::Docker,
            installed: true,
            cache_path: docker_path,
        });
    }

    // Homebrew
    let brew_cache = home.join("Library/Caches/Homebrew");
    if brew_cache.exists() {
        tools.push(DevToolInfo {
            tool: DevTool::Homebrew,
            installed: true,
            cache_path: brew_cache,
        });
    }

    // npm
    let npm_cache = home.join(".npm/_cacache");
    if npm_cache.exists() {
        tools.push(DevToolInfo {
            tool: DevTool::Npm,
            installed: true,
            cache_path: npm_cache,
        });
    }

    // Yarn
    let yarn_cache = home.join("Library/Caches/Yarn");
    if yarn_cache.exists() {
        tools.push(DevToolInfo {
            tool: DevTool::Yarn,
            installed: true,
            cache_path: yarn_cache,
        });
    }

    // pip
    let pip_cache = home.join("Library/Caches/pip");
    if pip_cache.exists() {
        tools.push(DevToolInfo {
            tool: DevTool::Pip,
            installed: true,
            cache_path: pip_cache,
        });
    }

    // Cargo
    let cargo_cache = home.join(".cargo/registry/cache");
    if cargo_cache.exists() {
        tools.push(DevToolInfo {
            tool: DevTool::Cargo,
            installed: true,
            cache_path: cargo_cache,
        });
    }

    // Gradle
    let gradle_cache = home.join(".gradle/caches");
    if gradle_cache.exists() {
        tools.push(DevToolInfo {
            tool: DevTool::Gradle,
            installed: true,
            cache_path: gradle_cache,
        });
    }

    // CocoaPods
    let pods_cache = home.join("Library/Caches/CocoaPods");
    if pods_cache.exists() {
        tools.push(DevToolInfo {
            tool: DevTool::CocoaPods,
            installed: true,
            cache_path: pods_cache,
        });
    }

    // Conda
    let conda_cache = home.join(".conda/pkgs");
    if conda_cache.exists() {
        tools.push(DevToolInfo {
            tool: DevTool::Conda,
            installed: true,
            cache_path: conda_cache,
        });
    }

    tools
}

/// Scan for stale node_modules across project directories
pub fn scan_node_modules(stale_days: u32) -> ScanItem {
    let home = dirs::home_dir().unwrap_or_default();
    let search_roots = vec![
        home.join("Projects"),
        home.join("projects"),
        home.join("Code"),
        home.join("code"),
        home.join("Development"),
        home.join("dev"),
        home.join("workspace"),
        home.join("repos"),
        home.join("src"),
        home.join("Documents"),
        home.join("Desktop"),
    ];

    let existing_roots: Vec<PathBuf> = search_roots.into_iter().filter(|p| p.exists()).collect();
    let files = walker::find_node_modules(&existing_roots, stale_days);

    let total_size: u64 = files.iter().map(|f| f.size_bytes).sum();
    let count = files.len();

    ScanItem {
        name: format!("Stale node_modules (>{} days)", stale_days),
        category: Category::DevCache(DevTool::NodeModules),
        path: home.join("**/node_modules"),
        size_bytes: total_size,
        file_count: count,
        safety: SafetyLevel::Safe,
        reason: format!(
            "node_modules in projects not modified for {}+ days — reinstall with 'npm install'",
            stale_days
        ),
        files,
    }
}

/// Scan for stale Python virtual environments
pub fn scan_venvs(stale_days: u32) -> ScanItem {
    let home = dirs::home_dir().unwrap_or_default();
    let search_roots = vec![
        home.join("Projects"),
        home.join("projects"),
        home.join("Code"),
        home.join("code"),
        home.join("Development"),
        home.join("dev"),
        home.join("workspace"),
        home.join("repos"),
        home.join("src"),
        home.join("Documents"),
    ];

    let existing_roots: Vec<PathBuf> = search_roots.into_iter().filter(|p| p.exists()).collect();
    let files = walker::find_venvs(&existing_roots, stale_days);

    let total_size: u64 = files.iter().map(|f| f.size_bytes).sum();
    let count = files.len();

    ScanItem {
        name: format!("Stale Python venvs (>{} days)", stale_days),
        category: Category::DevCache(DevTool::Venv),
        path: home.join("**/.venv"),
        size_bytes: total_size,
        file_count: count,
        safety: SafetyLevel::Safe,
        reason: format!(
            "Python virtualenvs in stale projects — recreate with 'python -m venv .venv'",
        ),
        files,
    }
}

#[derive(Debug, Clone)]
pub struct DevToolInfo {
    pub tool: DevTool,
    pub installed: bool,
    pub cache_path: PathBuf,
}
