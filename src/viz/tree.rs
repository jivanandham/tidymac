//! Disk map: a size tree for treemap / sunburst views.

use rayon::prelude::*;
use serde::Serialize;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use crate::scanner::walker::{self, LinkTracker};

/// One node of the disk map.
#[derive(Debug, Clone, Serialize)]
pub struct TreeNode {
    pub name: String,
    pub path: PathBuf,
    /// Allocated bytes (hard links counted once).
    pub size: u64,
    /// Bytes deleting this would free (APFS clones counted by private bytes).
    pub reclaimable: u64,
    pub files: usize,
    pub is_dir: bool,
    /// True for the synthetic "N smaller items" node.
    pub is_other: bool,
    pub children: Vec<TreeNode>,
}

/// Options for [`build_tree`].
#[derive(Debug, Clone, Copy)]
pub struct TreeOptions {
    /// How many levels of children to return (1 = only the root's children).
    pub max_depth: usize,
    /// Keep this many largest children per folder; the rest become one "other" node.
    pub max_children: usize,
}

impl Default for TreeOptions {
    fn default() -> Self {
        Self {
            max_depth: 3,
            max_children: 24,
        }
    }
}

/// Build a size tree rooted at `root`. Never crosses onto another volume.
pub fn build_tree(root: &Path, opts: TreeOptions) -> TreeNode {
    let links = LinkTracker::default();
    let dev = std::fs::symlink_metadata(root).map(|m| m.dev()).unwrap_or(0);
    node_for(root, 0, &opts, &links, dev)
}

fn node_for(path: &Path, depth: usize, opts: &TreeOptions, links: &LinkTracker, dev: u64) -> TreeNode {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string());
    let md = std::fs::symlink_metadata(path).ok();
    let is_dir = md.as_ref().map(|m| m.is_dir()).unwrap_or(false);

    if !is_dir || depth >= opts.max_depth {
        let stats = walker::dir_stats_with(path, links, true);
        return TreeNode {
            name,
            path: path.to_path_buf(),
            size: stats.size,
            reclaimable: stats.reclaimable,
            files: stats.files,
            is_dir,
            is_other: false,
            children: Vec::new(),
        };
    }

    let children_paths: Vec<PathBuf> = match std::fs::read_dir(path) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                // stay on this volume; skip firmlink duplicate of the data volume
                let s = p.to_string_lossy();
                if s == "/System/Volumes" || s == "/Volumes" || s == "/dev" {
                    return false;
                }
                std::fs::symlink_metadata(p).map(|m| m.dev() == dev).unwrap_or(false)
            })
            .collect(),
        Err(_) => Vec::new(),
    };

    let mut children: Vec<TreeNode> = children_paths
        .par_iter()
        .map(|c| node_for(c, depth + 1, opts, links, dev))
        .filter(|n| n.size > 0)
        .collect();
    children.sort_by(|a, b| b.size.cmp(&a.size));

    if children.len() > opts.max_children {
        let rest: Vec<TreeNode> = children.split_off(opts.max_children);
        let other = TreeNode {
            name: format!("{} smaller items", rest.len()),
            path: path.to_path_buf(),
            size: rest.iter().map(|n| n.size).sum(),
            reclaimable: rest.iter().map(|n| n.reclaimable).sum(),
            files: rest.iter().map(|n| n.files).sum(),
            is_dir: false,
            is_other: true,
            children: Vec::new(),
        };
        children.push(other);
    }

    TreeNode {
        name,
        path: path.to_path_buf(),
        size: children.iter().map(|c| c.size).sum(),
        reclaimable: children.iter().map(|c| c.reclaimable).sum(),
        files: children.iter().map(|c| c.files).sum(),
        is_dir: true,
        is_other: false,
        children,
    }
}

/// Print a tree as indented text.
pub fn print_tree(node: &TreeNode, depth: usize, max_depth: usize) {
    use colored::Colorize;
    let indent = "  ".repeat(depth);
    let size = crate::common::format::format_size(node.size);
    let shared = if node.reclaimable + 1024 * 1024 < node.size {
        format!(
            "  ({} reclaimable — rest is shared with APFS clones)",
            crate::common::format::format_size(node.reclaimable)
        )
    } else {
        String::new()
    };
    let label = if node.is_dir && !node.is_other {
        format!("{}/", node.name).bold().to_string()
    } else {
        node.name.clone()
    };
    println!("  {}{:>10}  {}{}", indent, size.cyan(), label, shared.dimmed());
    if depth < max_depth {
        for c in &node.children {
            print_tree(c, depth + 1, max_depth);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_sums_children_and_groups_small_items() {
        let dir = tempfile::TempDir::new().unwrap();
        for i in 0..5 {
            std::fs::write(dir.path().join(format!("f{}", i)), vec![0u8; 4096 * (i + 1)]).unwrap();
        }
        let tree = build_tree(
            dir.path(),
            TreeOptions {
                max_depth: 2,
                max_children: 3,
            },
        );
        assert_eq!(tree.children.len(), 4); // 3 largest + "other"
        assert!(tree.children.last().unwrap().is_other);
        let sum: u64 = tree.children.iter().map(|c| c.size).sum();
        assert_eq!(sum, tree.size);
    }
}
