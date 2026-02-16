use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Check if Docker is installed
pub fn is_docker_installed() -> bool {
    std::process::Command::new("docker")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if Docker daemon is running
pub fn is_docker_running() -> bool {
    std::process::Command::new("docker")
        .args(["info", "--format", "{{.ServerVersion}}"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Docker disk usage breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerUsage {
    pub installed: bool,
    pub running: bool,
    pub images: DockerCategory,
    pub containers: DockerCategory,
    pub volumes: DockerCategory,
    pub build_cache: DockerCategory,
    pub total_size: u64,
    pub reclaimable: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerCategory {
    pub label: String,
    pub count: usize,
    pub size: u64,
    pub reclaimable: u64,
    pub details: Vec<DockerItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerItem {
    pub id: String,
    pub name: String,
    pub size: String,
    pub created: String,
    pub status: String,
}

impl DockerUsage {
    /// Create an empty usage report (Docker not available)
    pub fn unavailable() -> Self {
        Self {
            installed: false,
            running: false,
            images: DockerCategory::empty("Images"),
            containers: DockerCategory::empty("Containers"),
            volumes: DockerCategory::empty("Volumes"),
            build_cache: DockerCategory::empty("Build Cache"),
            total_size: 0,
            reclaimable: 0,
        }
    }
}

impl DockerCategory {
    pub fn empty(label: &str) -> Self {
        Self {
            label: label.to_string(),
            count: 0,
            size: 0,
            reclaimable: 0,
            details: Vec::new(),
        }
    }
}

/// Get full Docker disk usage breakdown
pub fn get_docker_usage() -> DockerUsage {
    if !is_docker_installed() {
        return DockerUsage::unavailable();
    }

    if !is_docker_running() {
        let mut usage = DockerUsage::unavailable();
        usage.installed = true;
        return usage;
    }

    let mut usage = DockerUsage {
        installed: true,
        running: true,
        images: get_image_info(),
        containers: get_container_info(),
        volumes: get_volume_info(),
        build_cache: get_build_cache_info(),
        total_size: 0,
        reclaimable: 0,
    };

    usage.total_size = usage.images.size
        + usage.containers.size
        + usage.volumes.size
        + usage.build_cache.size;
    usage.reclaimable = usage.images.reclaimable
        + usage.containers.reclaimable
        + usage.volumes.reclaimable
        + usage.build_cache.reclaimable;

    usage
}

/// Get Docker image information
fn get_image_info() -> DockerCategory {
    let mut cat = DockerCategory::empty("Images");

    // List all images
    let output = std::process::Command::new("docker")
        .args(["images", "--format", "{{.ID}}\t{{.Repository}}:{{.Tag}}\t{{.Size}}\t{{.CreatedSince}}"])
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 4 {
                    cat.details.push(DockerItem {
                        id: parts[0].to_string(),
                        name: parts[1].to_string(),
                        size: parts[2].to_string(),
                        created: parts[3].to_string(),
                        status: String::new(),
                    });
                }
            }
            cat.count = cat.details.len();
        }
    }

    // Get total image size
    cat.size = parse_docker_df_size("Images");

    // Dangling images are reclaimable
    let dangling_count = std::process::Command::new("docker")
        .args(["images", "-f", "dangling=true", "-q"])
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter(|l| !l.is_empty())
                .count()
        })
        .unwrap_or(0);

    // Estimate reclaimable as percentage of dangling
    if cat.count > 0 && dangling_count > 0 {
        cat.reclaimable = cat.size * dangling_count as u64 / cat.count as u64;
    }

    cat
}

/// Get Docker container information
fn get_container_info() -> DockerCategory {
    let mut cat = DockerCategory::empty("Containers");

    let output = std::process::Command::new("docker")
        .args(["ps", "-a", "--format", "{{.ID}}\t{{.Names}}\t{{.Size}}\t{{.CreatedAt}}\t{{.Status}}"])
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 5 {
                    cat.details.push(DockerItem {
                        id: parts[0].to_string(),
                        name: parts[1].to_string(),
                        size: parts[2].to_string(),
                        created: parts[3].to_string(),
                        status: parts[4].to_string(),
                    });
                }
            }
            cat.count = cat.details.len();
        }
    }

    cat.size = parse_docker_df_size("Containers");

    // Stopped containers are reclaimable
    let stopped = cat
        .details
        .iter()
        .filter(|c| c.status.contains("Exited"))
        .count();
    if cat.count > 0 && stopped > 0 {
        cat.reclaimable = cat.size * stopped as u64 / cat.count as u64;
    }

    cat
}

/// Get Docker volume information
fn get_volume_info() -> DockerCategory {
    let mut cat = DockerCategory::empty("Volumes");

    let output = std::process::Command::new("docker")
        .args(["volume", "ls", "--format", "{{.Name}}\t{{.Driver}}"])
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split('\t').collect();
                if !parts.is_empty() {
                    cat.details.push(DockerItem {
                        id: String::new(),
                        name: parts[0].to_string(),
                        size: String::new(),
                        created: String::new(),
                        status: parts.get(1).unwrap_or(&"").to_string(),
                    });
                }
            }
            cat.count = cat.details.len();
        }
    }

    cat.size = parse_docker_df_size("Local Volumes");
    // Dangling volumes are reclaimable
    let dangling = std::process::Command::new("docker")
        .args(["volume", "ls", "-f", "dangling=true", "-q"])
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter(|l| !l.is_empty())
                .count()
        })
        .unwrap_or(0);

    if cat.count > 0 && dangling > 0 {
        cat.reclaimable = cat.size * dangling as u64 / cat.count as u64;
    }

    cat
}

/// Get Docker build cache information
fn get_build_cache_info() -> DockerCategory {
    let mut cat = DockerCategory::empty("Build Cache");
    cat.size = parse_docker_df_size("Build Cache");
    cat.reclaimable = cat.size; // Build cache is always fully reclaimable
    cat
}

/// Parse size from `docker system df` output for a specific type
fn parse_docker_df_size(type_name: &str) -> u64 {
    let output = std::process::Command::new("docker")
        .args(["system", "df", "--format", "{{.Type}}\t{{.Size}}\t{{.Reclaimable}}"])
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                if line.starts_with(type_name) {
                    let parts: Vec<&str> = line.split('\t').collect();
                    if parts.len() >= 2 {
                        return parse_size_string(parts[1]);
                    }
                }
            }
        }
    }
    0
}

/// Parse Docker size strings like "2.5GB", "150MB", "1.2kB"
fn parse_size_string(s: &str) -> u64 {
    let s = s.trim();
    if s == "0B" || s.is_empty() {
        return 0;
    }

    // Split numeric part from unit
    let mut num_end = 0;
    for (i, c) in s.chars().enumerate() {
        if c.is_ascii_digit() || c == '.' {
            num_end = i + 1;
        } else {
            break;
        }
    }

    let num: f64 = s[..num_end].parse().unwrap_or(0.0);
    let unit = &s[num_end..].trim().to_uppercase();

    match unit.as_str() {
        "B" => num as u64,
        "KB" => (num * 1024.0) as u64,
        "MB" => (num * 1024.0 * 1024.0) as u64,
        "GB" => (num * 1024.0 * 1024.0 * 1024.0) as u64,
        "TB" => (num * 1024.0 * 1024.0 * 1024.0 * 1024.0) as u64,
        _ => num as u64,
    }
}

/// Result of a Docker prune operation
#[derive(Debug)]
pub struct DockerPruneReport {
    pub images_removed: usize,
    pub containers_removed: usize,
    pub volumes_removed: usize,
    pub build_cache_cleared: bool,
    pub space_freed: String,
    pub errors: Vec<String>,
}

/// Execute docker system prune (dangling only)
pub fn prune_dangling(dry_run: bool) -> Result<DockerPruneReport> {
    if !is_docker_running() {
        anyhow::bail!("Docker is not running");
    }

    if dry_run {
        // Just show what would be pruned
        let usage = get_docker_usage();
        return Ok(DockerPruneReport {
            images_removed: 0,
            containers_removed: 0,
            volumes_removed: 0,
            build_cache_cleared: false,
            space_freed: crate::common::format::format_size(usage.reclaimable),
            errors: Vec::new(),
        });
    }

    let mut report = DockerPruneReport {
        images_removed: 0,
        containers_removed: 0,
        volumes_removed: 0,
        build_cache_cleared: false,
        space_freed: String::new(),
        errors: Vec::new(),
    };

    // Prune stopped containers
    match run_docker_command(&["container", "prune", "-f"]) {
        Ok(output) => {
            report.containers_removed = output
                .lines()
                .filter(|l| l.len() == 64 || l.len() == 12) // container IDs
                .count();
        }
        Err(e) => report.errors.push(format!("Container prune: {}", e)),
    }

    // Prune dangling images
    match run_docker_command(&["image", "prune", "-f"]) {
        Ok(output) => {
            report.images_removed = output
                .lines()
                .filter(|l| l.starts_with("deleted:") || l.starts_with("untagged:"))
                .count();
        }
        Err(e) => report.errors.push(format!("Image prune: {}", e)),
    }

    // Prune dangling volumes
    match run_docker_command(&["volume", "prune", "-f"]) {
        Ok(output) => {
            report.volumes_removed = output
                .lines()
                .filter(|l| !l.is_empty() && !l.starts_with("Total") && !l.starts_with("Deleted"))
                .count();
        }
        Err(e) => report.errors.push(format!("Volume prune: {}", e)),
    }

    // Prune build cache
    match run_docker_command(&["builder", "prune", "-f"]) {
        Ok(_) => report.build_cache_cleared = true,
        Err(e) => report.errors.push(format!("Builder prune: {}", e)),
    }

    // Get total reclaimed from final output
    let _after = get_docker_usage();
    report.space_freed = "completed".to_string();

    Ok(report)
}

/// Run a docker command and return stdout
fn run_docker_command(args: &[&str]) -> Result<String> {
    let output = std::process::Command::new("docker")
        .args(args)
        .output()
        .context("Failed to run docker command")?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("docker {} failed: {}", args.join(" "), stderr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_size_string() {
        assert_eq!(parse_size_string("0B"), 0);
        assert_eq!(parse_size_string("100B"), 100);
        assert_eq!(parse_size_string("1KB"), 1024);
        assert_eq!(parse_size_string("1MB"), 1048576);
        assert_eq!(parse_size_string("1.5GB"), (1.5 * 1024.0 * 1024.0 * 1024.0) as u64);
        assert_eq!(parse_size_string("2.5MB"), (2.5 * 1024.0 * 1024.0) as u64);
        assert_eq!(parse_size_string(""), 0);
    }

    #[test]
    fn test_docker_unavailable() {
        let usage = DockerUsage::unavailable();
        assert!(!usage.installed);
        assert!(!usage.running);
        assert_eq!(usage.total_size, 0);
    }
}
