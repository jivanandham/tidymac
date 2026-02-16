use std::path::PathBuf;

use super::perceptual::SimilarGroup;

/// Strategy for resolving which file to keep in a duplicate group
#[derive(Debug, Clone)]
pub enum ResolveStrategy {
    /// Keep the largest file (best quality for images)
    KeepLargest,
    /// Keep the newest file (most recently modified)
    KeepNewest,
    /// Keep the oldest file (original)
    KeepOldest,
    /// Keep file in a preferred directory
    KeepInDir(PathBuf),
    /// Interactive — let the user choose (for CLI)
    Interactive,
}

/// Result of resolving a duplicate group
#[derive(Debug, Clone)]
pub struct ResolvedGroup {
    /// The file to keep
    pub keep: PathBuf,
    /// Files to remove
    pub remove: Vec<PathBuf>,
    /// Total bytes that would be freed
    pub bytes_freed: u64,
    /// Why this file was kept
    pub reason: String,
}

/// Resolve a duplicate group using the given strategy
pub fn resolve_group(group: &SimilarGroup, strategy: &ResolveStrategy) -> ResolvedGroup {
    let members = &group.members;

    if members.is_empty() {
        return ResolvedGroup {
            keep: PathBuf::new(),
            remove: Vec::new(),
            bytes_freed: 0,
            reason: "Empty group".to_string(),
        };
    }

    let (keep_idx, reason) = match strategy {
        ResolveStrategy::KeepLargest => {
            let idx = members
                .iter()
                .enumerate()
                .max_by_key(|(_, m)| m.size_bytes)
                .map(|(i, _)| i)
                .unwrap_or(0);
            (idx, "Largest file".to_string())
        }

        ResolveStrategy::KeepNewest => {
            let idx = members
                .iter()
                .enumerate()
                .max_by_key(|(_, m)| {
                    std::fs::metadata(&m.path)
                        .and_then(|meta| meta.modified())
                        .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                })
                .map(|(i, _)| i)
                .unwrap_or(0);
            (idx, "Most recently modified".to_string())
        }

        ResolveStrategy::KeepOldest => {
            let idx = members
                .iter()
                .enumerate()
                .min_by_key(|(_, m)| {
                    std::fs::metadata(&m.path)
                        .and_then(|meta| meta.modified())
                        .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                })
                .map(|(i, _)| i)
                .unwrap_or(0);
            (idx, "Oldest file (original)".to_string())
        }

        ResolveStrategy::KeepInDir(preferred) => {
            let idx = members
                .iter()
                .enumerate()
                .find(|(_, m)| m.path.starts_with(preferred))
                .map(|(i, _)| i)
                .unwrap_or(0); // Fall back to first if none in preferred dir
            (idx, format!("In preferred directory: {}", preferred.display()))
        }

        ResolveStrategy::Interactive => {
            // Default to keeping the largest; actual interaction happens in CLI
            (0, "User selection (defaulting to first)".to_string())
        }
    };

    let keep = members[keep_idx].path.clone();
    let remove: Vec<PathBuf> = members
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != keep_idx)
        .map(|(_, m)| m.path.clone())
        .collect();
    let bytes_freed: u64 = members
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != keep_idx)
        .map(|(_, m)| m.size_bytes)
        .sum();

    ResolvedGroup {
        keep,
        remove,
        bytes_freed,
        reason,
    }
}

/// Resolve all groups in a result set
pub fn resolve_all(
    groups: &[SimilarGroup],
    strategy: &ResolveStrategy,
) -> Vec<ResolvedGroup> {
    groups.iter().map(|g| resolve_group(g, strategy)).collect()
}
