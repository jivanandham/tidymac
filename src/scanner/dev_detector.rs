use std::path::PathBuf;

use super::targets::{Category, DevTool, SafetyLevel, ScanItem};
use crate::common::safety::DeletePolicy;
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

/// Folders where projects usually live. Duplicates (e.g. `Projects` and
/// `projects` on a case-insensitive volume) are removed by canonicalizing.
pub fn project_search_roots() -> Vec<PathBuf> {
    let home = dirs::home_dir().unwrap_or_default();
    let candidates = [
        "Developer",
        "Projects",
        "projects",
        "Code",
        "code",
        "Development",
        "dev",
        "workspace",
        "repos",
        "src",
        "git",
        "GitHub",
        "Documents",
        "Desktop",
    ];
    let mut seen = std::collections::HashSet::new();
    let mut roots = Vec::new();
    for c in candidates {
        let p = home.join(c);
        if let Ok(canon) = std::fs::canonicalize(&p) {
            if canon.is_dir() && seen.insert(canon.to_string_lossy().to_lowercase()) {
                roots.push(canon);
            }
        }
    }
    // Drop roots nested inside another root (they'd be walked twice).
    let all = roots.clone();
    roots.retain(|r| {
        !all.iter()
            .any(|o| o != r && r.starts_with(o))
    });
    roots
}

fn artifact_item(name: String, tool: DevTool, reason: String, files: Vec<super::targets::FileEntry>) -> ScanItem {
    let total_size: u64 = files.iter().map(|f| f.size_bytes).sum();
    // Each entry's project folder is its root: the entry is strictly inside it.
    let mut roots: Vec<PathBuf> = files
        .iter()
        .filter_map(|f| f.path.parent().map(|p| p.to_path_buf()))
        .collect();
    roots.sort();
    roots.dedup();
    ScanItem {
        name,
        category: Category::DevCache(tool),
        path: dirs::home_dir().unwrap_or_default(),
        size_bytes: total_size,
        reclaimable_bytes: total_size,
        file_count: files.len(),
        safety: SafetyLevel::Safe,
        reason,
        files,
        roots,
        policy: DeletePolicy::DevArtifact,
        ..Default::default()
    }
}

/// Scan for stale node_modules across project directories
pub fn scan_node_modules(stale_days: u32) -> ScanItem {
    let files = walker::find_node_modules(&project_search_roots(), stale_days);
    artifact_item(
        format!("Stale node_modules (>{} days)", stale_days),
        DevTool::NodeModules,
        format!(
            "node_modules in projects untouched for {}+ days — reinstall with 'npm install'",
            stale_days
        ),
        files,
    )
}

/// Scan for stale Python virtual environments
pub fn scan_venvs(stale_days: u32) -> ScanItem {
    let files = walker::find_venvs(&project_search_roots(), stale_days);
    artifact_item(
        format!("Stale Python venvs (>{} days)", stale_days),
        DevTool::Venv,
        "Python virtualenvs in stale projects — recreate with 'python -m venv .venv'".to_string(),
        files,
    )
}

#[derive(Debug, Clone)]
pub struct DevToolInfo {
    pub tool: DevTool,
    pub installed: bool,
    pub cache_path: PathBuf,
}
