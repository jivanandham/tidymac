use anyhow::Result;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use rayon::prelude::*;
use std::sync::{Arc, Mutex, Condvar};
use std::sync::atomic::Ordering;

use crate::ffi::CANCEL_FLAG;

/// Size of the quick hash prefix (first 4KB)
const QUICK_HASH_SIZE: usize = 4096;

/// Maximum number of concurrent full-file hashes to prevent OOM
const MAX_CONCURRENT_HASHES: usize = 8;

struct Semaphore {
    lock: Mutex<usize>,
    cvar: Condvar,
}

impl Semaphore {
    fn new(count: usize) -> Self {
        Self {
            lock: Mutex::new(count),
            cvar: Condvar::new(),
        }
    }

    fn acquire(&self) {
        let mut count = self.lock.lock().unwrap();
        while *count == 0 {
            count = self.cvar.wait(count).unwrap();
        }
        *count -= 1;
    }

    fn release(&self) {
        let mut count = self.lock.lock().unwrap();
        *count += 1;
        self.cvar.notify_one();
    }
}

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
pub fn group_by_size(files: &[PathBuf]) -> std::collections::HashMap<u64, Vec<PathBuf>> {
    let groups = Arc::new(Mutex::new(std::collections::HashMap::<u64, Vec<PathBuf>>::new()));

    files.par_iter().for_each(|path| {
        if CANCEL_FLAG.load(Ordering::Relaxed) {
            return;
        }
        if let Ok(meta) = std::fs::metadata(path) {
            if meta.is_file() {
                let mut map = groups.lock().unwrap();
                map.entry(meta.len()).or_default().push(path.clone());
            }
        }
    });

    let mut result = Arc::try_unwrap(groups).unwrap().into_inner().unwrap();
    // Only keep groups with 2+ files (potential duplicates)
    result.retain(|_, v| v.len() > 1);
    result
}

/// Group files by quick hash (first 4KB)
/// This is Pass 2: eliminates most remaining false positives cheaply.
pub fn group_by_quick_hash(files: &[PathBuf]) -> std::collections::HashMap<String, Vec<PathBuf>> {
    let groups = Arc::new(Mutex::new(std::collections::HashMap::<String, Vec<PathBuf>>::new()));

    files.par_iter().for_each(|path| {
        if CANCEL_FLAG.load(Ordering::Relaxed) {
            return;
        }
        match quick_hash(path) {
            Ok(hash) => {
                let mut map = groups.lock().unwrap();
                map.entry(hash).or_default().push(path.clone());
            }
            Err(_) => return, // Skip unreadable files
        }
    });

    let mut result = Arc::try_unwrap(groups).unwrap().into_inner().unwrap();
    result.retain(|_, v| v.len() > 1);
    result
}

/// Group files by full SHA-256 hash
/// This is Pass 3: confirms exact byte-for-byte duplicates.
pub fn group_by_full_hash(files: &[PathBuf]) -> std::collections::HashMap<String, Vec<PathBuf>> {
    let groups = Arc::new(Mutex::new(std::collections::HashMap::<String, Vec<PathBuf>>::new()));
    let sem = Arc::new(Semaphore::new(MAX_CONCURRENT_HASHES));

    files.par_iter().for_each(|path| {
        if CANCEL_FLAG.load(Ordering::Relaxed) {
            return;
        }
        
        sem.acquire();
        let hash_result = full_hash(path);
        sem.release();

        match hash_result {
            Ok(hash) => {
                let mut map = groups.lock().unwrap();
                map.entry(hash).or_default().push(path.clone());
            }
            Err(_) => return,
        }
    });

    let mut result = Arc::try_unwrap(groups).unwrap().into_inner().unwrap();
    result.retain(|_, v| v.len() > 1);
    result
}
