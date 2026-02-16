use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::scanner::targets::{self, ScanTarget};

/// A smart cleanup profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub profile: ProfileMeta,
    pub targets: ProfileTargets,
    #[serde(default)]
    pub thresholds: ProfileThresholds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileMeta {
    pub name: String,
    pub description: String,
    pub aggression: Aggression,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Aggression {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileTargets {
    #[serde(default = "default_true")]
    pub system_caches: bool,
    #[serde(default = "default_true")]
    pub user_caches: bool,
    #[serde(default = "default_true")]
    pub logs: bool,
    #[serde(default = "default_true")]
    pub temp_files: bool,
    #[serde(default = "default_true")]
    pub trash: bool,
    #[serde(default)]
    pub crash_reports: bool,
    #[serde(default)]
    pub mail_attachments: bool,
    #[serde(default)]
    pub downloaded_dmgs: bool,
    #[serde(default)]
    pub large_files: bool,
    #[serde(default)]
    pub dev: DevTargets,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DevTargets {
    #[serde(default)]
    pub xcode_derived_data: bool,
    #[serde(default)]
    pub xcode_archives: bool,
    #[serde(default)]
    pub ios_simulators: bool,
    #[serde(default)]
    pub docker_dangling: bool,
    #[serde(default)]
    pub node_modules_stale: bool,
    #[serde(default)]
    pub venv: bool,
    #[serde(default)]
    pub homebrew_cache: bool,
    #[serde(default)]
    pub pip_cache: bool,
    #[serde(default)]
    pub npm_cache: bool,
    #[serde(default)]
    pub yarn_cache: bool,
    #[serde(default)]
    pub cocoapods_cache: bool,
    #[serde(default)]
    pub cargo_cache: bool,
    #[serde(default)]
    pub gradle_cache: bool,
    #[serde(default)]
    pub conda_cache: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileThresholds {
    #[serde(default = "default_stale")]
    pub stale_days: u32,
    #[serde(default = "default_large")]
    pub large_file_mb: u64,
}

impl Default for ProfileThresholds {
    fn default() -> Self {
        Self {
            stale_days: default_stale(),
            large_file_mb: default_large(),
        }
    }
}

fn default_true() -> bool { true }
fn default_stale() -> u32 { 30 }
fn default_large() -> u64 { 500 }

impl Profile {
    /// Load a profile by name
    pub fn load(name: &str) -> Result<Self> {
        // First check built-in profiles
        if let Some(profile) = builtin_profile(name) {
            return Ok(profile);
        }

        // Then check user profiles directory
        let path = crate::common::config::Config::profiles_dir().join(format!("{}.toml", name));
        if path.exists() {
            let contents = std::fs::read_to_string(&path)
                .with_context(|| format!("Failed to read profile: {}", path.display()))?;
            let profile: Profile = toml::from_str(&contents)
                .with_context(|| format!("Failed to parse profile: {}", path.display()))?;
            return Ok(profile);
        }

        anyhow::bail!("Profile '{}' not found. Available: quick, developer, creative, deep", name)
    }

    /// Check if dev project scanning is enabled
    pub fn includes_dev_projects(&self) -> bool {
        self.targets.dev.node_modules_stale || self.targets.dev.venv
    }

    /// Get the list of scan targets enabled by this profile
    pub fn enabled_targets(&self) -> Vec<ScanTarget> {
        let all = targets::all_targets();
        all.into_iter()
            .filter(|t| self.is_target_enabled(t))
            .collect()
    }

    fn is_target_enabled(&self, target: &ScanTarget) -> bool {
        use targets::Category::*;
        match &target.category {
            SystemCache => self.targets.system_caches,
            UserCache => self.targets.user_caches,
            Logs => self.targets.logs,
            TempFiles => self.targets.temp_files,
            Trash => self.targets.trash,
            CrashReports => self.targets.crash_reports,
            MailAttachment => self.targets.mail_attachments,
            DownloadedDmg => self.targets.downloaded_dmgs,
            LargeFile => self.targets.large_files,
            DevCache(tool) => self.is_dev_tool_enabled(tool),
            _ => false,
        }
    }

    fn is_dev_tool_enabled(&self, tool: &targets::DevTool) -> bool {
        use targets::DevTool::*;
        match tool {
            Xcode => self.targets.dev.xcode_derived_data,
            XcodeArchives => self.targets.dev.xcode_archives,
            XcodeSimulators => self.targets.dev.ios_simulators,
            Docker => self.targets.dev.docker_dangling,
            NodeModules => self.targets.dev.node_modules_stale,
            Venv => self.targets.dev.venv,
            Conda => self.targets.dev.conda_cache,
            Homebrew => self.targets.dev.homebrew_cache,
            Pip => self.targets.dev.pip_cache,
            Npm => self.targets.dev.npm_cache,
            Yarn => self.targets.dev.yarn_cache,
            CocoaPods => self.targets.dev.cocoapods_cache,
            Cargo => self.targets.dev.cargo_cache,
            Gradle => self.targets.dev.gradle_cache,
            _ => false,
        }
    }

    /// List all available profile names
    pub fn available_profiles() -> Vec<String> {
        let mut names = vec![
            "quick".to_string(),
            "developer".to_string(),
            "creative".to_string(),
            "deep".to_string(),
        ];

        // Add user profiles
        if let Ok(entries) = std::fs::read_dir(crate::common::config::Config::profiles_dir()) {
            for entry in entries.filter_map(|e| e.ok()) {
                if let Some(name) = entry.path().file_stem() {
                    let name = name.to_string_lossy().to_string();
                    if !names.contains(&name) {
                        names.push(name);
                    }
                }
            }
        }

        names
    }
}

/// Built-in profile definitions
fn builtin_profile(name: &str) -> Option<Profile> {
    match name {
        "quick" | "quick_sweep" => Some(Profile {
            profile: ProfileMeta {
                name: "quick_sweep".into(),
                description: "Fast daily cleanup — caches, temp files, trash".into(),
                aggression: Aggression::Low,
            },
            targets: ProfileTargets {
                system_caches: true,
                user_caches: true,
                logs: true,
                temp_files: true,
                trash: true,
                crash_reports: false,
                mail_attachments: false,
                downloaded_dmgs: false,
                large_files: false,
                dev: DevTargets::default(),
            },
            thresholds: ProfileThresholds {
                stale_days: 30,
                large_file_mb: 500,
            },
        }),

        "developer" | "dev" => Some(Profile {
            profile: ProfileMeta {
                name: "developer".into(),
                description: "Full developer cache cleanup — Xcode, Docker, npm, pip, and more"
                    .into(),
                aggression: Aggression::Medium,
            },
            targets: ProfileTargets {
                system_caches: true,
                user_caches: true,
                logs: true,
                temp_files: true,
                trash: true,
                crash_reports: true,
                mail_attachments: false,
                downloaded_dmgs: true,
                large_files: false,
                dev: DevTargets {
                    xcode_derived_data: true,
                    xcode_archives: false,
                    ios_simulators: true,
                    docker_dangling: true,
                    node_modules_stale: true,
                    venv: true,
                    homebrew_cache: true,
                    pip_cache: true,
                    npm_cache: true,
                    yarn_cache: true,
                    cocoapods_cache: true,
                    cargo_cache: true,
                    gradle_cache: true,
                    conda_cache: true,
                },
            },
            thresholds: ProfileThresholds {
                stale_days: 30,
                large_file_mb: 500,
            },
        }),

        "creative" => Some(Profile {
            profile: ProfileMeta {
                name: "creative".into(),
                description: "Clean up after creative work — render caches, previews, scratch files"
                    .into(),
                aggression: Aggression::Medium,
            },
            targets: ProfileTargets {
                system_caches: true,
                user_caches: true,
                logs: true,
                temp_files: true,
                trash: true,
                crash_reports: true,
                mail_attachments: true,
                downloaded_dmgs: true,
                large_files: false,
                dev: DevTargets::default(),
            },
            thresholds: ProfileThresholds {
                stale_days: 14,
                large_file_mb: 200,
            },
        }),

        "deep" | "deep_clean" => Some(Profile {
            profile: ProfileMeta {
                name: "deep_clean".into(),
                description:
                    "Thorough cleanup — everything including large files and app leftovers".into(),
                aggression: Aggression::High,
            },
            targets: ProfileTargets {
                system_caches: true,
                user_caches: true,
                logs: true,
                temp_files: true,
                trash: true,
                crash_reports: true,
                mail_attachments: true,
                downloaded_dmgs: true,
                large_files: true,
                dev: DevTargets {
                    xcode_derived_data: true,
                    xcode_archives: true,
                    ios_simulators: true,
                    docker_dangling: true,
                    node_modules_stale: true,
                    venv: true,
                    homebrew_cache: true,
                    pip_cache: true,
                    npm_cache: true,
                    yarn_cache: true,
                    cocoapods_cache: true,
                    cargo_cache: true,
                    gradle_cache: true,
                    conda_cache: true,
                },
            },
            thresholds: ProfileThresholds {
                stale_days: 14,
                large_file_mb: 100,
            },
        }),

        _ => None,
    }
}
