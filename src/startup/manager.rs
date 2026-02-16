use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// A startup/login item on the system
#[derive(Debug, Clone)]
pub struct StartupItem {
    pub name: String,
    pub label: String,
    pub path: PathBuf,
    pub kind: StartupKind,
    pub enabled: bool,
    pub program: Option<String>,
    pub run_at_load: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StartupKind {
    UserLaunchAgent,
    SystemLaunchAgent,
    SystemLaunchDaemon,
}

impl std::fmt::Display for StartupKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StartupKind::UserLaunchAgent => write!(f, "User Agent"),
            StartupKind::SystemLaunchAgent => write!(f, "System Agent"),
            StartupKind::SystemLaunchDaemon => write!(f, "System Daemon"),
        }
    }
}

/// Discover all startup items on the system
pub fn discover_startup_items() -> Vec<StartupItem> {
    let mut items = Vec::new();
    let home = dirs::home_dir().unwrap_or_default();

    // User Launch Agents
    let user_agents = home.join("Library/LaunchAgents");
    scan_launch_dir(&user_agents, StartupKind::UserLaunchAgent, &mut items);

    // System Launch Agents
    scan_launch_dir(
        Path::new("/Library/LaunchAgents"),
        StartupKind::SystemLaunchAgent,
        &mut items,
    );

    // System Launch Daemons
    scan_launch_dir(
        Path::new("/Library/LaunchDaemons"),
        StartupKind::SystemLaunchDaemon,
        &mut items,
    );

    items.sort_by(|a, b| a.name.cmp(&b.name));
    items
}

/// Scan a LaunchAgents/Daemons directory
fn scan_launch_dir(dir: &Path, kind: StartupKind, items: &mut Vec<StartupItem>) {
    if !dir.exists() {
        return;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("plist") {
            continue;
        }

        if let Some(item) = parse_launch_plist(&path, kind.clone()) {
            items.push(item);
        }
    }
}

/// Parse a launchd plist file
fn parse_launch_plist(path: &Path, kind: StartupKind) -> Option<StartupItem> {
    let plist_val = plist::Value::from_file(path).ok()?;
    let dict = plist_val.as_dictionary()?;

    let label = dict
        .get("Label")
        .and_then(|v| v.as_string())
        .unwrap_or("unknown")
        .to_string();

    // Extract program path
    let program = dict
        .get("Program")
        .and_then(|v| v.as_string())
        .map(|s| s.to_string())
        .or_else(|| {
            dict.get("ProgramArguments")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_string())
                .map(|s| s.to_string())
        });

    let run_at_load = dict
        .get("RunAtLoad")
        .and_then(|v| v.as_boolean())
        .unwrap_or(false);

    // Check if disabled
    let disabled = dict
        .get("Disabled")
        .and_then(|v| v.as_boolean())
        .unwrap_or(false);

    // Also check overrides directory for user agents
    let is_disabled_override = check_disabled_override(&label);

    let name = label
        .rsplit('.')
        .next()
        .unwrap_or(&label)
        .to_string();

    Some(StartupItem {
        name,
        label,
        path: path.to_path_buf(),
        kind,
        enabled: !disabled && !is_disabled_override,
        program,
        run_at_load,
    })
}

/// Check launchctl overrides for disabled state
fn check_disabled_override(label: &str) -> bool {
    // Use launchctl to check if the service is disabled
    let output = std::process::Command::new("launchctl")
        .args(["print-disabled", "user/501"])
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            // Look for `"label" => disabled`
            stdout.contains(&format!("\"{}\" => disabled", label))
        }
        Err(_) => false,
    }
}

/// Disable a startup item by label
pub fn disable_item(item: &StartupItem) -> Result<String> {
    if !item.enabled {
        return Ok(format!("'{}' is already disabled", item.label));
    }

    match item.kind {
        StartupKind::UserLaunchAgent => {
            // Use launchctl to disable
            let status = std::process::Command::new("launchctl")
                .args(["disable", &format!("user/501/{}", item.label)])
                .status()
                .context("Failed to run launchctl")?;

            if status.success() {
                // Also unload if loaded
                let _ = std::process::Command::new("launchctl")
                    .args(["bootout", &format!("user/501/{}", item.label)])
                    .status();

                Ok(format!("Disabled '{}'", item.label))
            } else {
                anyhow::bail!("launchctl disable failed for '{}'", item.label)
            }
        }
        StartupKind::SystemLaunchAgent | StartupKind::SystemLaunchDaemon => {
            anyhow::bail!(
                "Cannot disable system item '{}'. Requires sudo.",
                item.label
            )
        }
    }
}

/// Enable a previously disabled startup item
pub fn enable_item(item: &StartupItem) -> Result<String> {
    if item.enabled {
        return Ok(format!("'{}' is already enabled", item.label));
    }

    match item.kind {
        StartupKind::UserLaunchAgent => {
            let status = std::process::Command::new("launchctl")
                .args(["enable", &format!("user/501/{}", item.label)])
                .status()
                .context("Failed to run launchctl")?;

            if status.success() {
                // Also bootstrap (load) the service
                let _ = std::process::Command::new("launchctl")
                    .args(["bootstrap", "user/501", &item.path.display().to_string()])
                    .status();

                Ok(format!("Enabled '{}'", item.label))
            } else {
                anyhow::bail!("launchctl enable failed for '{}'", item.label)
            }
        }
        StartupKind::SystemLaunchAgent | StartupKind::SystemLaunchDaemon => {
            anyhow::bail!(
                "Cannot enable system item '{}'. Requires sudo.",
                item.label
            )
        }
    }
}

/// Find a startup item by name or label (case-insensitive partial match)
pub fn find_item_by_name<'a>(items: &'a [StartupItem], name: &str) -> Vec<&'a StartupItem> {
    let lower = name.to_lowercase();
    items
        .iter()
        .filter(|i| {
            i.name.to_lowercase().contains(&lower) || i.label.to_lowercase().contains(&lower)
        })
        .collect()
}
