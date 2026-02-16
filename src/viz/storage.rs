use colored::*;
use std::path::{Path, PathBuf};

use crate::common::format;
use crate::scanner::walker;

/// Category of disk usage
#[derive(Debug, Clone)]
pub struct UsageCategory {
    pub name: String,
    pub icon: &'static str,
    pub path: PathBuf,
    pub size: u64,
    pub color: CategoryColor,
}

#[derive(Debug, Clone)]
pub enum CategoryColor {
    Blue,
    Green,
    Yellow,
    Red,
    Cyan,
    Magenta,
    White,
}

/// Complete disk usage breakdown
#[derive(Debug, Clone)]
pub struct DiskUsage {
    pub total_capacity: u64,
    pub used: u64,
    pub available: u64,
    pub categories: Vec<UsageCategory>,
    pub mount_point: String,
}

/// Analyze disk usage and categorize by directory
pub fn analyze_disk_usage() -> DiskUsage {
    let home = dirs::home_dir().unwrap_or_default();

    // Get disk capacity info using `df`
    let (total, available) = get_disk_info("/");
    let used = total.saturating_sub(available);

    // Scan major categories
    let mut categories = Vec::new();

    let scan_targets: Vec<(&str, &str, PathBuf, CategoryColor)> = vec![
        ("Applications", "📱", PathBuf::from("/Applications"), CategoryColor::Blue),
        ("Documents", "📄", home.join("Documents"), CategoryColor::Green),
        ("Desktop", "🖥️", home.join("Desktop"), CategoryColor::Cyan),
        ("Downloads", "📥", home.join("Downloads"), CategoryColor::Yellow),
        ("Pictures", "🖼️", home.join("Pictures"), CategoryColor::Magenta),
        ("Music", "🎵", home.join("Music"), CategoryColor::Cyan),
        ("Movies", "🎬", home.join("Movies"), CategoryColor::Red),
        ("Developer", "🔧", home.join("Developer"), CategoryColor::Green),
        ("Library Caches", "📁", home.join("Library/Caches"), CategoryColor::Yellow),
        ("Library App Support", "📁", home.join("Library/Application Support"), CategoryColor::White),
    ];

    // Also check common dev directories
    let dev_dirs = ["Projects", "projects", "Code", "code", "repos", "src", "workspace", "dev"];
    let mut found_dev = false;

    for dir_name in &dev_dirs {
        let dir = home.join(dir_name);
        if dir.exists() && !found_dev {
            categories.push(UsageCategory {
                name: format!("~/{}", dir_name),
                icon: "💻",
                path: dir.clone(),
                size: quick_dir_size(&dir),
                color: CategoryColor::Green,
            });
            found_dev = true;
        }
    }

    for (name, icon, path, color) in scan_targets {
        if path.exists() {
            let size = quick_dir_size(&path);
            if size > 0 {
                categories.push(UsageCategory {
                    name: name.to_string(),
                    icon,
                    path,
                    size,
                    color,
                });
            }
        }
    }

    // Sort by size descending
    categories.sort_by(|a, b| b.size.cmp(&a.size));

    DiskUsage {
        total_capacity: total,
        used,
        available,
        categories,
        mount_point: "/".to_string(),
    }
}

/// Quick directory size (only top-level, not deeply recursive for speed)
fn quick_dir_size(path: &Path) -> u64 {
    walker::dir_size(path)
}

/// Get disk total and available space using statvfs-style approach
fn get_disk_info(mount: &str) -> (u64, u64) {
    let output = std::process::Command::new("df")
        .args(["-k", mount])
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            // Parse second line: filesystem 1K-blocks used available capacity ...
            if let Some(line) = stdout.lines().nth(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    let total = parts[1].parse::<u64>().unwrap_or(0) * 1024;
                    let available = parts[3].parse::<u64>().unwrap_or(0) * 1024;
                    return (total, available);
                }
            }
            (0, 0)
        }
        Err(_) => (0, 0),
    }
}

/// Print the storage visualization
pub fn print_viz(usage: &DiskUsage) {
    let bar_width: usize = 40;

    println!();
    println!("  {} Storage Overview", "💾");
    println!("{}", "─".repeat(65).dimmed());
    println!();

    // Overall disk usage bar
    let used_frac = if usage.total_capacity > 0 {
        usage.used as f64 / usage.total_capacity as f64
    } else {
        0.0
    };

    let filled = (used_frac * bar_width as f64).round() as usize;
    let empty = bar_width.saturating_sub(filled);

    let bar_color = if used_frac > 0.9 {
        "━".repeat(filled).red()
    } else if used_frac > 0.75 {
        "━".repeat(filled).yellow()
    } else {
        "━".repeat(filled).green()
    };

    println!(
        "  {} / {}  ({:.1}% used)",
        format::format_size(usage.used),
        format::format_size(usage.total_capacity),
        used_frac * 100.0,
    );
    println!(
        "  {}{}  {} available",
        bar_color,
        "━".repeat(empty).dimmed(),
        format::format_size(usage.available).cyan(),
    );
    println!();

    // Category breakdown
    println!("  {} Category Breakdown", "📊");
    println!("{}", "─".repeat(65).dimmed());
    println!();

    let max_size = usage.categories.first().map(|c| c.size).unwrap_or(1);

    for cat in &usage.categories {
        let frac = cat.size as f64 / max_size as f64;
        let bar_len = (frac * 25.0).round().max(1.0) as usize;
        let pct_of_disk = if usage.total_capacity > 0 {
            (cat.size as f64 / usage.total_capacity as f64) * 100.0
        } else {
            0.0
        };

        let bar = match cat.color {
            CategoryColor::Blue => "█".repeat(bar_len).blue(),
            CategoryColor::Green => "█".repeat(bar_len).green(),
            CategoryColor::Yellow => "█".repeat(bar_len).yellow(),
            CategoryColor::Red => "█".repeat(bar_len).red(),
            CategoryColor::Cyan => "█".repeat(bar_len).cyan(),
            CategoryColor::Magenta => "█".repeat(bar_len).magenta(),
            CategoryColor::White => "█".repeat(bar_len).white(),
        };

        println!(
            "  {} {:<25} {} {:>10}  ({:.1}%)",
            cat.icon,
            format::truncate(&cat.name, 25),
            format!("{:<25}", bar),
            format::format_size(cat.size),
            pct_of_disk,
        );
    }

    // "Other" category
    let categorized: u64 = usage.categories.iter().map(|c| c.size).sum();
    let other = usage.used.saturating_sub(categorized);
    if other > 0 {
        let pct = if usage.total_capacity > 0 {
            (other as f64 / usage.total_capacity as f64) * 100.0
        } else {
            0.0
        };
        let frac = other as f64 / max_size as f64;
        let bar_len = (frac * 25.0).round().max(1.0) as usize;
        println!(
            "  {} {:<25} {} {:>10}  ({:.1}%)",
            "📦",
            "Other / System",
            format!("{:<25}", "░".repeat(bar_len).dimmed()),
            format::format_size(other),
            pct,
        );
    }

    println!();
}

/// Print visualization as JSON
pub fn print_viz_json(usage: &DiskUsage) {
    let json = serde_json::json!({
        "mount_point": usage.mount_point,
        "total_capacity": usage.total_capacity,
        "used": usage.used,
        "available": usage.available,
        "used_percentage": if usage.total_capacity > 0 {
            (usage.used as f64 / usage.total_capacity as f64) * 100.0
        } else { 0.0 },
        "categories": usage.categories.iter().map(|c| {
            serde_json::json!({
                "name": c.name,
                "path": c.path.display().to_string(),
                "size": c.size,
                "percentage": if usage.total_capacity > 0 {
                    (c.size as f64 / usage.total_capacity as f64) * 100.0
                } else { 0.0 },
            })
        }).collect::<Vec<_>>(),
    });
    match serde_json::to_string_pretty(&json) {
        Ok(s) => println!("{}", s),
        Err(e) => eprintln!("Error: {}", e),
    }
}
