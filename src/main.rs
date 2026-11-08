use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use std::path::PathBuf;

use tidymac::cleaner::{self, CleanManifest, CleanMode};
use tidymac::cli::args::{Cli, Commands, ConfigAction, OutputFormat, SafetyFilter};
use tidymac::cli::output;
use tidymac::common::config::Config;
use tidymac::common::format;
use tidymac::profiles::loader::Profile;
use tidymac::scanner;
use tidymac::scanner::targets::SafetyLevel;

use tidymac::common::observability;

fn setup_logging(verbose: bool) -> Result<()> {
    observability::init(verbose)
}

/// Ask a yes/no question on the terminal.
fn confirm(question: &str) -> Result<bool> {
    use std::io::Write;
    print!("\n  {} {} [y/N] ", "❓", question);
    std::io::stdout().flush()?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(input.trim().eq_ignore_ascii_case("y"))
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.no_color {
        colored::control::set_override(false);
    }

    setup_logging(cli.verbose).unwrap_or_else(|e| {
        eprintln!("Failed to initialize logging: {}", e);
    });

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
            ref categories,
            ref max_safety,
            dry_run,
            force,
        } => cmd_clean(&cli, hard, yes, categories.clone(), max_safety.clone(), dry_run, force),

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
            remove,
            yes,
        } => cmd_dup(&cli, path, perceptual, threshold, min_size, detailed, remove, yes),

        Commands::Map { ref path, depth, top } => cmd_map(&cli, path.clone(), depth, top),
        Commands::SystemData => cmd_system_data(&cli),
        Commands::Schedule { ref action } => cmd_schedule(&cli, action),
        Commands::Report => cmd_report(&cli),

        Commands::Apps { ref action } => cmd_apps(&cli, action),

        Commands::Startup { ref action } => cmd_startup(&cli, action),

        Commands::Privacy { ref action } => cmd_privacy(&cli, action),

        Commands::Viz { interactive } => cmd_viz(&cli, interactive),

        Commands::Docker {
            prune,
            dry_run,
            yes,
        } => cmd_docker(&cli, prune, dry_run, yes),

        Commands::Config { ref action } => cmd_config(action),
        Commands::Status { diagnostics } => cmd_status(diagnostics),

        Commands::Completions { ref shell } => {
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

    let _ = config;
    let mut results = scanner::run_scan_with_cache(
        &scan_targets,
        show_progress,
        profile.includes_dev_projects(),
        profile.thresholds.stale_days,
        profile.large_file_threshold(),
        !no_cache,
        profile_name,
    )?;
    results.errors.extend(profile.rule_problems());

    match cli.format {
        OutputFormat::Human => output::print_scan_results(&results, detailed),
        OutputFormat::Json => output::print_scan_json(&results),
        OutputFormat::Quiet => output::print_scan_quiet(&results),
    }

    Ok(())
}

// ─── Dup ──────────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn cmd_dup(
    cli: &Cli,
    path: &str,
    perceptual: bool,
    threshold: f64,
    min_size: u64,
    detailed: bool,
    remove: bool,
    yes: bool,
) -> Result<()> {
    // Expand ~ in path
    let root = if path.starts_with('~') {
        let home = dirs::home_dir().unwrap_or_default();
        home.join(
            path.strip_prefix("~/")
                .unwrap_or(path.strip_prefix('~').unwrap_or(path)),
        )
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
                "🖼️",
                threshold * 100.0
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

    if remove && !results.exact_groups.is_empty() {
        // Exact duplicates only — "similar" images are never removed automatically.
        let resolved = tidymac::duplicates::resolve_all(
            &results.exact_groups,
            &tidymac::duplicates::ResolveStrategy::KeepLargest,
        );
        let paths: Vec<PathBuf> = resolved.iter().flat_map(|r| r.remove.clone()).collect();
        if paths.is_empty() {
            return Ok(());
        }
        if !yes
            && !confirm(&format!(
                "Move {} duplicate copies to the undo area (one copy of each file is kept)?",
                paths.len()
            ))?
        {
            println!("  {} Cancelled", "✗".red());
            return Ok(());
        }
        let requests = paths
            .into_iter()
            .map(|p| {
                let size = std::fs::metadata(&p)
                    .map(|m| {
                        use std::os::unix::fs::MetadataExt;
                        m.blocks() * 512
                    })
                    .unwrap_or(0);
                cleaner::DeleteRequest {
                    path: p,
                    size_bytes: size,
                    category: "Duplicate".into(),
                    safety: "Caution".into(),
                    roots: Vec::new(),
                    policy: tidymac::common::safety::DeletePolicy::UserFile,
                }
            })
            .collect();
        let mut opts = cleaner::DeleteOptions::new(CleanMode::SoftDelete, "duplicates");
        opts.show_progress = show_progress;
        let report = cleaner::execute(requests, Vec::new(), &opts)?;
        output::print_clean_report(&report);
    }

    Ok(())
}

// ─── Clean ────────────────────────────────────────────────────────────────────

fn safety_allowed(level: &SafetyLevel, max: &SafetyFilter) -> bool {
    match max {
        SafetyFilter::Safe => *level == SafetyLevel::Safe,
        SafetyFilter::Caution => *level != SafetyLevel::Dangerous,
        SafetyFilter::Dangerous => true,
    }
}

fn cmd_clean(
    cli: &Cli,
    hard: bool,
    yes: bool,
    categories: Option<Vec<String>>,
    max_safety: SafetyFilter,
    dry_run: bool,
    force: bool,
) -> Result<()> {
    let profile_name = cli.profile.as_deref().unwrap_or("quick");
    let profile = Profile::load(profile_name)?;

    if !cli.quiet {
        output::print_profile_info(&profile);
        for p in profile.rule_problems() {
            println!("  {} {}", "⚠".yellow(), p);
        }
    }

    if !hard && !dry_run {
        let health = cleaner::check_staging_health()?;
        if !cli.quiet {
            output::print_staging_health(&health);
        }
    }

    let show_progress = !cli.quiet && matches!(cli.format, OutputFormat::Human);

    // Always a fresh scan (never the cache): cleaning needs exact entry lists.
    let mut results = scanner::run_scan(
        &profile.enabled_targets(),
        show_progress,
        profile.includes_dev_projects(),
        profile.thresholds.stale_days,
        None, // large files are review-only; never part of a bulk clean
    )?;

    // Filter by --max-safety and --categories
    let wanted: Vec<String> = categories
        .unwrap_or_default()
        .into_iter()
        .map(|c| c.to_lowercase())
        .collect();
    let mut excluded_by_safety = Vec::new();
    results.items.retain(|item| {
        if !wanted.is_empty() {
            let cat = item.category.to_string().to_lowercase();
            let name = item.name.to_lowercase();
            if !wanted.iter().any(|w| cat.contains(w) || name.contains(w)) {
                return false;
            }
        }
        if !safety_allowed(&item.safety, &max_safety) {
            excluded_by_safety.push(format!("{} ({})", item.name, format::format_size(item.size_bytes)));
            return false;
        }
        true
    });
    results.recalculate();

    if results.items.is_empty() {
        println!("  {} Nothing to clean!", "✨");
        if !excluded_by_safety.is_empty() {
            println!(
                "  {} {} item(s) need review: rerun with {}",
                "💡",
                excluded_by_safety.len(),
                "--max-safety caution".cyan()
            );
        }
        return Ok(());
    }

    if matches!(cli.format, OutputFormat::Human) {
        output::print_scan_results(&results, false);
        if !excluded_by_safety.is_empty() {
            println!(
                "  {} Not included (review recommended): {}. Use {} to include them.",
                "ℹ️",
                excluded_by_safety.join(", "),
                "--max-safety caution".cyan()
            );
            println!();
        }
    }

    let mode = if dry_run {
        CleanMode::DryRun
    } else if hard {
        CleanMode::HardDelete
    } else {
        CleanMode::SoftDelete
    };

    let tool_items: Vec<String> = results
        .items
        .iter()
        .filter_map(|i| i.tool_action.as_ref().map(|t| format!("{} ({})", i.name, t.command_line())))
        .collect();

    if mode != CleanMode::DryRun && !yes {
        if !tool_items.is_empty() {
            println!("  {} These run a tool and can't be undone:", "🛠");
            for t in &tool_items {
                println!("    {} {}", "•".dimmed(), t);
            }
        }
        let question = if hard {
            format!(
                "PERMANENTLY DELETE {} ({})?",
                format::format_count(results.total_files),
                format::format_size(results.total_reclaimable)
            )
        } else {
            format!(
                "Move {} ({}) to the undo area?",
                format::format_count(results.total_files),
                format::format_size(results.total_reclaimable)
            )
        };
        if !confirm(&question)? {
            println!("  {} Cancelled", "✗".red());
            return Ok(());
        }
    }

    let report = cleaner::clean_with(
        &results.items,
        mode,
        profile_name,
        &cleaner::CleanOptions {
            show_progress,
            force,
            exclude_paths: Vec::new(),
        },
    )?;

    match cli.format {
        OutputFormat::Human => output::print_clean_report(&report),
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&output::clean_report_value(&report))?);
        }
        OutputFormat::Quiet => {
            println!(
                "{}  {}  {}",
                format::format_size(report.bytes_freed + report.bytes_staged),
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
        PrivacyAction::Scan {
            browsers,
            cookies,
            all,
        } => {
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

        PrivacyAction::Clean {
            yes,
            dry_run,
            cookies,
            app_cookies,
            hard,
        } => {
            let report = tidymac::privacy::run_privacy_audit(true, true);
            output::print_privacy_report(&report);

            let mode = if *dry_run {
                CleanMode::DryRun
            } else if *hard {
                CleanMode::HardDelete
            } else {
                CleanMode::SoftDelete
            };
            let opts = tidymac::privacy::PrivacyCleanOptions {
                mode: mode.clone(),
                caches: true,
                browser_cookies: *cookies,
                app_cookies: *app_cookies,
            };

            if mode != CleanMode::DryRun && !yes {
                let mut what = vec!["browser caches"];
                if *cookies {
                    what.push("browser cookies (signs you out of websites)");
                }
                if *app_cookies {
                    what.push("app cookies (signs you out of apps)");
                }
                if !confirm(&format!("Clean {}? Browsing history is not touched.", what.join(", ")))? {
                    println!("  {} Cancelled", "✗".red());
                    return Ok(());
                }
            }

            let result = tidymac::privacy::clean_privacy(&report, &opts)?;
            output::print_clean_report(&result);
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
            OutputFormat::Json => match serde_json::to_string_pretty(&usage) {
                Ok(s) => println!("{}", s),
                Err(e) => eprintln!("Error: {}", e),
            },
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
    use tidymac::cli::args::{AppSort, AppsAction};

    match action {
        AppsAction::List {
            sort,
            detailed,
            unused_days,
        } => {
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
                apps.retain(|a| a.last_opened.map(|t| t < threshold).unwrap_or(true));
            }

            // Sort
            match sort {
                AppSort::Name => {
                    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                }
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
            match tidymac::apps::resolve_app(&apps, name) {
                tidymac::apps::AppLookup::Found(app) => {
                    output::print_app_info(app);
                    output::print_install_info(&tidymac::apps::install_info(&app.path));
                }
                tidymac::apps::AppLookup::Ambiguous(list) | tidymac::apps::AppLookup::NotFound(list) => {
                    print_app_suggestions(name, &list);
                }
            }
            Ok(())
        }

        AppsAction::Remove {
            ref name,
            dry_run,
            yes,
            hard,
            include_name_matches,
            keep_bundle,
        } => {
            let apps = tidymac::apps::discover_apps();
            let app = match tidymac::apps::resolve_app(&apps, name) {
                tidymac::apps::AppLookup::Found(a) => a,
                tidymac::apps::AppLookup::Ambiguous(list) | tidymac::apps::AppLookup::NotFound(list) => {
                    print_app_suggestions(name, &list);
                    return Ok(());
                }
            };
            output::print_app_info(app);
            let info = tidymac::apps::install_info(&app.path);
            output::print_install_info(&info);

            let mode = if *dry_run {
                CleanMode::DryRun
            } else if *hard {
                CleanMode::HardDelete
            } else {
                CleanMode::SoftDelete
            };
            if mode != CleanMode::DryRun && !yes {
                let what = if *keep_bundle { "the data of" } else { "" };
                let how = if *hard { "permanently delete" } else { "move to the undo area" };
                if !confirm(&format!(
                    "Remove {} '{}' ({}) — {}?",
                    what,
                    app.name,
                    format::format_size(app.total_size),
                    how
                ))? {
                    println!("  {} Cancelled", "✗".red());
                    return Ok(());
                }
            }

            let opts = tidymac::apps::UninstallOptions {
                mode,
                remove_bundle: !keep_bundle,
                regenerable_only: false,
                include_name_matches: *include_name_matches,
                extra: info.brew_zap.clone(),
                only_paths: None,
            };
            let report = tidymac::apps::uninstall_app_with(app, &opts)?;
            output::print_uninstall_report(&report);
            Ok(())
        }

        AppsAction::Orphans { ref clean, hard, yes } => {
            let groups = tidymac::apps::find_orphans();
            if clean.is_empty() {
                match cli.format {
                    OutputFormat::Json => {
                        let json: Vec<_> = groups
                            .iter()
                            .map(|g| {
                                serde_json::json!({
                                    "bundle_id": g.bundle_id,
                                    "total_size": g.total_size,
                                    "files": g.files.iter().map(|f| f.path.display().to_string()).collect::<Vec<_>>(),
                                })
                            })
                            .collect();
                        println!("{}", serde_json::to_string_pretty(&json)?);
                    }
                    _ => output::print_orphans(&groups),
                }
                return Ok(());
            }

            let wanted: Vec<String> = clean.iter().map(|c| c.to_lowercase()).collect();
            let selected: Vec<_> = groups.iter().filter(|g| wanted.contains(&g.bundle_id)).cloned().collect();
            if selected.is_empty() {
                println!("  No orphaned leftovers for: {}", clean.join(", "));
                return Ok(());
            }
            output::print_orphans(&selected);
            if !yes && !confirm("Remove these leftovers?")? {
                println!("  {} Cancelled", "✗".red());
                return Ok(());
            }
            let library = dirs::home_dir().unwrap_or_default().join("Library");
            let requests = selected
                .iter()
                .flat_map(|g| {
                    let lib = library.clone();
                    g.files.iter().map(move |f| cleaner::DeleteRequest {
                        path: f.path.clone(),
                        size_bytes: f.size,
                        category: "Orphan".into(),
                        safety: "Caution".into(),
                        roots: vec![lib.clone()],
                        policy: tidymac::common::safety::DeletePolicy::AppData,
                    })
                })
                .collect();
            let mode = if *hard { CleanMode::HardDelete } else { CleanMode::SoftDelete };
            let report = cleaner::execute(requests, Vec::new(), &cleaner::DeleteOptions::new(mode, "orphans"))?;
            output::print_clean_report(&report);
            Ok(())
        }
    }
}

fn print_app_suggestions(query: &str, candidates: &[&tidymac::apps::InstalledApp]) {
    if candidates.is_empty() {
        println!("  No app found matching '{}'", query);
        return;
    }
    println!(
        "  '{}' doesn't identify a single app. Use the exact name or bundle id:",
        query
    );
    for a in candidates {
        println!(
            "    {} {}  {}",
            "•".dimmed(),
            a.name,
            a.bundle_id.clone().unwrap_or_default().dimmed()
        );
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
                    let json: Vec<_> = items
                        .iter()
                        .map(|i| {
                            serde_json::json!({
                                "label": i.label,
                                "name": i.name,
                                "kind": format!("{}", i.kind),
                                "enabled": i.enabled,
                                "program": i.program,
                                "run_at_load": i.run_at_load,
                                "path": i.path.display().to_string(),
                            })
                        })
                        .collect();
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

            if matches.len() > 1 {
                println!("  Several startup items match '{}'. Use the exact label:", name);
                for m in &matches {
                    println!("    {} {}", "•".dimmed(), m.label);
                }
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

            if matches.len() > 1 {
                println!("  Several startup items match '{}'. Use the exact label:", name);
                for m in &matches {
                    println!("    {} {}", "•".dimmed(), m.label);
                }
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
                    let status = if s.restored {
                        "restored"
                    } else if s.is_expired {
                        "expired"
                    } else {
                        "active"
                    };
                    println!(
                        "{}  {}  {}  {}",
                        s.session_id,
                        s.profile,
                        format::format_size(s.staged_size),
                        status
                    );
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
        println!(
            "    {} Restore specific",
            "tidymac undo --session <ID>".cyan()
        );
        println!("    {} List all", "tidymac undo --list".cyan());
        println!();

        let sessions = CleanManifest::list_sessions()?;
        let active: Vec<_> = sessions
            .iter()
            .filter(|s| !s.restored && !s.is_expired)
            .collect();
        if !active.is_empty() {
            println!("  Active sessions:");
            for s in active.iter().take(5) {
                println!(
                    "    {} {} — {} ({} files)",
                    "•".dimmed(),
                    s.session_id,
                    format::format_size(s.staged_size),
                    s.total_files
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

    let restorable_count = manifest
        .items
        .iter()
        .filter(|i| i.success && i.staged_path.is_some())
        .count();

    println!();
    println!(
        "  {} Restoring session '{}' — {} files ({})",
        "↩️",
        session_id.cyan(),
        restorable_count,
        format::format_size(manifest.total_bytes)
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
            println!(
                "{}  {}  {}",
                report.session_id,
                report.restored_count,
                format::format_size(report.restored_bytes)
            );
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

        println!(
            "  {} Installed auto-purge plist: {}",
            "✓".green(),
            plist_path.display()
        );
        let domain = tidymac::common::sys::gui_domain();
        let _ = std::process::Command::new("/bin/launchctl")
            .args(["bootout", &format!("{}/com.tidymac.purge", domain)])
            .output();
        let loaded = std::process::Command::new("/bin/launchctl")
            .args(["bootstrap", &domain, &plist_path.display().to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if loaded {
            println!("  {} Loaded with launchd ({})", "✓".green(), domain);
        } else {
            println!(
                "  {} Load with: {}",
                "💡",
                format!("launchctl bootstrap {} {}", domain, plist_path.display()).cyan()
            );
        }
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
        println!(
            "  {} Purged '{}', freed {}",
            "🔥",
            sid,
            format::format_size(freed)
        );
        return Ok(());
    }

    // Purge all
    if all {
        let sessions = CleanManifest::list_sessions()?;
        let total: u64 = sessions.iter().map(|s| s.staged_size).sum();

        if !yes {
            print!(
                "  {} Permanently purge ALL {} sessions ({})? [y/N] ",
                "❓",
                sessions.len(),
                format::format_size(total)
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
    println!(
        "    {} Specific session",
        "tidymac purge --session <ID>".cyan()
    );
    println!(
        "    {} Auto-purge daily",
        "tidymac purge --install-auto".cyan()
    );
    println!();

    let health = cleaner::check_staging_health()?;
    println!(
        "  Staging: {} sessions, {} total",
        health.session_count,
        format::format_size(health.total_size)
    );
    if health.expired_count > 0 {
        println!(
            "  {} {} expired ({}) — run {}",
            "⚠".yellow(),
            health.expired_count,
            format::format_size(health.expired_size),
            "tidymac purge --expired".cyan()
        );
    }
    println!();

    Ok(())
}

// ─── Config ───────────────────────────────────────────────────────────────────

fn cmd_config(action: &ConfigAction) -> Result<()> {
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
                "exclude_paths" => {
                    config.exclude_paths = value.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
                }
                _ => anyhow::bail!("Unknown config key: {}", key),
            }
            config.save()?;
            println!("  {} Set {} = {}", "✓".green(), key, value);
            Ok(())
        }
        ConfigAction::ClearCache => {
            tidymac::scanner::cache::ScanCache::clear()?;
            println!(
                "  {} Scan cache cleared. Next scan will be fresh.",
                "✓".green()
            );
            Ok(())
        }
    }
}

// ─── Status ───────────────────────────────────────────────────────────────────

fn cmd_status(diagnostics: bool) -> Result<()> {
    let config = Config::load()?;

    if diagnostics {
        return generate_diagnostics();
    }

    println!();
    println!("  {} TidyMac Status", "📊");
    println!("{}", "─".repeat(60).dimmed());
    println!();

    println!("  {} Default profile: {}", "⚙️", config.default_profile);
    println!(
        "  {} Staging retention: {} days",
        "⚙️", config.staging_retention_days
    );
    println!(
        "  {} Large file threshold: {} MB",
        "⚙️", config.large_file_threshold_mb
    );

    // Staging health
    let health = cleaner::check_staging_health()?;
    println!();
    println!(
        "  {} Staging: {} sessions, {} used",
        "📦",
        health.session_count,
        format::format_size(health.total_size)
    );
    if health.expired_count > 0 {
        println!(
            "  {} {} expired ({}) — run {}",
            "⚠".yellow(),
            health.expired_count,
            format::format_size(health.expired_size),
            "tidymac purge --expired".cyan()
        );
    }
    output::print_staging_health(&health);

    // Scan cache info
    if let Some(cache) = tidymac::scanner::cache::ScanCache::load(&config.default_profile) {
        println!(
            "  {} Scan cache: {} entries, updated {}",
            "⚡",
            cache.entry_count(),
            cache.age_string()
        );
    } else {
        println!("  {} Scan cache: empty (first scan will populate)", "⚡");
    }

    // Recent sessions
    let sessions = CleanManifest::list_sessions()?;
    if !sessions.is_empty() {
        println!("  {} Recent sessions:", "📋");
        for s in sessions.iter().take(5) {
            let status = if s.restored {
                "restored".green().to_string()
            } else if s.is_expired {
                "expired".red().to_string()
            } else {
                "active".yellow().to_string()
            };
            println!(
                "    {} {} — {} ({} files) [{}]",
                "•".dimmed(),
                s.session_id,
                format::format_size(s.total_bytes),
                s.total_files,
                status
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

fn generate_diagnostics() -> Result<()> {
    use std::fs::File;
    use std::io::Write;

    let zip_path = std::env::current_dir()?.join("tidymac_diagnostics.zip");
    println!(
        "{} Generating diagnostics archive at: {}",
        "📦".cyan(),
        zip_path.display()
    );

    let file = File::create(&zip_path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    // 1. Add Config
    let config_path = tidymac::common::config::Config::config_path();
    if config_path.exists() {
        zip.start_file("config.toml", options)?;
        if let Ok(content) = std::fs::read(&config_path) {
            zip.write_all(&content)?;
        }
    }

    // 2. Add Logs
    let log_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("Library/Logs/TidyMac");

    if log_dir.exists() {
        for entry in std::fs::read_dir(log_dir)?.filter_map(|e| e.ok()) {
            if entry.file_type()?.is_file() {
                let name = entry.file_name().to_string_lossy().to_string();
                zip.start_file(format!("logs/{}", name), options)?;
                if let Ok(content) = std::fs::read(entry.path()) {
                    zip.write_all(&content)?;
                }
            }
        }
    }

    zip.finish()?;
    println!("{} Diagnostics generated successfully.", "✓".green());
    Ok(())
}

// ─── Map / System Data ────────────────────────────────────────────────────────

fn cmd_map(cli: &Cli, path: Option<String>, depth: usize, top: usize) -> Result<()> {
    let home = dirs::home_dir().unwrap_or_default();
    let root = match path {
        Some(p) if p == "~" => home,
        Some(p) => match p.strip_prefix("~/") {
            Some(rest) => home.join(rest),
            None => PathBuf::from(p),
        },
        None => home,
    };
    if !root.is_dir() {
        anyhow::bail!("Not a folder: {}", root.display());
    }
    if !cli.quiet && matches!(cli.format, OutputFormat::Human) {
        println!();
        println!("  {} Mapping {} ...", "🗺", format::format_path(&root).cyan());
        println!();
    }
    let tree = tidymac::viz::build_tree(
        &root,
        tidymac::viz::TreeOptions {
            max_depth: depth.clamp(1, 6),
            max_children: top.max(1),
        },
    );
    match cli.format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&tree)?),
        OutputFormat::Quiet => println!("{}  {}", format::format_size(tree.size), root.display()),
        OutputFormat::Human => {
            tidymac::viz::print_tree(&tree, 0, depth);
            println!();
        }
    }
    Ok(())
}

fn cmd_system_data(cli: &Cli) -> Result<()> {
    let report = tidymac::viz::explain_system_data();
    match cli.format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&report)?),
        _ => {
            println!();
            println!("  {} What's in System Data", "🧩");
            println!("{}", "─".repeat(70).dimmed());
            for e in &report.entries {
                let size = e
                    .size
                    .map(format::format_size)
                    .unwrap_or_else(|| "—".to_string());
                println!("  {:<32} {:>10}", e.name.bold(), size.cyan());
                println!("    {}", e.description.dimmed());
                println!("    {} {}", "→".dimmed(), e.advice);
            }
            println!();
            for n in &report.notes {
                println!("  {} {}", "ℹ️", n);
            }
            println!();
        }
    }
    Ok(())
}

// ─── Schedule / Report ────────────────────────────────────────────────────────

fn cmd_schedule(cli: &Cli, action: &tidymac::cli::args::ScheduleAction) -> Result<()> {
    use tidymac::cli::args::ScheduleAction;
    match action {
        ScheduleAction::Install { every } => {
            let profile = cli.profile.as_deref().unwrap_or("quick");
            let status = tidymac::schedule::install(profile, *every, None)?;
            println!(
                "  {} Scheduled a {} dry-run report for profile '{}'.",
                "✓".green(),
                status.interval,
                status.profile
            );
            println!("    Reports are saved in {}", format::format_path(&tidymac::schedule::reports_dir()));
            println!("    Nothing is ever deleted by scheduled reports.");
        }
        ScheduleAction::Remove => {
            if tidymac::schedule::remove()? {
                println!("  {} Scheduled report removed.", "✓".green());
            } else {
                println!("  No scheduled report was installed.");
            }
        }
        ScheduleAction::Status => {
            match tidymac::schedule::status() {
                Some(s) => println!("  {} {} report for profile '{}'", "⏰", s.interval, s.profile),
                None => println!("  No scheduled report. Install with {}", "tidymac schedule install --profile developer --every weekly".cyan()),
            }
            if let Some(r) = tidymac::schedule::latest_report() {
                println!(
                    "  Latest report ({}): {} reclaimable",
                    r.timestamp.format("%Y-%m-%d %H:%M"),
                    format::format_size(r.total_reclaimable)
                );
                for line in r.items.iter().take(8) {
                    println!("    {} {:<36} {:>10}", "•".dimmed(), line.name, format::format_size(line.size_bytes));
                }
            }
        }
    }
    Ok(())
}

fn cmd_report(cli: &Cli) -> Result<()> {
    let profile = cli.profile.as_deref().unwrap_or("quick");
    let (path, summary) = tidymac::schedule::write_report(profile)?;
    println!(
        "{} report: {} reclaimable — {}",
        summary.timestamp.to_rfc3339(),
        format::format_size(summary.total_reclaimable),
        path.display()
    );
    Ok(())
}
