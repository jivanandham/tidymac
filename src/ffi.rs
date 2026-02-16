//! FFI bridge for SwiftUI GUI
//!
//! Exposes TidyMac core functionality via C-ABI functions that Swift can call.
//! All returned strings are JSON-encoded and must be freed by the caller.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::common::config::Config;
use crate::common::format;
use crate::profiles::loader::Profile;
use crate::scanner;

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Convert a Rust string to a C string pointer. Caller must free with `tidymac_free_string`.
fn to_c_string(s: &str) -> *mut c_char {
    CString::new(s).unwrap_or_default().into_raw()
}

/// Convert a JSON-serializable value to a C string pointer.
fn json_to_c<T: serde::Serialize>(val: &T) -> *mut c_char {
    match serde_json::to_string(val) {
        Ok(s) => to_c_string(&s),
        Err(e) => to_c_string(&format!("{{\"error\":\"{}\"}}", e)),
    }
}

/// Return an error JSON as a C string.
fn error_c(msg: &str) -> *mut c_char {
    let val = serde_json::json!({"error": msg});
    to_c_string(&val.to_string())
}

// ─── Memory Management ──────────────────────────────────────────────────────

/// Free a string returned by any tidymac FFI function.
#[no_mangle]
pub extern "C" fn tidymac_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}

// ─── Scan ────────────────────────────────────────────────────────────────────

/// Run a scan with the given profile name. Returns JSON string.
/// Profile names: "quick", "developer", "creative", "deep"
#[no_mangle]
pub extern "C" fn tidymac_scan(profile_name: *const c_char) -> *mut c_char {
    let profile_name = if profile_name.is_null() {
        "quick".to_string()
    } else {
        unsafe { CStr::from_ptr(profile_name) }
            .to_str()
            .unwrap_or("quick")
            .to_string()
    };

    let profile = match Profile::load(&profile_name) {
        Ok(p) => p,
        Err(e) => return error_c(&format!("Failed to load profile: {}", e)),
    };

    let config = match Config::load() {
        Ok(c) => c,
        Err(e) => return error_c(&format!("Failed to load config: {}", e)),
    };

    let scan_targets = profile.enabled_targets();
    let results = match scanner::run_scan_with_cache(
        &scan_targets,
        false,
        profile.includes_dev_projects(),
        profile.thresholds.stale_days,
        config.large_file_threshold_bytes(),
        true,
        &profile_name,
    ) {
        Ok(r) => r,
        Err(e) => return error_c(&format!("Scan failed: {}", e)),
    };

    // Build a JSON-friendly response
    let response = serde_json::json!({
        "profile": profile_name,
        "duration_secs": results.duration_secs,
        "total_reclaimable": results.total_reclaimable,
        "total_reclaimable_formatted": format::format_size(results.total_reclaimable),
        "total_files": results.total_files,
        "items": results.items.iter().map(|item| {
            serde_json::json!({
                "name": item.name,
                "category": format!("{}", item.category),
                "path": item.path.display().to_string(),
                "size_bytes": item.size_bytes,
                "size_formatted": format::format_size(item.size_bytes),
                "file_count": item.file_count,
                "safety": format!("{:?}", item.safety),
                "reason": item.reason,
            })
        }).collect::<Vec<_>>(),
        "errors": results.errors,
    });

    json_to_c(&response)
}

// ─── Disk Usage ──────────────────────────────────────────────────────────────

/// Get disk usage breakdown. Returns JSON string.
#[no_mangle]
pub extern "C" fn tidymac_disk_usage() -> *mut c_char {
    let usage = crate::viz::analyze_disk_usage();

    let response = serde_json::json!({
        "total_capacity": usage.total_capacity,
        "total_capacity_formatted": format::format_size(usage.total_capacity),
        "used": usage.used,
        "used_formatted": format::format_size(usage.used),
        "available": usage.available,
        "available_formatted": format::format_size(usage.available),
        "used_percentage": if usage.total_capacity > 0 {
            (usage.used as f64 / usage.total_capacity as f64 * 100.0) as u32
        } else { 0 },
        "categories": usage.categories.iter().map(|cat| {
            serde_json::json!({
                "name": cat.name,
                "icon": cat.icon,
                "path": cat.path.display().to_string(),
                "size": cat.size,
                "size_formatted": format::format_size(cat.size),
            })
        }).collect::<Vec<_>>(),
    });

    json_to_c(&response)
}

// ─── Apps ────────────────────────────────────────────────────────────────────

/// Discover all installed applications. Returns JSON string.
#[no_mangle]
pub extern "C" fn tidymac_apps_list() -> *mut c_char {
    let mut apps = crate::apps::discover_apps();
    apps.sort_by(|a, b| b.total_size.cmp(&a.total_size));

    let response: Vec<_> = apps
        .iter()
        .map(|a| {
            serde_json::json!({
                "name": a.name,
                "bundle_id": a.bundle_id,
                "version": a.version,
                "path": a.path.display().to_string(),
                "app_size": a.app_size,
                "app_size_formatted": format::format_size(a.app_size),
                "total_size": a.total_size,
                "total_size_formatted": format::format_size(a.total_size),
                "leftovers_size": a.total_size.saturating_sub(a.app_size),
                "leftovers_formatted": format::format_size(a.total_size.saturating_sub(a.app_size)),
                "source": format!("{}", a.source),
                "associated_files": a.associated_files.iter().filter(|f| f.exists).map(|f| {
                    serde_json::json!({
                        "path": f.path.display().to_string(),
                        "size": f.size,
                        "size_formatted": format::format_size(f.size),
                        "kind": format!("{}", f.kind),
                    })
                }).collect::<Vec<_>>(),
            })
        })
        .collect();

    json_to_c(&response)
}

/// Clean leftovers (caches, logs, app support, etc.) for a specific app by name.
/// Does NOT remove the app bundle itself.
/// Returns JSON string with report.
#[no_mangle]
pub extern "C" fn tidymac_app_clean_leftovers(app_name: *const c_char) -> *mut c_char {
    if app_name.is_null() {
        return error_c("app_name is required");
    }

    let app_name = unsafe { CStr::from_ptr(app_name) }
        .to_str()
        .unwrap_or("");

    let apps = crate::apps::discover_apps();
    let matches = crate::apps::find_app_by_name(&apps, app_name);

    if matches.is_empty() {
        return error_c(&format!("No app found matching '{}'", app_name));
    }

    let app = matches[0];
    let mut removed = 0usize;
    let mut freed = 0u64;
    let mut errors = Vec::new();
    let mut skipped = Vec::new();
    let mut removed_paths = Vec::new();

    // Paths under these directories are SIP-protected on macOS and require
    // Full Disk Access. We skip them and inform the user instead of failing.
    let sip_protected_components = ["Containers", "Group Containers"];

    for assoc in &app.associated_files {
        if !assoc.exists || assoc.size == 0 {
            continue;
        }

        // Check if path is under a SIP-protected directory
        let path_str = assoc.path.display().to_string();
        let is_protected = sip_protected_components.iter().any(|comp| {
            path_str.contains(&format!("/Library/{}/", comp))
        });

        if is_protected {
            skipped.push(format!(
                "{} ({}) — protected by macOS. Grant Full Disk Access in System Settings > Privacy & Security to clean this.",
                path_str,
                format::format_size(assoc.size),
            ));
            continue;
        }

        let result = if assoc.path.is_dir() {
            std::fs::remove_dir_all(&assoc.path)
        } else {
            std::fs::remove_file(&assoc.path)
        };
        match result {
            Ok(()) => {
                freed += assoc.size;
                removed += 1;
                removed_paths.push(assoc.path.display().to_string());
            }
            Err(e) => {
                let msg = if e.raw_os_error() == Some(1) {
                    format!("{}: Permission denied — grant Full Disk Access in System Settings", path_str)
                } else {
                    format!("{}: {}", path_str, e)
                };
                errors.push(msg);
            }
        }
    }

    let response = serde_json::json!({
        "app_name": app.name,
        "files_removed": removed,
        "bytes_freed": freed,
        "bytes_freed_formatted": format::format_size(freed),
        "removed_paths": removed_paths,
        "skipped": skipped,
        "errors": errors,
    });

    json_to_c(&response)
}

// ─── Clean ───────────────────────────────────────────────────────────────────

/// Run a clean operation on selected items only. Returns JSON string with report.
/// profile_name: profile to scan with
/// mode: "dry_run", "soft", "hard"
/// selected_names_json: JSON array of item names to clean, e.g. ["User Cache Files","npm Cache"]
///                      If NULL or empty, cleans ALL items from the scan.
#[no_mangle]
pub extern "C" fn tidymac_clean(
    profile_name: *const c_char,
    mode: *const c_char,
    selected_names_json: *const c_char,
) -> *mut c_char {
    let profile_name = if profile_name.is_null() {
        "quick".to_string()
    } else {
        unsafe { CStr::from_ptr(profile_name) }
            .to_str()
            .unwrap_or("quick")
            .to_string()
    };

    let mode_str = if mode.is_null() {
        "soft".to_string()
    } else {
        unsafe { CStr::from_ptr(mode) }
            .to_str()
            .unwrap_or("soft")
            .to_string()
    };

    // Parse selected item names filter
    let selected_names: Option<Vec<String>> = if selected_names_json.is_null() {
        None
    } else {
        let json_str = unsafe { CStr::from_ptr(selected_names_json) }
            .to_str()
            .unwrap_or("[]");
        serde_json::from_str(json_str).ok()
    };

    let clean_mode = match mode_str.as_str() {
        "dry_run" => crate::cleaner::CleanMode::DryRun,
        "hard" => crate::cleaner::CleanMode::HardDelete,
        _ => crate::cleaner::CleanMode::SoftDelete,
    };

    let profile = match Profile::load(&profile_name) {
        Ok(p) => p,
        Err(e) => return error_c(&format!("Failed to load profile: {}", e)),
    };

    let config = match Config::load() {
        Ok(c) => c,
        Err(e) => return error_c(&format!("Failed to load config: {}", e)),
    };

    let scan_targets = profile.enabled_targets();
    let results = match scanner::run_scan(
        &scan_targets,
        false,
        profile.includes_dev_projects(),
        profile.thresholds.stale_days,
        config.large_file_threshold_bytes(),
    ) {
        Ok(r) => r,
        Err(e) => return error_c(&format!("Scan failed: {}", e)),
    };

    if results.items.is_empty() {
        return to_c_string(r#"{"files_removed":0,"bytes_freed":0,"message":"Nothing to clean"}"#);
    }

    // Filter to only selected items if a selection was provided
    let items_to_clean: Vec<_> = if let Some(ref names) = selected_names {
        if names.is_empty() {
            return to_c_string(r#"{"files_removed":0,"bytes_freed":0,"message":"No items selected"}"#);
        }
        results.items.iter().filter(|item| names.contains(&item.name)).cloned().collect()
    } else {
        results.items.clone()
    };

    if items_to_clean.is_empty() {
        return to_c_string(r#"{"files_removed":0,"bytes_freed":0,"message":"No matching items found"}"#);
    }

    let report = match crate::cleaner::clean(&items_to_clean, clean_mode, &profile_name, false) {
        Ok(r) => r,
        Err(e) => return error_c(&format!("Clean failed: {}", e)),
    };

    let response = serde_json::json!({
        "mode": format!("{}", report.mode),
        "files_removed": report.files_removed,
        "bytes_freed": report.bytes_freed,
        "bytes_freed_formatted": format::format_size(report.bytes_freed),
        "session_id": report.session_id,
        "errors": report.errors,
    });

    json_to_c(&response)
}

// ─── Privacy ─────────────────────────────────────────────────────────────────

/// Run privacy audit. Returns JSON string.
#[no_mangle]
pub extern "C" fn tidymac_privacy_scan() -> *mut c_char {
    let report = crate::privacy::run_privacy_audit(true, true);

    let response = serde_json::json!({
        "browser_profiles": report.browser_profiles.iter().map(|p| {
            serde_json::json!({
                "browser": format!("{}", p.browser),
                "cookies_size": p.cookies_size,
                "cookies_size_formatted": format::format_size(p.cookies_size),
                "history_size": p.history_size,
                "history_size_formatted": format::format_size(p.history_size),
                "local_storage_size": p.local_storage_size,
                "cache_size": p.cache_size,
                "cache_size_formatted": format::format_size(p.cache_size),
                "total_size": p.total_size,
                "total_size_formatted": format::format_size(p.total_size),
            })
        }).collect::<Vec<_>>(),
        "tracking_apps": report.tracking_apps.iter().map(|t| {
            serde_json::json!({
                "name": t.name,
                "kind": format!("{}", t.kind),
                "data_size": t.data_size,
                "data_size_formatted": format::format_size(t.data_size),
            })
        }).collect::<Vec<_>>(),
        "total_privacy_data_size": report.total_privacy_data_size,
        "total_privacy_data_formatted": format::format_size(report.total_privacy_data_size),
        "cookie_locations_count": report.cookie_locations.len(),
    });

    json_to_c(&response)
}

// ─── Docker ──────────────────────────────────────────────────────────────────

/// Get Docker usage. Returns JSON string.
#[no_mangle]
pub extern "C" fn tidymac_docker_usage() -> *mut c_char {
    let usage = crate::scanner::docker::get_docker_usage();

    let response = serde_json::json!({
        "installed": usage.installed,
        "running": usage.running,
        "total_size": usage.total_size,
        "total_size_formatted": format::format_size(usage.total_size),
        "reclaimable": usage.reclaimable,
        "reclaimable_formatted": format::format_size(usage.reclaimable),
        "images": {
            "count": usage.images.count,
            "size": usage.images.size,
            "size_formatted": format::format_size(usage.images.size),
        },
        "containers": {
            "count": usage.containers.count,
            "size": usage.containers.size,
            "size_formatted": format::format_size(usage.containers.size),
        },
        "volumes": {
            "count": usage.volumes.count,
            "size": usage.volumes.size,
            "size_formatted": format::format_size(usage.volumes.size),
        },
        "build_cache": {
            "size": usage.build_cache.size,
            "size_formatted": format::format_size(usage.build_cache.size),
        },
    });

    json_to_c(&response)
}

// ─── Undo ────────────────────────────────────────────────────────────────────

/// List undo sessions. Returns JSON string.
#[no_mangle]
pub extern "C" fn tidymac_undo_list() -> *mut c_char {
    let sessions = match crate::cleaner::CleanManifest::list_sessions() {
        Ok(s) => s,
        Err(e) => return error_c(&format!("Failed to list sessions: {}", e)),
    };

    let response: Vec<_> = sessions
        .iter()
        .map(|s| {
            serde_json::json!({
                "session_id": s.session_id,
                "profile": s.profile,
                "timestamp": s.timestamp.to_rfc3339(),
                "mode": s.mode,
                "total_files": s.total_files,
                "total_bytes": s.total_bytes,
                "total_bytes_formatted": format::format_size(s.total_bytes),
                "expires_at": s.expires_at.map(|e| e.to_rfc3339()),
                "restored": s.restored,
                "is_expired": s.is_expired,
            })
        })
        .collect();

    json_to_c(&response)
}

/// Restore a session by ID. Returns JSON string.
#[no_mangle]
pub extern "C" fn tidymac_undo_session(session_id: *const c_char) -> *mut c_char {
    if session_id.is_null() {
        return error_c("session_id is required");
    }

    let session_id = unsafe { CStr::from_ptr(session_id) }
        .to_str()
        .unwrap_or("");

    let report = match crate::cleaner::restore_session(session_id, false) {
        Ok(r) => r,
        Err(e) => return error_c(&format!("Restore failed: {}", e)),
    };

    let response = serde_json::json!({
        "session_id": report.session_id,
        "restored_count": report.restored_count,
        "restored_bytes": report.restored_bytes,
        "restored_bytes_formatted": format::format_size(report.restored_bytes),
        "errors": report.errors,
    });

    json_to_c(&response)
}

// ─── Profiles ────────────────────────────────────────────────────────────────

/// List available profiles. Returns JSON string.
#[no_mangle]
pub extern "C" fn tidymac_profiles_list() -> *mut c_char {
    let profiles = Profile::available_profiles();

    let response: Vec<_> = profiles
        .iter()
        .map(|name| {
            let info = Profile::load(name).ok().map(|p| {
                serde_json::json!({
                    "name": name,
                    "description": p.profile.description,
                    "aggression": format!("{:?}", p.profile.aggression),
                })
            });
            info.unwrap_or_else(|| serde_json::json!({"name": name}))
        })
        .collect();

    json_to_c(&response)
}

// ─── Version ─────────────────────────────────────────────────────────────────

/// Get TidyMac version. Returns a C string (not JSON).
#[no_mangle]
pub extern "C" fn tidymac_version() -> *mut c_char {
    to_c_string(env!("CARGO_PKG_VERSION"))
}
