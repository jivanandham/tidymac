//! Scheduled dry-run reports via launchd.
//!
//! `tidymac schedule install --profile developer --every weekly` writes
//! `~/Library/LaunchAgents/com.tidymac.report.plist`, which runs
//! `tidymac report --profile developer` on schedule. A report is a dry run: it
//! scans and records what *could* be cleaned in `~/.tidymac/reports/`, and
//! never deletes anything.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::common::config::Config;
use crate::common::sys::gui_domain;

pub const LABEL: &str = "com.tidymac.report";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum Interval {
    Daily,
    Weekly,
    Monthly,
}

impl Interval {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "daily" => Some(Interval::Daily),
            "weekly" => Some(Interval::Weekly),
            "monthly" => Some(Interval::Monthly),
            _ => None,
        }
    }

    fn calendar_xml(&self) -> &'static str {
        match self {
            Interval::Daily => "<key>Hour</key><integer>10</integer><key>Minute</key><integer>0</integer>",
            Interval::Weekly => {
                "<key>Weekday</key><integer>1</integer><key>Hour</key><integer>10</integer><key>Minute</key><integer>0</integer>"
            }
            Interval::Monthly => {
                "<key>Day</key><integer>1</integer><key>Hour</key><integer>10</integer><key>Minute</key><integer>0</integer>"
            }
        }
    }
}

impl std::fmt::Display for Interval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Interval::Daily => write!(f, "daily"),
            Interval::Weekly => write!(f, "weekly"),
            Interval::Monthly => write!(f, "monthly"),
        }
    }
}

/// Saved alongside the plist so `status` can describe the schedule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleStatus {
    pub profile: String,
    pub interval: Interval,
    pub binary: PathBuf,
    pub plist: PathBuf,
}

/// Summary of one scheduled report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSummary {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub profile: String,
    pub total_reclaimable: u64,
    pub items: Vec<ReportLine>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportLine {
    pub name: String,
    pub size_bytes: u64,
    pub entries: usize,
}

pub fn plist_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join("Library/LaunchAgents")
        .join(format!("{}.plist", LABEL))
}

pub fn reports_dir() -> PathBuf {
    Config::data_dir().join("reports")
}

fn status_path() -> PathBuf {
    Config::data_dir().join("schedule.json")
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Generate the launchd plist.
pub fn generate_plist(binary: &Path, profile: &str, interval: Interval) -> String {
    let logs = Config::logs_dir();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{label}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{bin}</string>
        <string>report</string>
        <string>--profile</string>
        <string>{profile}</string>
    </array>
    <key>StartCalendarInterval</key>
    <dict>{cal}</dict>
    <key>ProcessType</key>
    <string>Background</string>
    <key>LowPriorityIO</key>
    <true/>
    <key>Nice</key>
    <integer>10</integer>
    <key>StandardOutPath</key>
    <string>{logs}/report.log</string>
    <key>StandardErrorPath</key>
    <string>{logs}/report-error.log</string>
</dict>
</plist>
"#,
        label = LABEL,
        bin = xml_escape(&binary.display().to_string()),
        profile = xml_escape(profile),
        cal = interval.calendar_xml(),
        logs = xml_escape(&logs.display().to_string()),
    )
}

/// Install (or replace) the scheduled report and load it with launchd.
/// `binary` is the `tidymac` CLI to run (defaults to the current executable).
pub fn install(profile: &str, interval: Interval, binary: Option<PathBuf>) -> Result<ScheduleStatus> {
    crate::profiles::loader::Profile::load(profile)?; // validate the name
    let binary = match binary {
        Some(b) => b,
        None => std::env::current_exe().context("Cannot determine the tidymac executable path")?,
    };
    if !binary.is_file() {
        anyhow::bail!("tidymac CLI not found at {}", binary.display());
    }
    Config::init_dirs()?;
    std::fs::create_dir_all(reports_dir())?;

    let path = plist_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    // Replace an existing job cleanly.
    let _ = std::process::Command::new("/bin/launchctl")
        .args(["bootout", &format!("{}/{}", gui_domain(), LABEL)])
        .output();
    std::fs::write(&path, generate_plist(&binary, profile, interval))
        .with_context(|| format!("Failed to write {}", path.display()))?;
    let out = std::process::Command::new("/bin/launchctl")
        .args(["bootstrap", &gui_domain(), &path.display().to_string()])
        .output()
        .context("Failed to run launchctl")?;
    if !out.status.success() {
        tracing::warn!("launchctl bootstrap: {}", String::from_utf8_lossy(&out.stderr));
    }

    let status = ScheduleStatus {
        profile: profile.to_string(),
        interval,
        binary,
        plist: path,
    };
    std::fs::write(status_path(), serde_json::to_string_pretty(&status)?)?;
    Ok(status)
}

/// Unload and remove the scheduled report.
pub fn remove() -> Result<bool> {
    let path = plist_path();
    let _ = std::process::Command::new("/bin/launchctl")
        .args(["bootout", &format!("{}/{}", gui_domain(), LABEL)])
        .output();
    let existed = path.exists();
    if existed {
        std::fs::remove_file(&path)?;
    }
    let _ = std::fs::remove_file(status_path());
    Ok(existed)
}

/// Current schedule, if installed.
pub fn status() -> Option<ScheduleStatus> {
    if !plist_path().exists() {
        return None;
    }
    let text = std::fs::read_to_string(status_path()).ok()?;
    serde_json::from_str(&text).ok()
}

/// Run a dry-run scan and save it as a report. Returns the report path.
pub fn write_report(profile_name: &str) -> Result<(PathBuf, ReportSummary)> {
    let profile = crate::profiles::loader::Profile::load(profile_name)?;
    let results = crate::scanner::run_scan(
        &profile.enabled_targets(),
        false,
        profile.includes_dev_projects(),
        profile.thresholds.stale_days,
        None, // large-file walks are skipped in background reports
    )?;
    let summary = ReportSummary {
        timestamp: chrono::Utc::now(),
        profile: profile_name.to_string(),
        total_reclaimable: results.total_reclaimable,
        items: results
            .items
            .iter()
            .map(|i| ReportLine {
                name: i.name.clone(),
                size_bytes: i.size_bytes,
                entries: i.files.len(),
            })
            .collect(),
        errors: results.errors.clone(),
    };

    let dir = reports_dir();
    std::fs::create_dir_all(&dir)?;
    let file = dir.join(format!("report-{}.json", summary.timestamp.format("%Y-%m-%dT%H-%M-%S")));
    let json = serde_json::to_string_pretty(&summary)?;
    std::fs::write(&file, &json)?;
    std::fs::write(dir.join("latest.json"), &json)?;

    // Keep the 30 most recent reports.
    if let Ok(rd) = std::fs::read_dir(&dir) {
        let mut reports: Vec<PathBuf> = rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .map(|n| n.to_string_lossy().starts_with("report-"))
                    .unwrap_or(false)
            })
            .collect();
        reports.sort();
        while reports.len() > 30 {
            let old = reports.remove(0);
            let _ = std::fs::remove_file(old);
        }
    }
    Ok((file, summary))
}

/// The most recent report, if any.
pub fn latest_report() -> Option<ReportSummary> {
    let text = std::fs::read_to_string(reports_dir().join("latest.json")).ok()?;
    serde_json::from_str(&text).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plist_contains_profile_and_schedule() {
        let p = generate_plist(Path::new("/usr/local/bin/tidymac"), "developer", Interval::Weekly);
        assert!(p.contains("<string>developer</string>"));
        assert!(p.contains("<key>Weekday</key>"));
        assert!(p.contains("<string>report</string>"));
    }

    #[test]
    fn interval_parse() {
        assert_eq!(Interval::parse("Weekly"), Some(Interval::Weekly));
        assert_eq!(Interval::parse("hourly"), None);
    }
}
