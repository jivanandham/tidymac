use colored::*;
use serde_json;

use crate::common::format::{self, format_path, format_size, format_size_colored};
use crate::scanner::targets::{SafetyLevel, ScanItem, ScanResults};

/// Print scan results in human-readable format
pub fn print_scan_results(results: &ScanResults, detailed: bool) {
    println!();
    println!("{}  TidyMac Scan Results", "🧹".to_string());
    println!("{}", "─".repeat(60).dimmed());
    println!(
        "  Scanned in {}  •  {} reclaimable  •  {}",
        format::format_duration(results.duration_secs).cyan(),
        format_size_colored(results.total_reclaimable),
        format::format_count(results.total_files).dimmed()
    );
    println!("{}", "─".repeat(60).dimmed());
    println!();

    if results.items.is_empty() {
        println!("  {} Your Mac is already clean!", "✨".to_string());
        return;
    }

    // Group by safety level
    let safe_items: Vec<&ScanItem> = results
        .items
        .iter()
        .filter(|i| i.safety == SafetyLevel::Safe)
        .collect();
    let caution_items: Vec<&ScanItem> = results
        .items
        .iter()
        .filter(|i| i.safety == SafetyLevel::Caution)
        .collect();
    let dangerous_items: Vec<&ScanItem> = results
        .items
        .iter()
        .filter(|i| i.safety == SafetyLevel::Dangerous)
        .collect();

    // Print safe items
    if !safe_items.is_empty() {
        let safe_total: u64 = safe_items.iter().map(|i| i.size_bytes).sum();
        println!(
            "  {} {} ({})",
            "●".green(),
            "Safe to Remove".green().bold(),
            format_size_colored(safe_total)
        );
        println!();
        for item in &safe_items {
            print_scan_item(item, detailed);
        }
        println!();
    }

    // Print caution items
    if !caution_items.is_empty() {
        let caution_total: u64 = caution_items.iter().map(|i| i.size_bytes).sum();
        println!(
            "  {} {} ({})",
            "●".yellow(),
            "Review Recommended".yellow().bold(),
            format_size_colored(caution_total)
        );
        println!();
        for item in &caution_items {
            print_scan_item(item, detailed);
        }
        println!();
    }

    // Print dangerous items
    if !dangerous_items.is_empty() {
        let danger_total: u64 = dangerous_items.iter().map(|i| i.size_bytes).sum();
        println!(
            "  {} {} ({})",
            "●".red(),
            "Use Caution".red().bold(),
            format_size_colored(danger_total)
        );
        println!();
        for item in &dangerous_items {
            print_scan_item(item, detailed);
        }
        println!();
    }

    // Print errors if any
    if !results.errors.is_empty() {
        println!(
            "  {} {}",
            "⚠".yellow(),
            format!("{} warnings:", results.errors.len()).yellow()
        );
        for error in &results.errors {
            println!("    {} {}", "→".dimmed(), error.dimmed());
        }
        println!();
    }

    // Summary
    println!("{}", "─".repeat(60).dimmed());
    println!(
        "  {} Total reclaimable: {}",
        "💾".to_string(),
        format_size_colored(results.total_reclaimable)
    );
    println!(
        "  {} Run {} to clean safely",
        "💡".to_string(),
        "tidymac clean --profile <name>".cyan()
    );
    println!();
}

/// Print a single scan item
fn print_scan_item(item: &ScanItem, detailed: bool) {
    let category_icon = match item.category {
        crate::scanner::targets::Category::UserCache
        | crate::scanner::targets::Category::SystemCache => "📁",
        crate::scanner::targets::Category::Logs => "📋",
        crate::scanner::targets::Category::TempFiles => "🗑️",
        crate::scanner::targets::Category::CrashReports => "💥",
        crate::scanner::targets::Category::DevCache(_) => "🔧",
        crate::scanner::targets::Category::LargeFile => "📦",
        crate::scanner::targets::Category::Trash => "🗂️",
        crate::scanner::targets::Category::MailAttachment => "📎",
        crate::scanner::targets::Category::DownloadedDmg => "💿",
        _ => "📄",
    };

    println!(
        "    {} {:<40} {:>10}  ({})",
        category_icon,
        item.name,
        format_size(item.size_bytes),
        format::format_count(item.file_count).dimmed()
    );

    if detailed {
        println!(
            "      {} {}",
            "↳".dimmed(),
            format_path(&item.path).dimmed()
        );
        println!("      {} {}", "↳".dimmed(), item.reason.dimmed());

        // Show top 5 largest files
        let mut sorted_files = item.files.clone();
        sorted_files.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
        for file in sorted_files.iter().take(5) {
            println!(
                "        {} {} ({})",
                "•".dimmed(),
                format_path(&file.path).dimmed(),
                format_size(file.size_bytes).dimmed()
            );
        }
        if item.files.len() > 5 {
            println!(
                "        {} ... and {} more",
                "•".dimmed(),
                (item.files.len() - 5).to_string().dimmed()
            );
        }
        println!();
    }
}

/// Print scan results as JSON
pub fn print_scan_json(results: &ScanResults) {
    match serde_json::to_string_pretty(results) {
        Ok(json) => println!("{}", json),
        Err(e) => eprintln!("Error serializing results: {}", e),
    }
}

/// Print a minimal summary
pub fn print_scan_quiet(results: &ScanResults) {
    println!(
        "{}  {}  {}",
        format_size(results.total_reclaimable),
        results.total_files,
        results.items.len()
    );
}

/// Print profile info
pub fn print_profile_info(profile: &crate::profiles::loader::Profile) {
    println!();
    println!(
        "  {} Profile: {}",
        "📋".to_string(),
        profile.profile.name.bold()
    );
    println!(
        "  {} {}",
        "  ".to_string(),
        profile.profile.description.dimmed()
    );
    println!(
        "  {} Aggression: {:?}",
        "  ".to_string(),
        profile.profile.aggression
    );
    println!();
}

/// Print a clean operation report
pub fn print_clean_report(report: &crate::cleaner::CleanReport) {
    println!();
    let icon = match report.mode {
        crate::cleaner::CleanMode::DryRun => "ℹ️",
        crate::cleaner::CleanMode::SoftDelete => "✓",
        crate::cleaner::CleanMode::HardDelete => "🔥",
    };
    let mode_label = match report.mode {
        crate::cleaner::CleanMode::DryRun => "Dry run",
        crate::cleaner::CleanMode::SoftDelete => "Soft delete",
        crate::cleaner::CleanMode::HardDelete => "Hard delete",
    };

    println!(
        "  {} {} — {} files, {}",
        icon,
        mode_label.bold(),
        report.files_removed.to_string().cyan(),
        format_size_colored(report.bytes_freed),
    );

    if let Some(ref sid) = report.session_id {
        println!("  {} Session: {}", "💾", sid.cyan());
        println!(
            "  {} Undo with: {}",
            "💡",
            format!("tidymac undo --session {}", sid).cyan()
        );
    }

    if !report.errors.is_empty() {
        println!();
        println!("  {} {} errors:", "⚠".yellow(), report.errors.len());
        for (i, err) in report.errors.iter().enumerate().take(10) {
            println!("    {} {}", format!("{}.", i + 1).dimmed(), err.dimmed());
        }
        if report.errors.len() > 10 {
            println!(
                "    ... and {} more",
                (report.errors.len() - 10).to_string().dimmed()
            );
        }
    }
    println!();
}

/// Print the list of staging sessions
pub fn print_sessions(sessions: &[crate::cleaner::SessionSummary]) {
    println!();
    println!("  {} Staging Sessions", "📦");
    println!("{}", "─".repeat(80).dimmed());
    println!();

    if sessions.is_empty() {
        println!("  No sessions found in staging area.");
        println!();
        return;
    }

    println!(
        "  {:<24} {:<12} {:<10} {:>10} {:>8}  {}",
        "Session ID".dimmed(),
        "Profile".dimmed(),
        "Mode".dimmed(),
        "Size".dimmed(),
        "Files".dimmed(),
        "Status".dimmed(),
    );
    println!("  {}", "─".repeat(76).dimmed());

    for session in sessions {
        let status = if session.restored {
            "Restored".green().to_string()
        } else if session.is_expired {
            "Expired".red().to_string()
        } else {
            session
                .expires_at
                .map(|e| {
                    let duration = e - chrono::Utc::now();
                    let days = duration.num_days();
                    let hours = duration.num_hours() % 24;
                    if days > 0 {
                        format!("{}d left", days)
                    } else {
                        format!("{}h left", hours)
                    }
                })
                .unwrap_or_else(|| "N/A".to_string())
                .yellow()
                .to_string()
        };

        println!(
            "  {:<24} {:<12} {:<10} {:>10} {:>8}  {}",
            session.session_id,
            session.profile,
            session.mode,
            format_size(session.staged_size),
            session.total_files,
            status,
        );
    }

    println!();
    println!(
        "  {} Restore: {}",
        "💡",
        "tidymac undo --session <ID>".cyan()
    );
    println!(
        "  {} Purge expired: {}",
        "💡",
        "tidymac purge --expired".cyan()
    );
    println!();
}

/// Print restore report
pub fn print_restore_report(report: &crate::cleaner::RestoreReport) {
    println!();
    println!(
        "  {} Restored {} files ({})",
        "✓".green(),
        report.restored_count.to_string().cyan(),
        format_size_colored(report.restored_bytes),
    );
    println!("  {} Session: {}", "📦", report.session_id.cyan());

    if !report.errors.is_empty() {
        println!();
        println!(
            "  {} {} errors during restore:",
            "⚠".yellow(),
            report.errors.len()
        );
        for err in report.errors.iter().take(5) {
            println!("    {} {}", "→".dimmed(), err.dimmed());
        }
    }
    println!();
}

/// Print purge report
pub fn print_purge_report(report: &crate::cleaner::PurgeReport) {
    println!();
    if report.purged_sessions.is_empty() {
        println!("  {} No sessions to purge.", "✓".green());
    } else {
        println!(
            "  {} Purged {} sessions, freed {}",
            "🔥",
            report.purged_sessions.len().to_string().cyan(),
            format_size_colored(report.total_bytes_freed),
        );
        for session in &report.purged_sessions {
            println!(
                "    {} {} ({})",
                "✗".red(),
                session.session_id,
                format_size(session.bytes_freed),
            );
        }
    }

    if !report.errors.is_empty() {
        println!();
        for err in &report.errors {
            println!("    {} {}", "⚠".yellow(), err.dimmed());
        }
    }
    println!();
}

/// Print staging health warning if needed
pub fn print_staging_health(health: &crate::cleaner::StagingHealth) {
    if let Some(ref warning) = health.warning {
        println!("  {} {}", "⚠".yellow(), warning.yellow());
        println!();
    }
}

/// Print duplicate scan results in human-readable format
pub fn print_dup_results(results: &crate::duplicates::DupResults, detailed: bool) {
    println!();
    println!("  {} TidyMac Duplicate Scan", "👯");
    println!("{}", "─".repeat(60).dimmed());
    println!(
        "  Scanned {} files in {}",
        results.files_scanned.to_string().cyan(),
        format::format_duration(results.duration_secs).cyan()
    );
    println!("{}", "─".repeat(60).dimmed());
    println!();

    if results.exact_groups.is_empty() && results.similar_groups.is_empty() {
        println!("  {} No duplicates found!", "✨");
        println!();
        return;
    }

    // Exact duplicates
    if !results.exact_groups.is_empty() {
        let exact_wasted: u64 = results.exact_groups.iter().map(|g| g.wasted_bytes).sum();
        println!(
            "  {} {} ({} groups, {} wasted)",
            "●".red(),
            "Exact Duplicates".red().bold(),
            results.exact_groups.len(),
            format_size_colored(exact_wasted),
        );
        println!();

        for (i, group) in results.exact_groups.iter().enumerate() {
            println!(
                "    Group {} — {} files, {} wasted",
                (i + 1).to_string().bold(),
                group.members.len(),
                format_size(group.wasted_bytes),
            );

            if detailed {
                for (j, member) in group.members.iter().enumerate() {
                    let label = if j == 0 { "keep →" } else { "  dup →" };
                    let color_path = if j == 0 {
                        format_path(&member.path).green().to_string()
                    } else {
                        format_path(&member.path).dimmed().to_string()
                    };
                    println!(
                        "      {} {} ({})",
                        label.dimmed(),
                        color_path,
                        format_size(member.size_bytes),
                    );
                }
                println!();
            }
        }

        if !detailed && results.exact_groups.len() > 0 {
            println!("      Run with {} to see file paths", "--detailed".cyan());
            println!();
        }
    }

    // Perceptually similar images
    if !results.similar_groups.is_empty() {
        let similar_wasted: u64 = results.similar_groups.iter().map(|g| g.wasted_bytes).sum();
        println!(
            "  {} {} ({} groups, {} wasted)",
            "●".yellow(),
            "Visually Similar Images".yellow().bold(),
            results.similar_groups.len(),
            format_size_colored(similar_wasted),
        );
        println!();

        for (i, group) in results.similar_groups.iter().enumerate() {
            println!(
                "    Group {} — {} images, {} wasted",
                (i + 1).to_string().bold(),
                group.members.len(),
                format_size(group.wasted_bytes),
            );

            if detailed {
                for (j, member) in group.members.iter().enumerate() {
                    let sim_pct = format!("{:.0}%", member.similarity * 100.0);
                    let label = if j == 0 {
                        "best →"
                    } else {
                        &format!(" {}  →", sim_pct)
                    };
                    let color_path = if j == 0 {
                        format_path(&member.path).green().to_string()
                    } else {
                        format_path(&member.path).dimmed().to_string()
                    };
                    println!(
                        "      {} {} ({})",
                        label.dimmed(),
                        color_path,
                        format_size(member.size_bytes),
                    );
                }
                println!();
            }
        }

        if !detailed && results.similar_groups.len() > 0 {
            println!("      Run with {} to see file paths", "--detailed".cyan());
            println!();
        }
    }

    // Errors
    if !results.errors.is_empty() {
        println!("  {} {} warnings:", "⚠".yellow(), results.errors.len());
        for err in results.errors.iter().take(5) {
            println!("    {} {}", "→".dimmed(), err.dimmed());
        }
        if results.errors.len() > 5 {
            println!("    ... and {} more", results.errors.len() - 5);
        }
        println!();
    }

    // Summary
    println!("{}", "─".repeat(60).dimmed());
    println!(
        "  {} {} duplicate groups, {} total wasted space",
        "💾",
        results.total_groups.to_string().cyan(),
        format_size_colored(results.total_wasted),
    );
    println!(
        "  {} {} duplicate files that could be removed",
        "📄",
        results.total_duplicates.to_string().cyan(),
    );
    println!();
}

/// Print duplicate results as JSON
pub fn print_dup_json(results: &crate::duplicates::DupResults) {
    let json = serde_json::json!({
        "files_scanned": results.files_scanned,
        "duration_secs": results.duration_secs,
        "total_groups": results.total_groups,
        "total_wasted": results.total_wasted,
        "total_duplicates": results.total_duplicates,
        "exact_groups": results.exact_groups.iter().map(|g| {
            serde_json::json!({
                "match_type": format!("{}", g.match_type),
                "wasted_bytes": g.wasted_bytes,
                "members": g.members.iter().map(|m| {
                    serde_json::json!({
                        "path": m.path.display().to_string(),
                        "size_bytes": m.size_bytes,
                        "similarity": m.similarity,
                    })
                }).collect::<Vec<_>>(),
            })
        }).collect::<Vec<_>>(),
        "similar_groups": results.similar_groups.iter().map(|g| {
            serde_json::json!({
                "match_type": format!("{}", g.match_type),
                "wasted_bytes": g.wasted_bytes,
                "members": g.members.iter().map(|m| {
                    serde_json::json!({
                        "path": m.path.display().to_string(),
                        "size_bytes": m.size_bytes,
                        "similarity": m.similarity,
                    })
                }).collect::<Vec<_>>(),
            })
        }).collect::<Vec<_>>(),
        "errors": results.errors,
    });
    match serde_json::to_string_pretty(&json) {
        Ok(s) => println!("{}", s),
        Err(e) => eprintln!("Error serializing: {}", e),
    }
}

/// Print list of installed applications
pub fn print_app_list(apps: &[crate::apps::InstalledApp], detailed: bool) {
    println!();
    println!("  {} Installed Applications ({})", "📱", apps.len());
    println!("{}", "─".repeat(70).dimmed());
    println!();

    if apps.is_empty() {
        println!("  No applications found.");
        return;
    }

    println!(
        "  {:<30} {:>10} {:>10} {:>10}  {}",
        "Name".dimmed(),
        "App Size".dimmed(),
        "Leftovers".dimmed(),
        "Total".dimmed(),
        "Source".dimmed(),
    );
    println!("  {}", "─".repeat(68).dimmed());

    for app in apps {
        let leftovers: u64 = app
            .associated_files
            .iter()
            .filter(|a| a.exists)
            .map(|a| a.size)
            .sum();
        println!(
            "  {:<30} {:>10} {:>10} {:>10}  {}",
            format::truncate(&app.name, 30),
            format_size(app.app_size),
            if leftovers > 0 {
                format_size(leftovers)
            } else {
                "-".to_string()
            },
            format_size(app.total_size),
            format!("{}", app.source).dimmed(),
        );

        if detailed {
            if let Some(ref bid) = app.bundle_id {
                println!("    {} Bundle: {}", "↳".dimmed(), bid.dimmed());
            }
            if let Some(ref ver) = app.version {
                println!("    {} Version: {}", "↳".dimmed(), ver.dimmed());
            }
            for assoc in &app.associated_files {
                if assoc.exists {
                    println!(
                        "    {} {} ({}) — {}",
                        "↳".dimmed(),
                        format_path(&assoc.path).dimmed(),
                        format_size(assoc.size),
                        format!("{}", assoc.kind).dimmed(),
                    );
                }
            }
            println!();
        }
    }
    println!();
}

/// Print app info detail view
pub fn print_app_info(app: &crate::apps::InstalledApp) {
    println!();
    println!("  {} {}", "📱", app.name.bold());
    println!("{}", "─".repeat(50).dimmed());

    println!("  Path:       {}", format_path(&app.path));
    if let Some(ref bid) = app.bundle_id {
        println!("  Bundle ID:  {}", bid);
    }
    if let Some(ref ver) = app.version {
        println!("  Version:    {}", ver);
    }
    println!("  Source:     {}", app.source);
    println!("  App size:   {}", format_size(app.app_size));

    let existing: Vec<_> = app.associated_files.iter().filter(|a| a.exists).collect();
    if !existing.is_empty() {
        let assoc_size: u64 = existing.iter().map(|a| a.size).sum();
        println!();
        println!(
            "  {} Associated files ({}, {}):",
            "📁",
            existing.len(),
            format_size(assoc_size)
        );
        for assoc in &existing {
            println!(
                "    {} {} ({})",
                format!("[{}]", assoc.kind).dimmed(),
                format_path(&assoc.path),
                format_size(assoc.size),
            );
        }
    }

    println!();
    println!("  {} Total: {}", "💾", format_size_colored(app.total_size));
    println!();
}

/// Print uninstall report
pub fn print_uninstall_report(report: &crate::apps::UninstallReport) {
    println!();
    let icon = match report.mode {
        crate::apps::uninstaller::UninstallMode::DryRun => "ℹ️",
        crate::apps::uninstaller::UninstallMode::Remove => "✓",
    };
    let label = match report.mode {
        crate::apps::uninstaller::UninstallMode::DryRun => "Dry run",
        crate::apps::uninstaller::UninstallMode::Remove => "Removed",
    };

    println!(
        "  {} {} '{}' — {} items, {}",
        icon,
        label.bold(),
        report.app_name,
        report.files_removed,
        format_size_colored(report.bytes_freed),
    );

    for path in &report.removed_paths {
        println!("    {} {}", "✗".red(), format_path(path).dimmed());
    }

    if !report.errors.is_empty() {
        println!();
        for err in &report.errors {
            println!("    {} {}", "⚠".yellow(), err.dimmed());
        }
    }
    println!();
}

/// Print startup items list
pub fn print_startup_items(items: &[crate::startup::StartupItem]) {
    println!();
    println!("  {} Startup Items ({})", "🚀", items.len());
    println!("{}", "─".repeat(70).dimmed());
    println!();

    if items.is_empty() {
        println!("  No startup items found.");
        return;
    }

    println!(
        "  {:<6} {:<30} {:<15} {}",
        "".dimmed(),
        "Label".dimmed(),
        "Kind".dimmed(),
        "Program".dimmed(),
    );
    println!("  {}", "─".repeat(68).dimmed());

    for item in items {
        let status = if item.enabled {
            "●".green().to_string()
        } else {
            "○".red().to_string()
        };

        let prog = item.program.as_deref().unwrap_or("-");
        let prog_display = if prog.len() > 35 {
            format!("...{}", &prog[prog.len() - 32..])
        } else {
            prog.to_string()
        };

        println!(
            "  {} {:<30} {:<15} {}",
            status,
            format::truncate(&item.label, 30),
            format!("{}", item.kind).dimmed(),
            prog_display.dimmed(),
        );
    }

    println!();
    let enabled = items.iter().filter(|i| i.enabled).count();
    let disabled = items.len() - enabled;
    println!(
        "  {} enabled, {} disabled",
        enabled.to_string().green(),
        disabled.to_string().red()
    );
    println!();
}

/// Print startup item info
pub fn print_startup_info(item: &crate::startup::StartupItem) {
    println!();
    println!("  {} {}", "🚀", item.label.bold());
    println!("{}", "─".repeat(50).dimmed());

    println!("  Name:         {}", item.name);
    println!("  Label:        {}", item.label);
    println!("  Kind:         {}", item.kind);
    println!(
        "  Enabled:      {}",
        if item.enabled {
            "Yes".green().to_string()
        } else {
            "No".red().to_string()
        }
    );
    println!("  Run at load:  {}", item.run_at_load);
    if let Some(ref prog) = item.program {
        println!("  Program:      {}", prog);
    }
    println!("  Plist:        {}", format_path(&item.path));
    println!();
}

/// Print privacy audit report
pub fn print_privacy_report(report: &crate::privacy::PrivacyReport) {
    println!();
    println!("  {} TidyMac Privacy Audit", "🔒");
    println!("{}", "─".repeat(65).dimmed());
    println!();

    // Browser profiles
    if !report.browser_profiles.is_empty() {
        println!("  {} Browsers Detected", "🌐");
        println!();

        for profile in &report.browser_profiles {
            println!(
                "    {} {}",
                "●".cyan(),
                format!("{}", profile.browser).bold(),
            );

            let items: Vec<(&str, u64)> = vec![
                ("Cookies", profile.cookies_size),
                ("History", profile.history_size),
                ("Local Storage", profile.local_storage_size),
                ("Cache", profile.cache_size),
                ("Extensions", profile.extensions_size),
            ];

            for (name, size) in items {
                if size > 0 {
                    println!("      {:<20} {:>10}", name.dimmed(), format_size(size),);
                }
            }
            println!(
                "      {:<20} {:>10}",
                "Total".bold(),
                format_size_colored(profile.total_size),
            );
            println!();
        }
    }

    // Cookie locations
    if !report.cookie_locations.is_empty() {
        println!(
            "  {} Cookie Storage ({} locations)",
            "🍪",
            report.cookie_locations.len()
        );
        println!();

        for loc in report.cookie_locations.iter().take(15) {
            println!(
                "    {:<40} {:>10}",
                format::truncate(&loc.app_name, 40).dimmed(),
                format_size(loc.size),
            );
        }
        if report.cookie_locations.len() > 15 {
            println!("    ... and {} more", report.cookie_locations.len() - 15);
        }
        println!();
    }

    // Tracking apps
    if !report.tracking_apps.is_empty() {
        println!("  {} App Tracking Data", "👁️");
        println!();

        for app in &report.tracking_apps {
            println!(
                "    {:<35} {:>10}  [{}]",
                format::truncate(&app.name, 35),
                format_size(app.data_size),
                format!("{}", app.kind).dimmed(),
            );
        }
        println!();
    }

    // Summary
    println!("{}", "─".repeat(65).dimmed());
    println!(
        "  {} Total privacy-related data: {}",
        "💾",
        format_size_colored(report.total_privacy_data_size),
    );
    println!(
        "  {} Tracker database: {} known domains",
        "🛡️",
        crate::privacy::tracker_database_size(),
    );
    println!();
}

/// Print Docker disk usage report
pub fn print_docker_usage(usage: &crate::scanner::docker::DockerUsage) {
    println!();
    println!("  {} Docker Disk Usage", "🐳");
    println!("{}", "─".repeat(60).dimmed());
    println!();

    if !usage.installed {
        println!("  Docker is not installed.");
        println!();
        return;
    }

    if !usage.running {
        println!("  Docker is installed but not running.");
        println!("  Start Docker Desktop and try again.");
        println!();
        return;
    }

    let categories = [
        &usage.images,
        &usage.containers,
        &usage.volumes,
        &usage.build_cache,
    ];

    println!(
        "  {:<20} {:>8} {:>12} {:>12}",
        "Category".dimmed(),
        "Count".dimmed(),
        "Size".dimmed(),
        "Reclaimable".dimmed(),
    );
    println!("  {}", "─".repeat(56).dimmed());

    for cat in &categories {
        println!(
            "  {:<20} {:>8} {:>12} {:>12}",
            cat.label,
            cat.count,
            format_size(cat.size),
            if cat.reclaimable > 0 {
                format_size_colored(cat.reclaimable).to_string()
            } else {
                "-".to_string()
            },
        );
    }

    println!("  {}", "─".repeat(56).dimmed());
    println!(
        "  {:<20} {:>8} {:>12} {:>12}",
        "Total".bold(),
        "",
        format_size(usage.total_size),
        format_size_colored(usage.reclaimable),
    );
    println!();

    // Show image details
    if !usage.images.details.is_empty() {
        println!("  {} Images:", "📦");
        for img in usage.images.details.iter().take(10) {
            println!(
                "    {} {:>8}  {}",
                img.size.dimmed(),
                img.created.dimmed(),
                format::truncate(&img.name, 45),
            );
        }
        if usage.images.details.len() > 10 {
            println!("    ... and {} more", usage.images.details.len() - 10);
        }
        println!();
    }

    // Show stopped containers
    let stopped: Vec<_> = usage
        .containers
        .details
        .iter()
        .filter(|c| c.status.contains("Exited"))
        .collect();
    if !stopped.is_empty() {
        println!("  {} Stopped Containers ({}):", "⏹️", stopped.len());
        for c in stopped.iter().take(10) {
            println!(
                "    {} {}  {}",
                c.id[..12.min(c.id.len())].dimmed(),
                format::truncate(&c.name, 30),
                c.status.dimmed(),
            );
        }
        println!();
    }

    if usage.reclaimable > 0 {
        println!(
            "  {} Run {} to reclaim {}",
            "💡",
            "tidymac docker --prune".cyan(),
            format_size_colored(usage.reclaimable),
        );
        println!();
    }
}

/// Print Docker prune report
pub fn print_docker_prune_report(report: &crate::scanner::docker::DockerPruneReport) {
    println!();
    println!("  {} Docker prune complete", "🐳",);

    if report.containers_removed > 0 {
        println!(
            "    {} {} containers removed",
            "✗".red(),
            report.containers_removed
        );
    }
    if report.images_removed > 0 {
        println!(
            "    {} {} image layers removed",
            "✗".red(),
            report.images_removed
        );
    }
    if report.volumes_removed > 0 {
        println!(
            "    {} {} volumes removed",
            "✗".red(),
            report.volumes_removed
        );
    }
    if report.build_cache_cleared {
        println!("    {} Build cache cleared", "✗".red());
    }

    if !report.errors.is_empty() {
        println!();
        for err in &report.errors {
            println!("    {} {}", "⚠".yellow(), err.dimmed());
        }
    }
    println!();
}
