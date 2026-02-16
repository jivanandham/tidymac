use anyhow::Result;
use std::path::{Path, PathBuf};

/// Known image extensions
const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "bmp", "tiff", "tif", "webp", "heic", "heif",
];

/// Check if a file is an image based on extension
pub fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Filter a list of paths to only images
pub fn filter_images(paths: &[PathBuf]) -> Vec<PathBuf> {
    paths.iter().filter(|p| is_image(p)).cloned().collect()
}

/// A perceptual hash result for an image
#[derive(Debug, Clone)]
pub struct PerceptualHash {
    pub path: PathBuf,
    pub hash: image_hasher::ImageHash,
    pub size_bytes: u64,
}

/// Compute perceptual hash for a single image using aHash algorithm
pub fn compute_perceptual_hash(path: &Path) -> Result<image_hasher::ImageHash> {
    let img = image::open(path)?;

    let hasher = image_hasher::HasherConfig::new()
        .hash_size(16, 16) // 16x16 = 256-bit hash for good accuracy
        .hash_alg(image_hasher::HashAlg::DoubleGradient) // dHash — good balance of speed and accuracy
        .to_hasher();

    Ok(hasher.hash_image(&img))
}

/// Compute perceptual hashes for multiple images, skipping failures
pub fn compute_hashes(paths: &[PathBuf]) -> Vec<PerceptualHash> {
    let mut results = Vec::new();

    for path in paths {
        match compute_perceptual_hash(path) {
            Ok(hash) => {
                let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
                results.push(PerceptualHash {
                    path: path.clone(),
                    hash,
                    size_bytes: size,
                });
            }
            Err(_) => continue, // Skip undecodable images
        }
    }

    results
}

/// Find perceptually similar image groups based on hamming distance
///
/// `threshold` is a similarity score from 0.0 to 1.0
/// - 1.0 = identical
/// - 0.95 = very similar (minor edits)
/// - 0.85 = similar (different compression, slight crop)
/// - 0.70 = loosely similar
pub fn find_similar_groups(
    hashes: &[PerceptualHash],
    threshold: f64,
) -> Vec<SimilarGroup> {
    let hash_bits = 256; // 16x16 hash
    let max_distance = ((1.0 - threshold) * hash_bits as f64).round() as u32;

    // Track which images have been assigned to a group
    let mut assigned = vec![false; hashes.len()];
    let mut groups = Vec::new();

    for i in 0..hashes.len() {
        if assigned[i] {
            continue;
        }

        let mut group_members = vec![SimilarFile {
            path: hashes[i].path.clone(),
            size_bytes: hashes[i].size_bytes,
            similarity: 1.0, // Self-similarity
        }];

        for j in (i + 1)..hashes.len() {
            if assigned[j] {
                continue;
            }

            let distance = hashes[i].hash.dist(&hashes[j].hash);

            if distance <= max_distance {
                let similarity = 1.0 - (distance as f64 / hash_bits as f64);
                group_members.push(SimilarFile {
                    path: hashes[j].path.clone(),
                    size_bytes: hashes[j].size_bytes,
                    similarity,
                });
                assigned[j] = true;
            }
        }

        // Only create a group if there are 2+ members
        if group_members.len() > 1 {
            assigned[i] = true;

            // Sort by size descending (largest first = suggested keeper)
            group_members.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

            let wasted: u64 = group_members.iter().skip(1).map(|f| f.size_bytes).sum();

            groups.push(SimilarGroup {
                members: group_members,
                wasted_bytes: wasted,
                match_type: MatchType::PerceptuallySimilar,
            });
        }
    }

    // Sort groups by wasted space descending
    groups.sort_by(|a, b| b.wasted_bytes.cmp(&a.wasted_bytes));
    groups
}

/// A group of similar or duplicate files
#[derive(Debug, Clone)]
pub struct SimilarGroup {
    pub members: Vec<SimilarFile>,
    pub wasted_bytes: u64,
    pub match_type: MatchType,
}

/// A file within a similarity group
#[derive(Debug, Clone)]
pub struct SimilarFile {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub similarity: f64, // 0.0 to 1.0
}

/// Type of match between files
#[derive(Debug, Clone, PartialEq)]
pub enum MatchType {
    /// Byte-identical files
    Exact,
    /// Visually similar images
    PerceptuallySimilar,
}

impl std::fmt::Display for MatchType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MatchType::Exact => write!(f, "Exact"),
            MatchType::PerceptuallySimilar => write!(f, "Similar"),
        }
    }
}
