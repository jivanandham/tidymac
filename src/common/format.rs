use colored::*;

/// Format bytes into human-readable size string
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Format size with color based on magnitude
pub fn format_size_colored(bytes: u64) -> ColoredString {
    let s = format_size(bytes);
    const GB: u64 = 1024 * 1024 * 1024;
    const MB100: u64 = 100 * 1024 * 1024;

    if bytes >= GB {
        s.red().bold()
    } else if bytes >= MB100 {
        s.yellow()
    } else {
        s.white()
    }
}

/// Format file count with appropriate plural
pub fn format_count(count: usize) -> String {
    if count == 1 {
        "1 file".to_string()
    } else {
        format!("{} files", count)
    }
}

/// Format a path for display, replacing home directory with ~
pub fn format_path(path: &std::path::Path) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Ok(stripped) = path.strip_prefix(&home) {
            return format!("~/{}", stripped.display());
        }
    }
    path.display().to_string()
}

/// Format duration in human-readable form
pub fn format_duration(secs: f64) -> String {
    if secs < 1.0 {
        format!("{:.0}ms", secs * 1000.0)
    } else if secs < 60.0 {
        format!("{:.1}s", secs)
    } else {
        let mins = (secs / 60.0).floor() as u64;
        let remaining = secs - (mins as f64 * 60.0);
        format!("{}m {:.0}s", mins, remaining)
    }
}

/// Create a simple progress bar string
pub fn progress_bar(fraction: f64, width: usize) -> String {
    let filled = (fraction * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!(
        "{}{}",
        "━".repeat(filled).green(),
        "━".repeat(empty).truecolor(80, 80, 80)
    )
}

/// Colorize safety level
pub fn format_safety(safety: &crate::scanner::targets::SafetyLevel) -> ColoredString {
    match safety {
        crate::scanner::targets::SafetyLevel::Safe => "Safe".green(),
        crate::scanner::targets::SafetyLevel::Caution => "Caution".yellow(),
        crate::scanner::targets::SafetyLevel::Dangerous => "Dangerous".red().bold(),
    }
}

/// Print a section header
pub fn print_header(title: &str) {
    println!();
    println!("{}", title.bold().underline());
    println!();
}

/// Print a key-value pair
pub fn print_kv(key: &str, value: &str) {
    println!("  {}: {}", key.dimmed(), value);
}

/// Truncate a string to max length with ellipsis
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len <= 3 {
        ".".repeat(max_len)
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(1048576), "1.00 MB");
        assert_eq!(format_size(1073741824), "1.00 GB");
        assert_eq!(format_size(1099511627776), "1.00 TB");
    }

    #[test]
    fn test_format_count() {
        assert_eq!(format_count(0), "0 files");
        assert_eq!(format_count(1), "1 file");
        assert_eq!(format_count(42), "42 files");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(0.5), "500ms");
        assert_eq!(format_duration(3.7), "3.7s");
        assert_eq!(format_duration(125.0), "2m 5s");
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 8), "hello...");
    }
}
