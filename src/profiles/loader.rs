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
    /// User-defined cleanup rules (`[[rules]]` tables).
    #[serde(default)]
    pub rules: Vec<CustomRule>,
}

/// A user-defined cleanup rule.
///
/// ```toml
/// [[rules]]
/// name = "Old screen recordings"
/// paths = ["~/Movies/Screen Recordings"]
/// min_age_days = 60
/// min_size_mb = 50
/// extensions = ["mov", "mp4"]
/// safety = "caution"
/// ```
///
/// Each direct child of each path is one entry (the folder itself is never
/// removed). Rules are still subject to the safety guard: folders such as
/// ~/Documents or ~/Library/Application Support stay protected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomRule {
    pub name: String,
    pub paths: Vec<String>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub min_age_days: Option<u32>,
    #[serde(default)]
    pub min_size_mb: Option<u64>,
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
    /// `false` = only loose files directly inside the paths.
    #[serde(default = "default_true")]
    pub include_folders: bool,
    #[serde(default)]
    pub safety: Option<targets::SafetyLevel>,
}

impl CustomRule {
    /// Problems with the rule (empty if valid).
    pub fn validate(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if self.name.trim().is_empty() {
            problems.push("rule has no name".to_string());
        }
        if self.paths.is_empty() {
            problems.push(format!("rule '{}' has no paths", self.name));
        }
        let home = dirs::home_dir().unwrap_or_default();
        for raw in &self.paths {
            if !(raw.starts_with("~/") || raw.starts_with(&*home.to_string_lossy())) {
                problems.push(format!("rule '{}': path '{}' must be inside your home folder", self.name, raw));
            }
            if raw.split('/').any(|c| c == "..") {
                problems.push(format!("rule '{}': path '{}' may not contain '..'", self.name, raw));
            }
        }
        problems
    }

    /// Convert to a scan target.
    pub fn to_target(&self) -> ScanTarget {
        ScanTarget {
            name: self.name.clone(),
            category: targets::Category::UserCache,
            paths: self.paths.clone(),
            safety: self.safety.clone().unwrap_or(targets::SafetyLevel::Caution),
            reason: self
                .reason
                .clone()
                .unwrap_or_else(|| "Custom rule".to_string()),
            recursive: self.include_folders,
            min_age_days: self.min_age_days,
            min_size_bytes: self.min_size_mb.map(|mb| mb * 1024 * 1024),
            exclude: self.exclude.clone(),
            extensions: self.extensions.iter().map(|e| e.trim_start_matches('.').to_lowercase()).collect(),
            ..Default::default()
        }
    }
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
    #[serde(default)]
    pub go_cache: bool,
    #[serde(default)]
    pub maven_cache: bool,
    #[serde(default)]
    pub pnpm_store: bool,
    #[serde(default)]
    pub ruby_gems_cache: bool,
    #[serde(default)]
    pub terraform_cache: bool,
    #[serde(default)]
    pub jvm_caches: bool,
    #[serde(default)]
    pub xcode_device_support: bool,
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

fn default_true() -> bool {
    true
}
fn default_stale() -> u32 {
    30
}
fn default_large() -> u64 {
    500
}

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

        anyhow::bail!(
            "Profile '{}' not found. Available: quick, developer, creative, deep",
            name
        )
    }

    /// Check if dev project scanning is enabled
    pub fn includes_dev_projects(&self) -> bool {
        self.targets.dev.node_modules_stale || self.targets.dev.venv
    }

    /// Get the list of scan targets enabled by this profile (incl. valid custom rules).
    pub fn enabled_targets(&self) -> Vec<ScanTarget> {
        let all = targets::all_targets();
        let mut enabled: Vec<ScanTarget> = all.into_iter().filter(|t| self.is_target_enabled(t)).collect();
        for rule in &self.rules {
            let problems = rule.validate();
            if problems.is_empty() {
                enabled.push(rule.to_target());
            } else {
                for p in problems {
                    tracing::warn!("Ignoring custom rule: {}", p);
                }
            }
        }
        enabled
    }

    /// Problems in this profile's custom rules.
    pub fn rule_problems(&self) -> Vec<String> {
        self.rules.iter().flat_map(|r| r.validate()).collect()
    }

    /// Large-file threshold in bytes if this profile lists large files.
    pub fn large_file_threshold(&self) -> Option<u64> {
        if self.targets.large_files {
            Some(self.thresholds.large_file_mb.max(1) * 1024 * 1024)
        } else {
            None
        }
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
            Xcode => self.targets.dev.xcode_derived_data || self.targets.dev.xcode_device_support,
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
            Go => self.targets.dev.go_cache,
            Maven => self.targets.dev.maven_cache,
            Pnpm => self.targets.dev.pnpm_store,
            YarnBerry => self.targets.dev.yarn_cache,
            RubyGems => self.targets.dev.ruby_gems_cache,
            Terraform => self.targets.dev.terraform_cache,
            Ivy2 | Sbt => self.targets.dev.jvm_caches,
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
            rules: Vec::new(),
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
                    go_cache: true,
                    pnpm_store: true,
                    xcode_device_support: true,
                    ..Default::default()
                },
            },
            rules: Vec::new(),
            thresholds: ProfileThresholds {
                stale_days: 30,
                large_file_mb: 500,
            },
        }),

        "creative" => Some(Profile {
            profile: ProfileMeta {
                name: "creative".into(),
                description:
                    "Clean up after creative work — render caches, previews, scratch files".into(),
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
            rules: Vec::new(),
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
                    go_cache: true,
                    pnpm_store: true,
                    xcode_device_support: true,
                    maven_cache: true,
                    ruby_gems_cache: true,
                    terraform_cache: true,
                    jvm_caches: true,
                },
            },
            rules: Vec::new(),
            thresholds: ProfileThresholds {
                stale_days: 14,
                large_file_mb: 100,
            },
        }),

        _ => None,
    }
}
