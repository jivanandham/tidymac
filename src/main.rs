use anyhow::Result;
use clap::Parser;
use colored::Colorize;

use tidymac::cli::args::{Cli, Commands, ConfigAction, OutputFormat};
use tidymac::cli::output;
use tidymac::cleaner::{self, CleanManifest, CleanMode};
use tidymac::common::config::Config;
use tidymac::common::format;
use tidymac::profiles::loader::Profile;
use tidymac::scanner;

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.no_color {
        colored::control::set_override(false);
    }

    if cli.verbose {
        tracing_subscriber::fmt()
            .with_env_filter("tidymac=debug")
            .init();
    }

    match cli.command {
        Commands::Scan {
            detailed,
            categories: _,
            dry_run: _,
            no_cache,
        } => cmd_scan(&cli, detailed, no_cache),

        Commands::Clean {
            hard,
            yes,
            categories: _,
            max_safety: _,
            dry_run,
        } => cmd_clean(&cli, hard, yes, dry_run),

        Commands::Undo {
            last,
            ref session,
            list,
        } => cmd_undo(&cli, last, session.clone(), list),

        Commands::Purge {
            expired,
            all,
            ref session,
            yes,
            install_auto,
        } => cmd_purge(&cli, expired, all, session.clone(), yes, install_auto),

        Commands::Dup {
            ref path,
            perceptual,
            threshold,
            min_size,
            detailed,
        } => cmd_dup(&cli, path, perceptual, threshold, min_size, detailed),

        Commands::Apps { ref action } => cmd_apps(&cli, action),

        Commands::Startup { ref action } => cmd_startup(&cli, action),

        Commands::Privacy { ref action } => cmd_privacy(&cli, action),

        Commands::Viz { interactive } => cmd_viz(&cli, interactive),

        Commands::Docker { prune, dry_run, yes } => cmd_docker(&cli, prune, dry_run, yes),

        Commands::Config { action } => cmd_config(action),
        Commands::Status => cmd_status(),

        Commands::Completions { shell } => {
            use clap::CommandFactory;
            let mut cmd = Cli::command();
            let shell = match shell {
                tidymac::cli::args::CompletionShell::Bash => clap_complete::Shell::Bash,
                tidymac::cli::args::CompletionShell::Zsh => clap_complete::Shell::Zsh,
                tidymac::cli::args::CompletionShell::Fish => clap_complete::Shell::Fish,
            };
            clap_complete::generate(shell, &mut cmd, "tidymac", &mut std::io::stdout());
            Ok(())
        }
    }
}

// ─── Scan ─────────────────────────────────────────────────────────────────────

fn cmd_scan(cli: &Cli, detailed: bool, no_cache: bool) -> Result<()> {
    let profile_name = cli.profile.as_deref().unwrap_or("quick");
    let profile = Profile::load(profile_name)?;
    let config = Config::load()?;

    if !cli.quiet {
        output::print_profile_info(&profile);
    }

    let scan_targets = profile.enabled_targets();
    let show_progress = !cli.quiet && matches!(cli.format, OutputFormat::Human);

    let results = scanner::run_scan_with_cache(
        &scan_targets,
        show_progress,
        profile.includes_dev_projects(),
        profile.thresholds.stale_days,
        config.large_file_threshold_bytes(),
        !no_cache,
        profile_name,
    )?;

    match cli.format {
        OutputFormat::Human => output::print_scan_results(&results, detailed),
        OutputFormat::Json => output::print_scan_json(&results),
        OutputFormat::Quiet => output::print_scan_quiet(&results),
    }

    Ok(())
}

// ─── Dup ──────────────────────────────────────────────────────────────────────

fn cmd_dup(
    cli: &Cli,
    path: &str,
    perceptual: bool,
    threshold: f64,
    min_size: u64,
    detailed: bool,
) -> Result<()> {
    // Expand ~ in path
    let root = if path.starts_with('~') {
        let home = dirs::home_dir().unwrap_or_default();
        home.join(path.strip_prefix("~/").unwrap_or(path.strip_prefix('~').unwrap_or(path)))
    } else {
        std::path::PathBuf::from(path)
    };

    if !root.exists() {
        anyhow::bail!("Path does not exist: {}", root.display());
    }

    let show_progress = !cli.quiet && matches!(cli.format, OutputFormat::Human);

    if show_progress {
        println!();
        println!(
            "  {} Scanning for duplicates in: {}",
            "🔍",
            format::format_path(&root).cyan()
        );
        if perceptual {
            println!(
                "  {} Perceptual image matching enabled (threshold: {:.0}%)",
                "🖼️", threshold * 100.0
            );
        }
        println!();
    }

    let config = tidymac::duplicates::DupConfig {
        root,
        min_size,
        perceptual,
        threshold,
        show_progress,
    };

    let results = tidymac::duplicates::find_duplicates(&config)?;

    match cli.format {
        OutputFormat::Human => output::print_dup_results(&results, detailed),
        OutputFormat::Json => output::print_dup_json(&results),
        OutputFormat::Quiet => {
            println!(
                "{}  {}  {}",
                results.total_groups,
                results.total_duplicates,
                format::format_size(results.total_wasted)
            );
        }
    }

    Ok(())
}

// ─── Clean ────────────────────────────────────────────────────────────────────

fn cmd_clean(cli: &Cli, hard: bool, yes: bool, dry_run: bool) -> Result<()> {
    let profile_name = cli.profile.as_deref().unwrap_or("quick");
    let profile = Profile::load(profile_name)?;
    let config = Config::load()?;

    if !cli.quiet {
        output::print_profile_info(&profile);
    }

    // Check staging health before soft-delete
    if !hard && !dry_run {
        let health = cleaner::check_staging_health()?;
        if !cli.quiet {
            output::print_staging_health(&health);
        }
    }

    let scan_targets = profile.enabled_targets();
    let show_progress = !cli.quiet && matches!(cli.format, OutputFormat::Human);

    let results = scanner::run_scan(
        &scan_targets,
        show_progress,
        profile.includes_dev_projects(),
        profile.thresholds.stale_days,
        config.large_file_threshold_bytes(),
    )?;

    if results.items.is_empty() {
        println!("  {} Nothing to clean!", "✨");
        return Ok(());
    }

    // Show what would be cleaned
    if matches!(cli.format, OutputFormat::Human) {
        output::print_scan_results(&results, false);
    }

    // Determine mode
    let mode = if dry_run {
        CleanMode::DryRun
    } else if hard {
        CleanMode::HardDelete
    } else {
        CleanMode::SoftDelete
    };

    if mode == CleanMode::DryRun {
        let report = cleaner::clean(&results.items, mode, profile_name, false)?;
        println!(
            "  {} Dry run — would clean {} files ({}). No files modified.",
            "ℹ️",
            report.files_removed,
            format::format_size(report.bytes_freed)
        );
        return Ok(());
    }

    // Confirm unless --yes
    if !yes {
        let mode_label = if hard { "PERMANENTLY DELETE" } else { "soft delete" };
        print!(
            "\n  {} {} {} ({})? [y/N] ",
            "❓",
            mode_label,
            format::format_count(results.total_files),
            format::format_size(results.total_reclaimable)
        );
        use std::io::Write;
        std::io::stdout().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("  {} Cancelled", "✗".red());
            return Ok(());
        }
    }

    let report = cleaner::clean(&results.items, mode, profile_name, show_progress)?;

    match cli.format {
        OutputFormat::Human => output::print_clean_report(&report),
        OutputFormat::Json => {
            let json = serde_json::json!({
                "mode": format!("{}", report.mode),
                "files_removed": report.files_removed,
                "bytes_freed": report.bytes_freed,
                "session_id": report.session_id,
                "errors": report.errors,
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        OutputFormat::Quiet => {
            println!(
                "{}  {}  {}",
                format::format_size(report.bytes_freed),
                report.files_removed,
                report.session_id.as_deref().unwrap_or("none")
            );
        }
    }

    Ok(())
}

// ─── Privacy ──────────────────────────────────────────────────────────────────

fn cmd_privacy(cli: &Cli, action: &tidymac::cli::args::PrivacyAction) -> Result<()> {
    use tidymac::cli::args::PrivacyAction;

    match action {
        PrivacyAction::Scan { browsers, cookies, all } => {
            let scan_browsers = *browsers || *all;
            let scan_cookies = *cookies || *all;

            // Default to all if no flags specified
            let (scan_browsers, scan_cookies) = if !scan_browsers && !scan_cookies {
                (true, true)
            } else {
                (scan_browsers, scan_cookies)
            };

            if !cli.quiet {
                println!();
                println!("  {} Running privacy audit...", "🔒");
                println!();
            }

            let report = tidymac::privacy::run_privacy_audit(scan_browsers, scan_cookies);

            match cli.format {
                OutputFormat::Human => output::print_privacy_report(&report),
                OutputFormat::Json => {
                    let json = serde_json::json!({
                        "browser_profiles": report.browser_profiles.iter().map(|p| {
                            serde_json::json!({
                                "browser": format!("{}", p.browser),
                                "cookies_size": p.cookies_size,
                                "history_size": p.history_size,
                                "local_storage_size": p.local_storage_size,
                                "cache_size": p.cache_size,
                                "total_size": p.total_size,
                            })
                        }).collect::<Vec<_>>(),
                        "cookie_locations": report.cookie_locations.len(),
                        "tracking_apps": report.tracking_apps.len(),
                        "total_privacy_data_size": report.total_privacy_data_size,
                    });
                    println!("{}", serde_json::to_string_pretty(&json)?);
                }
                OutputFormat::Quiet => {
                    println!(
                        "{}  {}  {}",
                        report.browser_profiles.len(),
                        report.cookie_locations.len(),
                        format::format_size(report.total_privacy_data_size),
                    );
                }
            }
            Ok(())
        }

        PrivacyAction::Clean { yes, dry_run } => {
            // Run audit first to show what would be cleaned
            let report = tidymac::privacy::run_privacy_audit(true, true);
            output::print_privacy_report(&report);

            if *dry_run {
                println!(
                    "  {} Dry run — no files modified. {} of privacy data found.",
                    "ℹ️",
                    format::format_size(report.total_privacy_data_size)
                );
                return Ok(());
            }

            if !yes {
                print!(
                    "  {} Clean all browser caches and cookie data ({})? [y/N] ",
                    "❓",
                    format::format_size(report.total_privacy_data_size)
                );
                use std::io::Write;
                std::io::stdout().flush()?;
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                if !input.trim().eq_ignore_ascii_case("y") {
                    println!("  {} Cancelled", "✗".red());
                    return Ok(());
                }
            }

            // Clean browser caches
            let mut freed = 0u64;
            let mut cleaned = 0usize;

            for profile in &report.browser_profiles {
                if let Some(ref p) = profile.cache_path {
                    if p.exists() {
                        if let Ok(()) = std::fs::remove_dir_all(p) {
                            freed += profile.cache_size;
                            cleaned += 1;
                        }
                    }
                }
            }

            // Clean cookie locations
            for loc in &report.cookie_locations {
                if loc.path.exists() {
                    let result = if loc.path.is_dir() {
                        std::fs::remove_dir_all(&loc.path)
                    } else {
                        std::fs::remove_file(&loc.path)
                    };
                    if result.is_ok() {
                        freed += loc.size;
                        cleaned += 1;
                    }
                }
            }

            println!(
                "  {} Cleaned {} items, freed {}",
                "✓".green(),
                cleaned,
                format::format_size(freed)
            );
            println!();
            Ok(())
        }
    }
}

// ─── Viz ──────────────────────────────────────────────────────────────────────

fn cmd_viz(cli: &Cli, _interactive: bool) -> Result<()> {
    if !cli.quiet {
        println!();
        println!("  {} Analyzing disk usage...", "📊");
    }

    let usage = tidymac::viz::analyze_disk_usage();

    match cli.format {
        OutputFormat::Human => tidymac::viz::print_viz(&usage),
        OutputFormat::Json => tidymac::viz::print_viz_json(&usage),
        OutputFormat::Quiet => {
            println!(
                "{}  {}  {}",
                format::format_size(usage.used),
                format::format_size(usage.available),
                format::format_size(usage.total_capacity)
            );
        }
    }

    Ok(())
}

// ─── Docker ───────────────────────────────────────────────────────────────────

fn cmd_docker(cli: &Cli, prune: bool, dry_run: bool, yes: bool) -> Result<()> {
    use tidymac::scanner::docker;

    if !docker::is_docker_installed() {
        println!("  Docker is not installed on this system.");
        return Ok(());
    }

    if !docker::is_docker_running() {
        println!("  Docker is installed but not running. Start Docker Desktop and try again.");
        return Ok(());
    }

    if prune {
        if dry_run {
            let usage = docker::get_docker_usage();
            output::print_docker_usage(&usage);
            println!(
                "  {} Dry run — would reclaim approximately {}. No changes made.",
                "ℹ️",
                format::format_size(usage.reclaimable)
            );
            return Ok(());
        }

        if !yes {
            let usage = docker::get_docker_usage();
            output::print_docker_usage(&usage);

            print!(
                "  {} Prune dangling images, stopped containers, volumes, and build cache? [y/N] ",
                "❓"
            );
            use std::io::Write;
            std::io::stdout().flush()?;
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            if !input.trim().eq_ignore_ascii_case("y") {
                println!("  {} Cancelled", "✗".red());
                return Ok(());
            }
        }

        let report = docker::prune_dangling(false)?;
        output::print_docker_prune_report(&report);
    } else {
        // Default: show usage
        let usage = docker::get_docker_usage();

        match cli.format {
            OutputFormat::Human => output::print_docker_usage(&usage),
            OutputFormat::Json => {
                match serde_json::to_string_pretty(&usage) {
                    Ok(s) => println!("{}", s),
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
            OutputFormat::Quiet => {
                println!(
                    "{}  {}  {}",
                    format::format_size(usage.total_size),
                    format::format_size(usage.reclaimable),
                    if usage.running { "running" } else { "stopped" }
                );
            }
        }
    }

    Ok(())
}

// ─── Apps ─────────────────────────────────────────────────────────────────────

fn cmd_apps(cli: &Cli, action: &tidymac::cli::args::AppsAction) -> Result<()> {
    use tidymac::cli::args::{AppsAction, AppSort};

    match action {
        AppsAction::List { sort, detailed, unused_days } => {
            let show_progress = !cli.quiet && matches!(cli.format, OutputFormat::Human);
            if show_progress {
                println!();
                println!("  {} Scanning applications...", "🔍");
            }

            let mut apps = tidymac::apps::discover_apps();

            // Filter by unused days
            if let Some(days) = unused_days {
                let threshold = std::time::SystemTime::now()
                    - std::time::Duration::from_secs(*days as u64 * 86400);
                apps.retain(|a| {
                    a.last_opened
                        .map(|t| t < threshold)
                        .unwrap_or(true)
                });
            }

            // Sort
            match sort {
                AppSort::Name => apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
                AppSort::Size => apps.sort_by(|a, b| b.total_size.cmp(&a.total_size)),
                AppSort::LastOpened => apps.sort_by(|a, b| b.last_opened.cmp(&a.last_opened)),
            }

            match cli.format {
                OutputFormat::Human => output::print_app_list(&apps, *detailed),
                OutputFormat::Json => {
                    let json: Vec<_> = apps.iter().map(|a| {
                        serde_json::json!({
                            "name": a.name,
                            "bundle_id": a.bundle_id,
                            "version": a.version,
                            "path": a.path.display().to_string(),
                            "app_size": a.app_size,
                            "total_size": a.total_size,
                            "source": format!("{}", a.source),
                            "associated_files": a.associated_files.iter().filter(|f| f.exists).count(),
                        })
                    }).collect();
                    println!("{}", serde_json::to_string_pretty(&json)?);
                }
                OutputFormat::Quiet => {
                    for a in &apps {
                        println!("{}  {}", a.name, format::format_size(a.total_size));
                    }
                }
            }
            Ok(())
        }

        AppsAction::Info { ref name } => {
            let apps = tidymac::apps::discover_apps();
            let matches = tidymac::apps::find_app_by_name(&apps, name);

            if matches.is_empty() {
                println!("  No app found matching '{}'", name);
                return Ok(());
            }

            for app in matches {
                output::print_app_info(app);
            }
            Ok(())
        }

        AppsAction::Remove { ref name, dry_run, yes } => {
            let apps = tidymac::apps::discover_apps();
            let matches = tidymac::apps::find_app_by_name(&apps, name);

            if matches.is_empty() {
                println!("  No app found matching '{}'", name);
                return Ok(());
            }
            if matches.len() > 1 {
                println!("  Multiple apps match '{}'. Be more specific:", name);
                for app in &matches {
                    println!("    {} {}", "•".dimmed(), app.name);
                }
                return Ok(());
            }

            let app = matches[0];
            output::print_app_info(app);

            if !dry_run && !yes {
                print!(
                    "  {} Remove '{}' and all associated files ({})? [y/N] ",
                    "❓", app.name, format::format_size(app.total_size)
                );
                use std::io::Write;
                std::io::stdout().flush()?;
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                if !input.trim().eq_ignore_ascii_case("y") {
                    println!("  {} Cancelled", "✗".red());
                    return Ok(());
                }
            }

            let report = tidymac::apps::uninstall_app(app, *dry_run)?;
            output::print_uninstall_report(&report);
            Ok(())
        }
    }
}

// ─── Startup ──────────────────────────────────────────────────────────────────

fn cmd_startup(cli: &Cli, action: &tidymac::cli::args::StartupAction) -> Result<()> {
    use tidymac::cli::args::StartupAction;

    match action {
        StartupAction::List => {
            let items = tidymac::startup::discover_startup_items();

            match cli.format {
                OutputFormat::Human => output::print_startup_items(&items),
                OutputFormat::Json => {
                    let json: Vec<_> = items.iter().map(|i| {
                        serde_json::json!({
                            "label": i.label,
                            "name": i.name,
                            "kind": format!("{}", i.kind),
                            "enabled": i.enabled,
                            "program": i.program,
                            "run_at_load": i.run_at_load,
                            "path": i.path.display().to_string(),
                        })
                    }).collect();
                    println!("{}", serde_json::to_string_pretty(&json)?);
                }
                OutputFormat::Quiet => {
                    for i in &items {
                        let status = if i.enabled { "on" } else { "off" };
                        println!("{}  {}  {}", i.label, status, format!("{}", i.kind));
                    }
                }
            }
            Ok(())
        }

        StartupAction::Info { ref name } => {
            let items = tidymac::startup::discover_startup_items();
            let matches = tidymac::startup::find_item_by_name(&items, name);

            if matches.is_empty() {
                println!("  No startup item found matching '{}'", name);
            } else {
                for item in matches {
                    output::print_startup_info(item);
                }
            }
            Ok(())
        }

        StartupAction::Disable { ref name } => {
            let items = tidymac::startup::discover_startup_items();
            let matches = tidymac::startup::find_item_by_name(&items, name);

            if matches.is_empty() {
                println!("  No startup item found matching '{}'", name);
                return Ok(());
            }

            for item in matches {
                match tidymac::startup::disable_item(item) {
                    Ok(msg) => println!("  {} {}", "✓".green(), msg),
                    Err(e) => println!("  {} {}", "✗".red(), e),
                }
            }
            Ok(())
        }

        StartupAction::Enable { ref name } => {
            let items = tidymac::startup::discover_startup_items();
            let matches = tidymac::startup::find_item_by_name(&items, name);

            if matches.is_empty() {
                println!("  No startup item found matching '{}'", name);
                return Ok(());
            }

            for item in matches {
                match tidymac::startup::enable_item(item) {
                    Ok(msg) => println!("  {} {}", "✓".green(), msg),
                    Err(e) => println!("  {} {}", "✗".red(), e),
                }
            }
            Ok(())
        }
    }
}

// ─── Undo ─────────────────────────────────────────────────────────────────────

fn cmd_undo(cli: &Cli, last: bool, session: Option<String>, list: bool) -> Result<()> {
    if list {
        let sessions = CleanManifest::list_sessions()?;
        match cli.format {
            OutputFormat::Human => output::print_sessions(&sessions),
            OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&sessions)?),
            OutputFormat::Quiet => {
                for s in &sessions {
                    let status = if s.restored { "restored" }
                        else if s.is_expired { "expired" }
                        else { "active" };
                    println!("{}  {}  {}  {}", s.session_id, s.profile, format::format_size(s.staged_size), status);
                }
            }
        }
        return Ok(());
    }

    // Determine session to restore
    let session_id = if last {
        CleanManifest::most_recent_session()?
            .ok_or_else(|| anyhow::anyhow!("No sessions found in staging area"))?
    } else if let Some(sid) = session {
        sid
    } else {
        // Show help + active sessions
        println!();
        println!("  {} Undo a previous cleanup operation", "↩️");
        println!();
        println!("  Usage:");
        println!("    {} Restore last session", "tidymac undo --last".cyan());
        println!("    {} Restore specific", "tidymac undo --session <ID>".cyan());
        println!("    {} List all", "tidymac undo --list".cyan());
        println!();

        let sessions = CleanManifest::list_sessions()?;
        let active: Vec<_> = sessions.iter().filter(|s| !s.restored && !s.is_expired).collect();
        if !active.is_empty() {
            println!("  Active sessions:");
            for s in active.iter().take(5) {
                println!(
                    "    {} {} — {} ({} files)",
                    "•".dimmed(), s.session_id, format::format_size(s.staged_size), s.total_files
                );
            }
            println!();
        }
        return Ok(());
    };

    // Validate the session
    let manifest = CleanManifest::load_from_session(&session_id)?;
    if manifest.restored {
        println!("  {} Session '{}' was already restored.", "ℹ️", session_id);
        return Ok(());
    }

    let restorable_count = manifest.items.iter().filter(|i| i.success && i.staged_path.is_some()).count();

    println!();
    println!(
        "  {} Restoring session '{}' — {} files ({})",
        "↩️", session_id.cyan(), restorable_count, format::format_size(manifest.total_bytes)
    );

    let show_progress = !cli.quiet && matches!(cli.format, OutputFormat::Human);
    let report = cleaner::restore_session(&session_id, show_progress)?;

    match cli.format {
        OutputFormat::Human => output::print_restore_report(&report),
        OutputFormat::Json => {
            let json = serde_json::json!({
                "session_id": report.session_id,
                "restored_count": report.restored_count,
                "restored_bytes": report.restored_bytes,
                "errors": report.errors,
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        OutputFormat::Quiet => {
            println!("{}  {}  {}", report.session_id, report.restored_count, format::format_size(report.restored_bytes));
        }
    }

    Ok(())
}

// ─── Purge ────────────────────────────────────────────────────────────────────

fn cmd_purge(
    _cli: &Cli,
    expired: bool,
    all: bool,
    session: Option<String>,
    yes: bool,
    install_auto: bool,
) -> Result<()> {
    // Install auto-purge launchd plist
    if install_auto {
        let plist_content = cleaner::purger::generate_purge_plist();
        let plist_path = cleaner::purger::purge_plist_path();

        if let Some(parent) = plist_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&plist_path, &plist_content)?;

        println!("  {} Installed auto-purge plist: {}", "✓".green(), plist_path.display());
        println!("  {} Load with: {}", "💡", format!("launchctl load {}", plist_path.display()).cyan());
        println!("  Expired sessions will be purged daily at 3:00 AM.");
        println!();
        return Ok(());
    }

    // Purge specific session
    if let Some(sid) = session {
        if !yes {
            print!("  {} Permanently purge session '{}'? [y/N] ", "❓", sid);
            use std::io::Write;
            std::io::stdout().flush()?;
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            if !input.trim().eq_ignore_ascii_case("y") {
                println!("  {} Cancelled", "✗".red());
                return Ok(());
            }
        }
        let freed = cleaner::purge_session(&sid)?;
        println!("  {} Purged '{}', freed {}", "🔥", sid, format::format_size(freed));
        return Ok(());
    }

    // Purge all
    if all {
        let sessions = CleanManifest::list_sessions()?;
        let total: u64 = sessions.iter().map(|s| s.staged_size).sum();

        if !yes {
            print!(
                "  {} Permanently purge ALL {} sessions ({})? [y/N] ",
                "❓", sessions.len(), format::format_size(total)
            );
            use std::io::Write;
            std::io::stdout().flush()?;
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            if !input.trim().eq_ignore_ascii_case("y") {
                println!("  {} Cancelled", "✗".red());
                return Ok(());
            }
        }

        let report = cleaner::purge_all()?;
        output::print_purge_report(&report);
        return Ok(());
    }

    // Purge expired
    if expired {
        let report = cleaner::purge_expired()?;
        output::print_purge_report(&report);
        return Ok(());
    }

    // No flags — show help
    println!();
    println!("  {} Purge staging area sessions", "🔥");
    println!();
    println!("  Usage:");
    println!("    {} Expired only", "tidymac purge --expired".cyan());
    println!("    {} ALL sessions", "tidymac purge --all".cyan());
    println!("    {} Specific session", "tidymac purge --session <ID>".cyan());
    println!("    {} Auto-purge daily", "tidymac purge --install-auto".cyan());
    println!();

    let health = cleaner::check_staging_health()?;
    println!("  Staging: {} sessions, {} total", health.session_count, format::format_size(health.total_size));
    if health.expired_count > 0 {
        println!(
            "  {} {} expired ({}) — run {}",
            "⚠".yellow(), health.expired_count, format::format_size(health.expired_size), "tidymac purge --expired".cyan()
        );
    }
    println!();

    Ok(())
}

// ─── Config ───────────────────────────────────────────────────────────────────

fn cmd_config(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Init => {
            Config::init_dirs()?;
            let config = Config::default();
            config.save()?;
            println!("  {} TidyMac initialized at ~/.tidymac", "✓".green());
            println!("  Created: config.toml, staging/, logs/, profiles/");
            Ok(())
        }
        ConfigAction::Show => {
            let config = Config::load()?;
            println!("{}", toml::to_string_pretty(&config)?);
            Ok(())
        }
        ConfigAction::Reset => {
            let config = Config::default();
            config.save()?;
            println!("  {} Configuration reset to defaults", "✓".green());
            Ok(())
        }
        ConfigAction::Set { key, value } => {
            let mut config = Config::load()?;
            match key.as_str() {
                "stale_days" => config.stale_days = value.parse()?,
                "large_file_threshold_mb" => config.large_file_threshold_mb = value.parse()?,
                "staging_retention_days" => config.staging_retention_days = value.parse()?,
                "default_profile" => config.default_profile = value.clone(),
                _ => anyhow::bail!("Unknown config key: {}", key),
            }
            config.save()?;
            println!("  {} Set {} = {}", "✓".green(), key, value);
            Ok(())
        }
        ConfigAction::ClearCache => {
            tidymac::scanner::cache::ScanCache::clear()?;
            println!("  {} Scan cache cleared. Next scan will be fresh.", "✓".green());
            Ok(())
        }
    }
}

// ─── Status ───────────────────────────────────────────────────────────────────

fn cmd_status() -> Result<()> {
    let config = Config::load()?;

    println!();
    println!("  {} TidyMac Status", "📊");
    println!("{}", "─".repeat(60).dimmed());
    println!();

    println!("  {} Default profile: {}", "⚙️", config.default_profile);
    println!("  {} Staging retention: {} days", "⚙️", config.staging_retention_days);
    println!("  {} Large file threshold: {} MB", "⚙️", config.large_file_threshold_mb);

    // Staging health
    let health = cleaner::check_staging_health()?;
    println!();
    println!(
        "  {} Staging: {} sessions, {} used",
        "📦", health.session_count, format::format_size(health.total_size)
    );
    if health.expired_count > 0 {
        println!(
            "  {} {} expired ({}) — run {}",
            "⚠".yellow(), health.expired_count, format::format_size(health.expired_size), "tidymac purge --expired".cyan()
        );
    }
    output::print_staging_health(&health);

    // Scan cache info
    if let Some(cache) = tidymac::scanner::cache::ScanCache::load(&config.default_profile) {
        println!(
            "  {} Scan cache: {} entries, updated {}",
            "⚡", cache.entry_count(), cache.age_string()
        );
    } else {
        println!("  {} Scan cache: empty (first scan will populate)", "⚡");
    }

    // Recent sessions
    let sessions = CleanManifest::list_sessions()?;
    if !sessions.is_empty() {
        println!("  {} Recent sessions:", "📋");
        for s in sessions.iter().take(5) {
            let status = if s.restored { "restored".green().to_string() }
                else if s.is_expired { "expired".red().to_string() }
                else { "active".yellow().to_string() };
            println!(
                "    {} {} — {} ({} files) [{}]",
                "•".dimmed(), s.session_id, format::format_size(s.total_bytes), s.total_files, status
            );
        }
    }

    println!();
    println!("  {} Profiles:", "📋");
    for name in Profile::available_profiles() {
        println!("    {} {}", "•".dimmed(), name);
    }
    println!();

    Ok(())
}
