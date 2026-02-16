use anyhow::Result;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

/// Size of the quick hash prefix (first 4KB)
const QUICK_HASH_SIZE: usize = 4096;

/// Compute SHA-256 of the first N bytes of a file (quick hash)
pub fn quick_hash(path: &Path) -> Result<String> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut buffer = vec![0u8; QUICK_HASH_SIZE];
    let bytes_read = reader.read(&mut buffer)?;
    buffer.truncate(bytes_read);

    let mut hasher = Sha256::new();
    hasher.update(&buffer);
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

/// Compute full SHA-256 hash of a file
pub fn full_hash(path: &Path) -> Result<String> {
    let file = File::open(path)?;
    let mut reader = BufReader::with_capacity(1024 * 1024, file); // 1MB buffer
    let mut hasher = Sha256::new();

    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

/// Group file paths by their file size
/// This is Pass 1: instantly eliminates ~95% of files since
/// files with unique sizes cannot be duplicates.
pub fn group_by_size(files: &[PathBuf]) -> HashMap<u64, Vec<PathBuf>> {
    let mut groups: HashMap<u64, Vec<PathBuf>> = HashMap::new();

    for path in files {
        if let Ok(meta) = std::fs::metadata(path) {
            if meta.is_file() {
                groups.entry(meta.len()).or_default().push(path.clone());
            }
        }
    }

    // Only keep groups with 2+ files (potential duplicates)
    groups.retain(|_, v| v.len() > 1);
    groups
}

/// Group files by quick hash (first 4KB)
/// This is Pass 2: eliminates most remaining false positives cheaply.
pub fn group_by_quick_hash(files: &[PathBuf]) -> HashMap<String, Vec<PathBuf>> {
    let mut groups: HashMap<String, Vec<PathBuf>> = HashMap::new();

    for path in files {
        match quick_hash(path) {
            Ok(hash) => {
                groups.entry(hash).or_default().push(path.clone());
            }
            Err(_) => continue, // Skip unreadable files
        }
    }

    groups.retain(|_, v| v.len() > 1);
    groups
}

/// Group files by full SHA-256 hash
/// This is Pass 3: confirms exact byte-for-byte duplicates.
pub fn group_by_full_hash(files: &[PathBuf]) -> HashMap<String, Vec<PathBuf>> {
    let mut groups: HashMap<String, Vec<PathBuf>> = HashMap::new();

    for path in files {
        match full_hash(path) {
            Ok(hash) => {
                groups.entry(hash).or_default().push(path.clone());
            }
            Err(_) => continue,
        }
    }

    groups.retain(|_, v| v.len() > 1);
    groups
}
