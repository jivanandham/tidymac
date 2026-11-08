//! Small, dependency-free wrappers around a few libc calls and macOS facts.

use std::path::PathBuf;

extern "C" {
    fn getuid() -> u32;
}

/// The real user id of the current process (replaces the old hardcoded `501`).
pub fn current_uid() -> u32 {
    // SAFETY: getuid() has no preconditions and cannot fail.
    unsafe { getuid() }
}

/// launchd domain for the logged-in GUI user, e.g. `gui/502`.
pub fn gui_domain() -> String {
    format!("gui/{}", current_uid())
}

/// The per-user temporary directory (`$TMPDIR`, which lives under
/// `/private/var/folders/xx/yyyy/T`). Returns the canonical path, or `None`
/// if it doesn't look like a per-user macOS temp dir.
pub fn user_temp_dir() -> Option<PathBuf> {
    let tmp = std::env::temp_dir();
    let canonical = std::fs::canonicalize(&tmp).ok()?;
    let s = canonical.to_string_lossy().to_lowercase();
    if s.starts_with("/private/var/folders/") && s.trim_end_matches('/').ends_with("/t") {
        Some(canonical)
    } else {
        None
    }
}

/// Locate a command-line tool. GUI apps launched from Finder get a minimal
/// `PATH` (`/usr/bin:/bin:/usr/sbin:/sbin`), so also look where Homebrew,
/// Docker Desktop and the Go installer put their binaries.
pub fn find_executable(name: &str) -> Option<PathBuf> {
    let mut search: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    for extra in [
        "/opt/homebrew/bin",
        "/usr/local/bin",
        "/usr/local/go/bin",
        "/Applications/Docker.app/Contents/Resources/bin",
        "/usr/bin",
    ] {
        search.push(PathBuf::from(extra));
    }
    if let Some(home) = dirs::home_dir() {
        search.push(home.join("go/bin"));
        search.push(home.join(".docker/bin"));
    }
    search.into_iter()
        .map(|d| d.join(name))
        .find(|p| p.is_file())
}

/// A `Command` for a tool found with [`find_executable`] (falls back to the bare name).
pub fn tool_command(name: &str) -> std::process::Command {
    match find_executable(name) {
        Some(p) => std::process::Command::new(p),
        None => std::process::Command::new(name),
    }
}
