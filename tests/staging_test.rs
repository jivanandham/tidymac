use tempfile::TempDir;

use tidymac::cleaner::manifest::{CleanManifest, ManifestItem};
use tidymac::scanner::targets::{Category, FileEntry, SafetyLevel, ScanItem};

/// Helper to create a ScanItem with real files for testing
fn create_test_scan_item(dir: &std::path::Path, name: &str, file_count: usize) -> ScanItem {
    let mut files = Vec::new();
    let mut total_size = 0u64;

    for i in 0..file_count {
        let content = format!("test content for file {} in {}", i, name);
        let file_path = dir.join(format!("{}_{}.txt", name, i));
        std::fs::write(&file_path, &content).unwrap();
        let size = content.len() as u64;
        total_size += size;

        files.push(FileEntry {
            path: file_path,
            size_bytes: size,
            modified: Some(std::time::SystemTime::now()),
        });
    }

    ScanItem {
        name: name.to_string(),
        category: Category::TempFiles,
        path: dir.to_path_buf(),
        size_bytes: total_size,
        file_count,
        safety: SafetyLevel::Safe,
        reason: "Test item".to_string(),
        files,
    }
}

#[test]
fn test_manifest_creation() {
    let manifest = CleanManifest::new("developer", "soft_delete", 7);

    assert_eq!(manifest.profile, "developer");
    assert_eq!(manifest.mode, "soft_delete");
    assert_eq!(manifest.total_bytes, 0);
    assert_eq!(manifest.total_files, 0);
    assert!(!manifest.restored);
    assert!(manifest.expires_at.is_some());
    assert!(!manifest.session_id.is_empty());
}

#[test]
fn test_manifest_add_item() {
    let mut manifest = CleanManifest::new("quick", "soft_delete", 7);

    manifest.add_item(ManifestItem {
        original_path: std::path::PathBuf::from("/tmp/test.txt"),
        staged_path: Some(std::path::PathBuf::from("/tmp/staged/0001")),
        size_bytes: 1024,
        category: "TempFiles".to_string(),
        safety: "Safe".to_string(),
        is_dir: false,
        success: true,
        error: None,
    });

    assert_eq!(manifest.total_files, 1);
    assert_eq!(manifest.total_bytes, 1024);
    assert_eq!(manifest.items.len(), 1);
}

#[test]
fn test_manifest_failed_item_not_counted() {
    let mut manifest = CleanManifest::new("quick", "soft_delete", 7);

    manifest.add_item(ManifestItem {
        original_path: std::path::PathBuf::from("/tmp/test.txt"),
        staged_path: None,
        size_bytes: 1024,
        category: "TempFiles".to_string(),
        safety: "Safe".to_string(),
        is_dir: false,
        success: false,
        error: Some("Permission denied".to_string()),
    });

    assert_eq!(manifest.total_files, 0, "Failed items should not be counted");
    assert_eq!(manifest.total_bytes, 0);
    assert_eq!(manifest.items.len(), 1, "Failed items should still be in the list");
}

#[test]
fn test_manifest_hard_delete_no_expiry() {
    let manifest = CleanManifest::new("quick", "hard_delete", 0);
    assert!(manifest.expires_at.is_none(), "Hard delete should have no expiry");
    assert!(!manifest.is_expired());
}

#[test]
fn test_staging_moves_files() {
    let source_dir = TempDir::new().unwrap();
    let staging_dir = TempDir::new().unwrap();

    // Create test files
    let item = create_test_scan_item(source_dir.path(), "cache", 3);

    // Verify source files exist
    for f in &item.files {
        assert!(f.path.exists(), "Source file should exist before staging");
    }

    let mut manifest = CleanManifest::new("test", "soft_delete", 7);
    // Override the staging dir for testing
    let session_files_dir = staging_dir.path().join("files");
    std::fs::create_dir_all(&session_files_dir).unwrap();

    // Stage each file manually (since stage_files uses Config paths)
    for (i, file_entry) in item.files.iter().enumerate() {
        let staged_name = format!("{:06}", i + 1);
        let staged_path = session_files_dir.join(&staged_name);

        std::fs::rename(&file_entry.path, &staged_path).unwrap();

        manifest.add_item(ManifestItem {
            original_path: file_entry.path.clone(),
            staged_path: Some(staged_path),
            size_bytes: file_entry.size_bytes,
            category: "TempFiles".to_string(),
            safety: "Safe".to_string(),
            is_dir: false,
            success: true,
            error: None,
        });
    }

    assert_eq!(manifest.total_files, 3);

    // Verify source files are gone
    for f in &item.files {
        assert!(!f.path.exists(), "Source file should be gone after staging");
    }

    // Verify staged files exist
    for mi in &manifest.items {
        if let Some(ref sp) = mi.staged_path {
            assert!(sp.exists(), "Staged file should exist");
        }
    }
}

#[test]
fn test_restore_moves_files_back() {
    let source_dir = TempDir::new().unwrap();
    let staging_dir = TempDir::new().unwrap();

    // Create and "stage" files
    let original_content = "restore me please";
    let original_path = source_dir.path().join("important.txt");
    std::fs::write(&original_path, original_content).unwrap();

    let staged_path = staging_dir.path().join("000001");
    std::fs::rename(&original_path, &staged_path).unwrap();

    assert!(!original_path.exists());
    assert!(staged_path.exists());

    // Restore
    std::fs::rename(&staged_path, &original_path).unwrap();

    assert!(original_path.exists());
    assert!(!staged_path.exists());

    // Verify content preserved
    let restored_content = std::fs::read_to_string(&original_path).unwrap();
    assert_eq!(restored_content, original_content, "Content should be preserved after restore");
}

#[test]
fn test_manifest_serialization_roundtrip() {
    let dir = TempDir::new().unwrap();

    let mut manifest = CleanManifest::new("developer", "soft_delete", 7);
    manifest.add_item(ManifestItem {
        original_path: std::path::PathBuf::from("/Users/test/Library/Caches/foo"),
        staged_path: Some(std::path::PathBuf::from("/tmp/staging/000001")),
        size_bytes: 4096,
        category: "UserCache".to_string(),
        safety: "Safe".to_string(),
        is_dir: true,
        success: true,
        error: None,
    });
    manifest.add_error("Test warning".to_string());

    // Serialize
    let json = serde_json::to_string_pretty(&manifest).unwrap();
    let manifest_path = dir.path().join("manifest.json");
    std::fs::write(&manifest_path, &json).unwrap();

    // Deserialize
    let loaded_json = std::fs::read_to_string(&manifest_path).unwrap();
    let loaded: CleanManifest = serde_json::from_str(&loaded_json).unwrap();

    assert_eq!(loaded.session_id, manifest.session_id);
    assert_eq!(loaded.profile, "developer");
    assert_eq!(loaded.mode, "soft_delete");
    assert_eq!(loaded.total_files, 1);
    assert_eq!(loaded.total_bytes, 4096);
    assert_eq!(loaded.items.len(), 1);
    assert_eq!(loaded.errors.len(), 1);
    assert!(!loaded.restored);
}
