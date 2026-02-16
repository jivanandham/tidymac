use tempfile::TempDir;

use tidymac::duplicates::hasher;

#[test]
fn test_quick_hash_identical_files() {
    let dir = TempDir::new().unwrap();
    let content = b"Hello, TidyMac! This is test content for hashing.";

    let file1 = dir.path().join("file1.txt");
    let file2 = dir.path().join("file2.txt");
    std::fs::write(&file1, content).unwrap();
    std::fs::write(&file2, content).unwrap();

    let hash1 = hasher::quick_hash(&file1).unwrap();
    let hash2 = hasher::quick_hash(&file2).unwrap();

    assert_eq!(hash1, hash2, "Identical files should produce identical quick hashes");
}

#[test]
fn test_quick_hash_different_files() {
    let dir = TempDir::new().unwrap();

    let file1 = dir.path().join("file1.txt");
    let file2 = dir.path().join("file2.txt");
    std::fs::write(&file1, b"Content A").unwrap();
    std::fs::write(&file2, b"Content B").unwrap();

    let hash1 = hasher::quick_hash(&file1).unwrap();
    let hash2 = hasher::quick_hash(&file2).unwrap();

    assert_ne!(hash1, hash2, "Different files should produce different hashes");
}

#[test]
fn test_full_hash_identical_files() {
    let dir = TempDir::new().unwrap();
    // Create content larger than quick hash size (4KB)
    let content: Vec<u8> = (0..8192).map(|i| (i % 256) as u8).collect();

    let file1 = dir.path().join("file1.bin");
    let file2 = dir.path().join("file2.bin");
    std::fs::write(&file1, &content).unwrap();
    std::fs::write(&file2, &content).unwrap();

    let hash1 = hasher::full_hash(&file1).unwrap();
    let hash2 = hasher::full_hash(&file2).unwrap();

    assert_eq!(hash1, hash2);
}

#[test]
fn test_full_hash_differs_from_quick_when_content_differs_after_4kb() {
    let dir = TempDir::new().unwrap();

    // Same first 4KB, different after
    let mut content1 = vec![0u8; 8192];
    let mut content2 = vec![0u8; 8192];
    content1[5000] = 0xFF; // Differs after 4KB
    content2[5000] = 0x00;

    let file1 = dir.path().join("file1.bin");
    let file2 = dir.path().join("file2.bin");
    std::fs::write(&file1, &content1).unwrap();
    std::fs::write(&file2, &content2).unwrap();

    // Quick hashes should be the same (only first 4KB)
    let qh1 = hasher::quick_hash(&file1).unwrap();
    let qh2 = hasher::quick_hash(&file2).unwrap();
    assert_eq!(qh1, qh2, "Quick hashes should match (same first 4KB)");

    // Full hashes should differ
    let fh1 = hasher::full_hash(&file1).unwrap();
    let fh2 = hasher::full_hash(&file2).unwrap();
    assert_ne!(fh1, fh2, "Full hashes should differ (content differs after 4KB)");
}

#[test]
fn test_hash_nonexistent_file() {
    let result = hasher::quick_hash(std::path::Path::new("/nonexistent/file.txt"));
    assert!(result.is_err());
}

#[test]
fn test_group_by_size() {
    let dir = TempDir::new().unwrap();

    // Create files of different sizes
    std::fs::write(dir.path().join("a.txt"), "hello").unwrap(); // 5 bytes
    std::fs::write(dir.path().join("b.txt"), "hello").unwrap(); // 5 bytes (duplicate size)
    std::fs::write(dir.path().join("c.txt"), "hi").unwrap(); // 2 bytes (unique size)

    let files = vec![
        dir.path().join("a.txt"),
        dir.path().join("b.txt"),
        dir.path().join("c.txt"),
    ];

    let groups = hasher::group_by_size(&files);

    // Only the 5-byte group should remain (2+ files)
    assert_eq!(groups.len(), 1, "Should have 1 size group");
    let group = groups.values().next().unwrap();
    assert_eq!(group.len(), 2, "Size group should have 2 files");
}

#[test]
fn test_group_by_size_no_duplicates() {
    let dir = TempDir::new().unwrap();

    std::fs::write(dir.path().join("a.txt"), "a").unwrap();
    std::fs::write(dir.path().join("b.txt"), "bb").unwrap();
    std::fs::write(dir.path().join("c.txt"), "ccc").unwrap();

    let files = vec![
        dir.path().join("a.txt"),
        dir.path().join("b.txt"),
        dir.path().join("c.txt"),
    ];

    let groups = hasher::group_by_size(&files);
    assert_eq!(groups.len(), 0, "No size groups when all sizes are unique");
}

#[test]
fn test_group_by_full_hash() {
    let dir = TempDir::new().unwrap();

    std::fs::write(dir.path().join("a.txt"), "duplicate content").unwrap();
    std::fs::write(dir.path().join("b.txt"), "duplicate content").unwrap();
    std::fs::write(dir.path().join("c.txt"), "unique content").unwrap();

    let files = vec![
        dir.path().join("a.txt"),
        dir.path().join("b.txt"),
        dir.path().join("c.txt"),
    ];

    let groups = hasher::group_by_full_hash(&files);
    assert_eq!(groups.len(), 1, "Should have 1 hash group for the duplicates");
}

#[test]
fn test_empty_file_hashing() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("empty.txt");
    std::fs::write(&file, b"").unwrap();

    let hash = hasher::quick_hash(&file).unwrap();
    assert!(!hash.is_empty(), "Hash of empty file should still produce a value");

    let full = hasher::full_hash(&file).unwrap();
    assert_eq!(hash, full, "Quick and full hash of empty file should be identical");
}
