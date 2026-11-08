use clap::{Parser, Subcommand, ValueEnum};

/// TidyMac — A developer-aware, privacy-first Mac cleanup utility
#[derive(Parser, Debug)]
#[command(
    name = "tidymac",
    version,
    about = "A developer-aware Mac cleanup utility",
    long_about = "TidyMac scans your Mac for junk files, developer caches, duplicates,\n\
                   and leftover app data. Clean safely with dry-run and undo support.",
    after_help = "EXAMPLES:\n  \
        tidymac scan                           Quick scan with default targets\n  \
        tidymac scan --profile developer       Developer-focused scan\n  \
        tidymac scan --profile deep --json     Deep scan with JSON output\n  \
        tidymac clean --profile developer      Clean with soft delete (default)\n  \
        tidymac clean --profile quick --hard   Permanent deletion\n  \
        tidymac dup ~/Pictures --perceptual    Find similar photos\n  \
        tidymac apps list --sort size          List apps by total size\n  \
        tidymac apps remove Slack --dry-run    Preview app removal\n  \
        tidymac startup list                   Show startup items\n  \
        tidymac privacy scan                   Privacy audit\n  \
        tidymac viz                            Storage visualization\n  \
        tidymac undo --last                    Restore last cleanup\n  \
        tidymac status                         Show cleanup history"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Smart profile to use
    #[arg(long, short, global = true, value_name = "NAME")]
    pub profile: Option<String>,

    /// Output format
    #[arg(long, global = true, default_value = "human")]
    pub format: OutputFormat,

    /// Disable colored output
    #[arg(long, global = true)]
    pub no_color: bool,

    /// Verbose output
    #[arg(long, short, global = true)]
    pub verbose: bool,

    /// Quiet mode — minimal output
    #[arg(long, short, global = true)]
    pub quiet: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Scan for cleanable files
    Scan {
        /// Show individual files in results
        #[arg(long)]
        detailed: bool,

        /// Only scan specific categories
        #[arg(long, value_delimiter = ',')]
        categories: Option<Vec<String>>,

        /// Simulate — show what would be found without full scan
        #[arg(long)]
        dry_run: bool,

        /// Skip cache and force a fresh scan
        #[arg(long)]
        no_cache: bool,
    },

    /// Remove selected files
    Clean {
        /// Use hard delete (permanent, no recovery)
        #[arg(long)]
        hard: bool,

        /// Skip confirmation prompt
        #[arg(long, short = 'y')]
        yes: bool,

        /// Only clean specific categories
        #[arg(long, value_delimiter = ',')]
        categories: Option<Vec<String>>,

        /// Highest safety level to include: safe (default), caution, dangerous
        #[arg(long, default_value = "safe")]
        max_safety: SafetyFilter,

        /// Simulate — show what would be cleaned
        #[arg(long)]
        dry_run: bool,

        /// Allow operations above the 100,000-item safety limit
        #[arg(long)]
        force: bool,
    },

    /// Find duplicate files
    Dup {
        /// Directory to scan for duplicates
        #[arg(default_value = "~")]
        path: String,

        /// Use perceptual hashing for image similarity
        #[arg(long)]
        perceptual: bool,

        /// Similarity threshold for perceptual matching (0.0-1.0)
        #[arg(long, default_value = "0.85")]
        threshold: f64,

        /// Minimum file size to consider (in bytes)
        #[arg(long, default_value = "1024")]
        min_size: u64,

        /// Show individual files in each group
        #[arg(long)]
        detailed: bool,

        /// Remove exact duplicates (keeps the largest/first copy; soft delete)
        #[arg(long)]
        remove: bool,

        /// Skip confirmation when removing
        #[arg(long, short = 'y')]
        yes: bool,
    },

    /// Disk map: biggest folders under a path
    Map {
        /// Folder to map (default: your home folder)
        path: Option<String>,

        /// Levels to show
        #[arg(long, default_value = "2")]
        depth: usize,

        /// Largest items to show per folder
        #[arg(long, default_value = "15")]
        top: usize,
    },

    /// Explain what's in macOS "System Data"
    SystemData,

    /// Scheduled dry-run reports (launchd)
    Schedule {
        #[command(subcommand)]
        action: ScheduleAction,
    },

    /// Run a dry-run scan and save it as a report (used by `schedule`)
    Report,

    /// List and uninstall applications
    Apps {
        #[command(subcommand)]
        action: AppsAction,
    },

    /// Manage startup items
    Startup {
        #[command(subcommand)]
        action: StartupAction,
    },

    /// Run privacy audit
    Privacy {
        #[command(subcommand)]
        action: PrivacyAction,
    },

    /// Storage visualization
    Viz {
        /// Interactive terminal UI mode
        #[arg(long)]
        interactive: bool,
    },

    /// Docker disk usage and cleanup
    Docker {
        /// Prune dangling images, stopped containers, and volumes
        #[arg(long)]
        prune: bool,

        /// Preview what would be pruned
        #[arg(long)]
        dry_run: bool,

        /// Skip confirmation
        #[arg(long, short = 'y')]
        yes: bool,
    },

    /// Restore soft-deleted files
    Undo {
        /// Restore the most recent cleanup session
        #[arg(long)]
        last: bool,

        /// Specific session ID to restore
        #[arg(long)]
        session: Option<String>,

        /// List all available sessions
        #[arg(long)]
        list: bool,
    },

    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Show cleanup history and staging status
    Status {
        /// Generate a diagnostics zip file for support
        #[arg(long)]
        diagnostics: bool,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: CompletionShell,
    },

    /// Purge expired or all staging sessions
    Purge {
        /// Only purge expired sessions
        #[arg(long)]
        expired: bool,

        /// Purge ALL staging sessions (frees maximum space)
        #[arg(long)]
        all: bool,

        /// Purge a specific session by ID
        #[arg(long)]
        session: Option<String>,

        /// Skip confirmation
        #[arg(long, short = 'y')]
        yes: bool,

        /// Install launchd plist for automatic daily purge
        #[arg(long)]
        install_auto: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum AppsAction {
    /// List installed applications
    List {
        /// Sort order
        #[arg(long, default_value = "size")]
        sort: AppSort,

        /// Show associated files for each app
        #[arg(long)]
        detailed: bool,

        /// Only show apps not opened in N days
        #[arg(long)]
        unused_days: Option<u32>,
    },

    /// Remove an application and its associated files (soft delete by default)
    Remove {
        /// Exact app name, bundle identifier, or path to the .app
        name: String,

        /// Preview what would be removed
        #[arg(long)]
        dry_run: bool,

        /// Skip confirmation
        #[arg(long, short = 'y')]
        yes: bool,

        /// Delete permanently instead of moving to the undo area
        #[arg(long)]
        hard: bool,

        /// Also remove folders matched only by the app's name
        #[arg(long)]
        include_name_matches: bool,

        /// Keep the .app, remove only its data
        #[arg(long)]
        keep_bundle: bool,
    },

    /// Show details about a specific app
    Info {
        /// Exact app name, bundle identifier, or path to the .app
        name: String,
    },

    /// Leftovers of apps that are no longer installed
    Orphans {
        /// Bundle identifiers whose leftovers to remove (soft delete)
        #[arg(long, value_delimiter = ',', num_args = 1..)]
        clean: Vec<String>,

        /// Delete permanently instead of moving to the undo area
        #[arg(long)]
        hard: bool,

        /// Skip confirmation
        #[arg(long, short = 'y')]
        yes: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum ScheduleAction {
    /// Install (or replace) a scheduled dry-run report for --profile
    Install {
        /// How often
        #[arg(long, value_enum, default_value = "weekly")]
        every: crate::schedule::Interval,
    },
    /// Remove the scheduled report
    Remove,
    /// Show the schedule and the latest report
    Status,
}

#[derive(Subcommand, Debug)]
pub enum StartupAction {
    /// List all startup items
    List,

    /// Disable a startup item
    Disable {
        /// Item identifier or name
        name: String,
    },

    /// Enable a previously disabled startup item
    Enable {
        /// Item identifier or name
        name: String,
    },

    /// Show details about a startup item
    Info {
        /// Item identifier or name
        name: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum PrivacyAction {
    /// Scan for privacy-sensitive data
    Scan {
        /// Include browser data
        #[arg(long)]
        browsers: bool,

        /// Include cookie analysis
        #[arg(long)]
        cookies: bool,

        /// Include all checks
        #[arg(long)]
        all: bool,
    },

    /// Clean browser caches (and optionally cookies). History is never deleted.
    Clean {
        /// Skip confirmation
        #[arg(long, short = 'y')]
        yes: bool,

        /// Dry run
        #[arg(long)]
        dry_run: bool,

        /// Also delete browser cookie databases (signs you out of websites)
        #[arg(long)]
        cookies: bool,

        /// Also delete per-app cookies and HTTP storage (signs you out of apps)
        #[arg(long)]
        app_cookies: bool,

        /// Delete permanently instead of moving to the undo area
        #[arg(long)]
        hard: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// Show current configuration
    Show,

    /// Reset to default configuration
    Reset,

    /// Set a configuration value
    Set {
        /// Configuration key
        key: String,
        /// Configuration value
        value: String,
    },

    /// Initialize TidyMac directories and default config
    Init,

    /// Clear the scan cache (forces fresh scan next time)
    ClearCache,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum OutputFormat {
    Human,
    Json,
    Quiet,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum SafetyFilter {
    Safe,
    Caution,
    Dangerous,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum AppSort {
    Name,
    Size,
    LastOpened,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum CompletionShell {
    Bash,
    Zsh,
    Fish,
}
