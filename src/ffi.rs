//! FFI bridge for the SwiftUI app.
//!
//! All returned strings are JSON and must be freed with `tidymac_free_string`.
//! Every function body runs inside `catch_unwind` so a Rust panic can't cross
//! the C ABI.
//!
//! Cleaning from the GUI always acts on **the scan the user reviewed**: the
//! last scan is kept in memory with a `scan_id`, and `tidymac_clean` removes
//! only entries from that scan (minus any the user unticked). Nothing is
//! re-scanned behind the user's back, and an empty selection removes nothing.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

pub static CANCEL_FLAG: AtomicBool = AtomicBool::new(false);

use crate::cleaner::{CleanMode, CleanReport, DeleteOptions, DeleteRequest};
use crate::common::format;
use crate::common::safety::DeletePolicy;
use crate::profiles::loader::Profile;
use crate::scanner;
use crate::scanner::targets::ScanResults;

/// The most recent GUI scan, kept so cleaning acts on exactly what was shown.
struct StoredScan {
    scan_id: String,
    profile: String,
    results: ScanResults,
}

static LAST_SCAN: Mutex<Option<StoredScan>> = Mutex::new(None);

/// Max entries per item sent to the GUI (the full list stays in LAST_SCAN).
const MAX_ENTRIES_IN_JSON: usize = 500;

// ─── Panic Safety Macro ──────────────────────────────────────────────────────

macro_rules! ffi_safe {
    ($body:block) => {{
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| $body)) {
            Ok(result) => result,
            Err(panic) => {
                let msg = if let Some(s) = panic.downcast_ref::<&str>() {
                    format!("Internal error (panic): {}", s)
                } else if let Some(s) = panic.downcast_ref::<String>() {
                    format!("Internal error (panic): {}", s)
                } else {
                    "Internal error (unknown panic)".to_string()
                };
                error_c(&msg)
            }
        }
    }};
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn to_c_string(s: &str) -> *mut c_char {
    CString::new(s.replace('\0', "")).unwrap_or_default().into_raw()
}

fn json_to_c<T: serde::Serialize>(val: &T) -> *mut c_char {
    match serde_json::to_string(val) {
        Ok(s) => to_c_string(&s),
        Err(e) => error_c(&e.to_string()),
    }
}

fn error_c(msg: &str) -> *mut c_char {
    let val = serde_json::json!({ "error": msg });
    to_c_string(&val.to_string())
}

/// Read a C string argument (None if null or not UTF-8).
fn arg(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(ptr) }.to_str().ok().map(|s| s.to_string())
    }
}

/// Parse a JSON argument.
fn json_arg(ptr: *const c_char) -> Option<serde_json::Value> {
    arg(ptr).and_then(|s| serde_json::from_str(&s).ok())
}

fn mode_from(s: Option<&str>) -> CleanMode {
    match s.unwrap_or("soft") {
        "dry_run" => CleanMode::DryRun,
        "hard" | "hard_delete" => CleanMode::HardDelete,
        _ => CleanMode::SoftDelete,
    }
}

fn strings(v: &serde_json::Value, key: &str) -> Vec<String> {
    v.get(key)
        .and_then(|x| x.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
        .unwrap_or_default()
}

fn rfc3339(t: Option<std::time::SystemTime>) -> Option<String> {
    t.map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339())
}

/// JSON for a clean report (shared by every removal function).
pub fn clean_report_json(report: &CleanReport) -> serde_json::Value {
    serde_json::json!({
        "mode": format!("{}", report.mode),
        "files_removed": report.files_removed,
        "bytes_freed": report.bytes_freed,
        "bytes_freed_formatted": format::format_size(report.bytes_freed),
        "bytes_staged": report.bytes_staged,
        "bytes_staged_formatted": format::format_size(report.bytes_staged),
        "session_id": report.session_id,
        "expires_at": report.expires_at.map(|e| e.to_rfc3339()),
        "errors": report.errors,
        "warnings": report.warnings,
        "tool_actions": report.tool_actions,
        "skipped": report.skipped.iter().map(|s| serde_json::json!({
            "path": s.path.display().to_string(),
            "reason": s.reason,
        })).collect::<Vec<_>>(),
        "removed_paths": report.removed_paths.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
    })
}

// ─── Memory / setup ──────────────────────────────────────────────────────────

/// Free a string returned by any tidymac FFI function.
#[no_mangle]
pub extern "C" fn tidymac_free_string(ptr: *mut c_char) {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if !ptr.is_null() {
            unsafe {
                let _ = CString::from_raw(ptr);
            }
        }
    }));
}

/// Initialize local logging (no network, no crash reporter).
#[no_mangle]
pub extern "C" fn tidymac_init_observability(verbose: bool) {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = crate::common::observability::init(verbose);
    }));
}

/// Whether this process has Full Disk Access.
#[no_mangle]
pub extern "C" fn tidymac_has_full_disk_access() -> bool {
    std::panic::catch_unwind(crate::common::permissions::has_full_disk_access).unwrap_or(false)
}

// ─── Scan ────────────────────────────────────────────────────────────────────

/// Run a fresh scan with the given profile. Returns JSON with a `scan_id`.
#[no_mangle]
pub extern "C" fn tidymac_scan(profile_name: *const c_char) -> *mut c_char {
    ffi_safe!({
        CANCEL_FLAG.store(false, Ordering::Relaxed);
        let profile_name = arg(profile_name).unwrap_or_else(|| "quick".to_string());

        let profile = match Profile::load(&profile_name) {
            Ok(p) => p,
            Err(e) => return error_c(&format!("Failed to load profile: {}", e)),
        };

        let results = match scanner::run_scan(
            &profile.enabled_targets(),
            false,
            profile.includes_dev_projects(),
            profile.thresholds.stale_days,
            profile.large_file_threshold(),
        ) {
            Ok(r) => r,
            Err(e) => {
                if CANCEL_FLAG.load(Ordering::Relaxed) {
                    return error_c("CANCELLED");
                }
                return error_c(&format!("Scan failed: {}", e));
            }
        };
        if CANCEL_FLAG.load(Ordering::Relaxed) {
            return error_c("CANCELLED");
        }

        let scan_id = uuid::Uuid::new_v4().to_string();
        let mut errors = results.errors.clone();
        errors.extend(profile.rule_problems());

        let response = serde_json::json!({
            "scan_id": scan_id,
            "profile": profile_name,
            "duration_secs": results.duration_secs,
            "total_reclaimable": results.total_reclaimable,
            "total_reclaimable_formatted": format::format_size(results.total_reclaimable),
            "total_files": results.total_files,
            "items": results.items.iter().enumerate().map(|(idx, item)| {
                serde_json::json!({
                    "id": idx,
                    "name": item.name,
                    "category": format!("{}", item.category),
                    "path": item.path.display().to_string(),
                    "size_bytes": item.size_bytes,
                    "size_formatted": format::format_size(item.size_bytes),
                    "reclaimable_bytes": item.reclaimable_bytes,
                    "reclaimable_formatted": format::format_size(item.reclaimable_bytes),
                    "file_count": item.file_count,
                    "entry_count": item.files.len(),
                    "safety": format!("{:?}", item.safety),
                    "reason": item.reason,
                    "review_only": item.review_only,
                    "cleanable": item.is_cleanable(),
                    "tool_action": item.tool_action.as_ref().map(|t| t.command_line()),
                    "entries": item.files.iter().take(MAX_ENTRIES_IN_JSON).map(|f| serde_json::json!({
                        "path": f.path.display().to_string(),
                        "size_bytes": f.size_bytes,
                        "size_formatted": format::format_size(f.size_bytes),
                        "modified": rfc3339(f.modified),
                    })).collect::<Vec<_>>(),
                })
            }).collect::<Vec<_>>(),
            "errors": errors,
        });

        if let Ok(mut guard) = LAST_SCAN.lock() {
            *guard = Some(StoredScan {
                scan_id,
                profile: profile_name,
                results,
            });
        }

        json_to_c(&response)
    })
}

// ─── Clean ───────────────────────────────────────────────────────────────────

/// Clean items from the last scan.
///
/// `selection_json` is either a JSON array of item names (legacy) or an object:
/// `{"scan_id": "...", "items": ["User Cache Files", ...] | [0, 3, ...], "exclude_paths": ["/path", ...]}`.
/// A null or empty selection removes **nothing**.
/// `mode`: "dry_run", "soft" (default), "hard".
#[no_mangle]
pub extern "C" fn tidymac_clean(
    profile_name: *const c_char,
    mode: *const c_char,
    selection_json: *const c_char,
) -> *mut c_char {
    ffi_safe!({
        let profile_name = arg(profile_name).unwrap_or_else(|| "quick".to_string());
        let clean_mode = mode_from(arg(mode).as_deref());

        let selection = match json_arg(selection_json) {
            Some(v) => v,
            None => return error_c("Nothing selected"),
        };
        let (scan_id, item_keys, exclude_paths): (Option<String>, Vec<serde_json::Value>, Vec<PathBuf>) =
            match &selection {
                serde_json::Value::Array(a) => (None, a.clone(), Vec::new()),
                serde_json::Value::Object(_) => (
                    selection.get("scan_id").and_then(|s| s.as_str()).map(String::from),
                    selection.get("items").and_then(|i| i.as_array()).cloned().unwrap_or_default(),
                    strings(&selection, "exclude_paths").into_iter().map(PathBuf::from).collect(),
                ),
                _ => return error_c("Invalid selection"),
            };
        if item_keys.is_empty() {
            return error_c("Nothing selected");
        }

        let guard = match LAST_SCAN.lock() {
            Ok(g) => g,
            Err(_) => return error_c("Internal error: scan state unavailable"),
        };
        let stored = match guard.as_ref() {
            Some(s) => s,
            None => return error_c("Run a scan first"),
        };
        if let Some(ref id) = scan_id {
            if *id != stored.scan_id {
                return error_c("These results are out of date — scan again before cleaning");
            }
        }
        if stored.profile != profile_name {
            return error_c("The last scan used a different profile — scan again before cleaning");
        }

        let selected: Vec<_> = stored
            .results
            .items
            .iter()
            .enumerate()
            .filter(|(idx, item)| {
                item_keys.iter().any(|k| match k {
                    serde_json::Value::Number(n) => n.as_u64() == Some(*idx as u64),
                    serde_json::Value::String(s) => *s == item.name,
                    _ => false,
                })
            })
            .map(|(_, item)| item.clone())
            .collect();
        drop(guard);

        if selected.is_empty() {
            return error_c("None of the selected items are in the last scan");
        }

        let opts = crate::cleaner::CleanOptions {
            show_progress: false,
            force: false,
            exclude_paths,
        };
        match crate::cleaner::clean_with(&selected, clean_mode, &profile_name, &opts) {
            Ok(report) => {
                // Cleaned entries are gone: forget the scan so it can't be reused.
                if let Ok(mut g) = LAST_SCAN.lock() {
                    *g = None;
                }
                json_to_c(&clean_report_json(&report))
            }
            Err(e) => error_c(&format!("Clean failed: {}", e)),
        }
    })
}

/// Remove individually chosen files (duplicates, large files).
/// `request_json`: `{"paths": [...], "mode": "soft"|"hard"|"dry_run", "label": "duplicates"}`.
/// Only single regular files are accepted, and the safety guard still applies.
#[no_mangle]
pub extern "C" fn tidymac_delete_paths(request_json: *const c_char) -> *mut c_char {
    ffi_safe!({
        let req = match json_arg(request_json) {
            Some(v) => v,
            None => return error_c("Invalid request"),
        };
        let paths = strings(&req, "paths");
        if paths.is_empty() {
            return error_c("Nothing selected");
        }
        let mode = mode_from(req.get("mode").and_then(|m| m.as_str()));
        let label = req.get("label").and_then(|l| l.as_str()).unwrap_or("manual");
        let requests: Vec<DeleteRequest> = paths
            .into_iter()
            .map(|p| {
                let path = PathBuf::from(p);
                let size = std::fs::symlink_metadata(&path)
                    .map(|m| {
                        use std::os::unix::fs::MetadataExt;
                        m.blocks() * 512
                    })
                    .unwrap_or(0);
                DeleteRequest {
                    path,
                    size_bytes: size,
                    category: label.to_string(),
                    safety: "Caution".into(),
                    roots: Vec::new(),
                    policy: DeletePolicy::UserFile,
                }
            })
            .collect();
        let opts = DeleteOptions::new(mode, label);
        match crate::cleaner::execute(requests, Vec::new(), &opts) {
            Ok(r) => json_to_c(&clean_report_json(&r)),
            Err(e) => error_c(&e.to_string()),
        }
    })
}

// ─── Disk usage / map / system data ──────────────────────────────────────────

/// Get disk usage breakdown. Returns JSON string.
#[no_mangle]
pub extern "C" fn tidymac_disk_usage() -> *mut c_char {
    ffi_safe!({
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
    })
}

/// Build a disk map rooted at `path` (default: home). Returns a JSON tree.
#[no_mangle]
pub extern "C" fn tidymac_disk_tree(path: *const c_char, max_depth: u32, max_children: u32) -> *mut c_char {
    ffi_safe!({
        CANCEL_FLAG.store(false, Ordering::Relaxed);
        let root = arg(path)
            .map(PathBuf::from)
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_default());
        if !root.is_dir() {
            return error_c(&format!("Not a folder: {}", root.display()));
        }
        let opts = crate::viz::TreeOptions {
            max_depth: (max_depth.clamp(1, 6)) as usize,
            max_children: (max_children.clamp(4, 200)) as usize,
        };
        let tree = crate::viz::build_tree(&root, opts);
        if CANCEL_FLAG.load(Ordering::Relaxed) {
            return error_c("CANCELLED");
        }
        json_to_c(&tree)
    })
}

/// Explain what's in "System Data". Returns JSON.
#[no_mangle]
pub extern "C" fn tidymac_system_data() -> *mut c_char {
    ffi_safe!({ json_to_c(&crate::viz::explain_system_data()) })
}

// ─── Apps ────────────────────────────────────────────────────────────────────

fn app_json(a: &crate::apps::InstalledApp) -> serde_json::Value {
    serde_json::json!({
        "name": a.name,
        "bundle_id": a.bundle_id,
        "version": a.version,
        "executable": a.executable,
        "path": a.path.display().to_string(),
        "is_apple": a.is_apple(),
        "app_size": a.app_size,
        "app_size_formatted": format::format_size(a.app_size),
        "total_size": a.total_size,
        "total_size_formatted": format::format_size(a.total_size),
        "leftovers_size": a.leftovers_size(),
        "leftovers_formatted": format::format_size(a.leftovers_size()),
        "source": format!("{}", a.source),
        "last_opened": rfc3339(a.last_opened),
        "associated_files": a.associated_files.iter().filter(|f| f.exists).map(associated_json).collect::<Vec<_>>(),
    })
}

fn associated_json(f: &crate::apps::AssociatedFile) -> serde_json::Value {
    serde_json::json!({
        "path": f.path.display().to_string(),
        "size": f.size,
        "size_formatted": format::format_size(f.size),
        "kind": format!("{}", f.kind),
        "matched_by": format!("{}", f.matched_by),
        "needs_confirmation": f.needs_confirmation(),
        "regenerable": f.kind.is_regenerable(),
    })
}

/// Discover all installed applications. Returns JSON array.
#[no_mangle]
pub extern "C" fn tidymac_apps_list() -> *mut c_char {
    ffi_safe!({
        let mut apps = crate::apps::discover_apps();
        apps.sort_by(|a, b| b.total_size.cmp(&a.total_size));
        let response: Vec<_> = apps.iter().map(app_json).collect();
        json_to_c(&response)
    })
}

/// Resolve an app by bundle id or path (exact). Error JSON if not found.
fn resolve_for_ffi(query: &str) -> Result<crate::apps::InstalledApp, String> {
    let apps = crate::apps::discover_apps();
    match crate::apps::resolve_app(&apps, query) {
        crate::apps::AppLookup::Found(a) => Ok(a.clone()),
        crate::apps::AppLookup::Ambiguous(list) => Err(format!(
            "Several apps are named '{}'. Use the bundle id: {}",
            query,
            list.iter().filter_map(|a| a.bundle_id.clone()).collect::<Vec<_>>().join(", ")
        )),
        crate::apps::AppLookup::NotFound(_) => Err(format!("No app found for '{}'", query)),
    }
}

/// Clear regenerable leftovers (caches, logs, saved state, crash reports) of an
/// installed app. Its settings, documents and containers are kept.
/// `app_identifier`: bundle id or app path.
#[no_mangle]
pub extern "C" fn tidymac_app_clean_leftovers(app_identifier: *const c_char) -> *mut c_char {
    ffi_safe!({
        let query = match arg(app_identifier) {
            Some(q) => q,
            None => return error_c("app identifier is required"),
        };
        let app = match resolve_for_ffi(&query) {
            Ok(a) => a,
            Err(e) => return error_c(&e),
        };
        let opts = crate::apps::UninstallOptions {
            mode: CleanMode::SoftDelete,
            remove_bundle: false,
            regenerable_only: true,
            ..Default::default()
        };
        match crate::apps::uninstall_app_with(&app, &opts) {
            Ok(r) => json_to_c(&uninstall_json(&r)),
            Err(e) => error_c(&e.to_string()),
        }
    })
}

fn uninstall_json(r: &crate::apps::UninstallReport) -> serde_json::Value {
    serde_json::json!({
        "app_name": r.app_name,
        "files_removed": r.files_removed,
        "bytes_freed": r.bytes_freed,
        "bytes_freed_formatted": format::format_size(r.bytes_freed),
        "bytes_staged": r.bytes_staged,
        "bytes_staged_formatted": format::format_size(r.bytes_staged),
        "session_id": r.session_id,
        "removed_paths": r.removed_paths.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
        "skipped": r.skipped.iter().map(|s| format!("{} — {}", format::format_path(&s.path), s.reason)).collect::<Vec<_>>(),
        "errors": r.errors,
    })
}

/// Uninstall an app.
/// `request_json`: `{"app": "<bundle id or path>", "mode": "soft"|"hard"|"dry_run",
///   "include_name_matches": false, "include_brew_zap": true, "only_paths": [...], "keep_bundle": false}`
#[no_mangle]
pub extern "C" fn tidymac_app_uninstall(request_json: *const c_char) -> *mut c_char {
    ffi_safe!({
        let req = match json_arg(request_json) {
            Some(v) => v,
            None => return error_c("Invalid request"),
        };
        let query = match req.get("app").and_then(|a| a.as_str()) {
            Some(q) => q.to_string(),
            None => return error_c("app is required"),
        };
        let app = match resolve_for_ffi(&query) {
            Ok(a) => a,
            Err(e) => return error_c(&e),
        };
        let flag = |k: &str| req.get(k).and_then(|v| v.as_bool()).unwrap_or(false);
        let extra = if req.get("include_brew_zap").and_then(|v| v.as_bool()).unwrap_or(true) {
            crate::apps::receipts::brew_zap_files(&app.path)
                .map(|(_, f)| f)
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let opts = crate::apps::UninstallOptions {
            mode: mode_from(req.get("mode").and_then(|m| m.as_str())),
            remove_bundle: !flag("keep_bundle"),
            regenerable_only: false,
            include_name_matches: flag("include_name_matches"),
            extra,
            only_paths: req
                .get("only_paths")
                .map(|_| strings(&req, "only_paths").into_iter().map(PathBuf::from).collect()),
        };
        match crate::apps::uninstall_app_with(&app, &opts) {
            Ok(r) => json_to_c(&uninstall_json(&r)),
            Err(e) => error_c(&e.to_string()),
        }
    })
}

/// Installer receipts and Homebrew cask info for an app path.
#[no_mangle]
pub extern "C" fn tidymac_app_install_info(app_path: *const c_char) -> *mut c_char {
    ffi_safe!({
        let path = match arg(app_path) {
            Some(p) => PathBuf::from(p),
            None => return error_c("app path is required"),
        };
        let info = crate::apps::install_info(&path);
        json_to_c(&serde_json::json!({
            "receipts": info.receipts,
            "brew_cask": info.brew_cask,
            "brew_zap": info.brew_zap.iter().map(associated_json).collect::<Vec<_>>(),
        }))
    })
}

/// Find leftovers of apps that are no longer installed.
#[no_mangle]
pub extern "C" fn tidymac_orphans_scan() -> *mut c_char {
    ffi_safe!({
        let groups = crate::apps::find_orphans();
        let response: Vec<_> = groups
            .iter()
            .map(|g| {
                serde_json::json!({
                    "bundle_id": g.bundle_id,
                    "total_size": g.total_size,
                    "total_size_formatted": format::format_size(g.total_size),
                    "files": g.files.iter().map(associated_json).collect::<Vec<_>>(),
                })
            })
            .collect();
        json_to_c(&response)
    })
}

/// Remove selected orphan files. `request_json`: `{"paths": [...], "mode": "soft"}`.
/// Each path must still be an orphan (re-checked now).
#[no_mangle]
pub extern "C" fn tidymac_orphans_clean(request_json: *const c_char) -> *mut c_char {
    ffi_safe!({
        let req = match json_arg(request_json) {
            Some(v) => v,
            None => return error_c("Invalid request"),
        };
        let wanted: Vec<PathBuf> = strings(&req, "paths").into_iter().map(PathBuf::from).collect();
        if wanted.is_empty() {
            return error_c("Nothing selected");
        }
        let current = crate::apps::find_orphans();
        let library = dirs::home_dir().unwrap_or_default().join("Library");
        let mut requests = Vec::new();
        for g in &current {
            for f in &g.files {
                if wanted.contains(&f.path) {
                    requests.push(DeleteRequest {
                        path: f.path.clone(),
                        size_bytes: f.size,
                        category: format!("Orphan: {}", g.bundle_id),
                        safety: "Caution".into(),
                        roots: vec![library.clone()],
                        policy: DeletePolicy::AppData,
                    });
                }
            }
        }
        let opts = DeleteOptions::new(mode_from(req.get("mode").and_then(|m| m.as_str())), "orphans");
        match crate::cleaner::execute(requests, Vec::new(), &opts) {
            Ok(r) => json_to_c(&clean_report_json(&r)),
            Err(e) => error_c(&e.to_string()),
        }
    })
}

// ─── Duplicates ──────────────────────────────────────────────────────────────

/// Find duplicates. `request_json`: `{"path": "...", "perceptual": false, "threshold": 0.9, "min_size": 1024}`.
#[no_mangle]
pub extern "C" fn tidymac_duplicates(request_json: *const c_char) -> *mut c_char {
    ffi_safe!({
        CANCEL_FLAG.store(false, Ordering::Relaxed);
        let req = json_arg(request_json).unwrap_or(serde_json::json!({}));
        let root = req
            .get("path")
            .and_then(|p| p.as_str())
            .map(PathBuf::from)
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_default());
        if !root.is_dir() {
            return error_c(&format!("Not a folder: {}", root.display()));
        }
        let config = crate::duplicates::DupConfig {
            root,
            min_size: req.get("min_size").and_then(|v| v.as_u64()).unwrap_or(1024),
            perceptual: req.get("perceptual").and_then(|v| v.as_bool()).unwrap_or(false),
            threshold: req.get("threshold").and_then(|v| v.as_f64()).unwrap_or(0.9),
            show_progress: false,
        };
        match crate::duplicates::find_duplicates(&config) {
            Ok(r) => json_to_c(&crate::duplicates::results_to_json(&r)),
            Err(e) => error_c(&e.to_string()),
        }
    })
}

// ─── Privacy ─────────────────────────────────────────────────────────────────

/// Run privacy audit. Returns JSON string.
#[no_mangle]
pub extern "C" fn tidymac_privacy_scan() -> *mut c_char {
    ffi_safe!({
        let report = crate::privacy::run_privacy_audit(true, true);
        let running = crate::common::running::RunningApps::snapshot();
        let response = serde_json::json!({
            "browser_profiles": report.browser_profiles.iter().map(|p| {
                serde_json::json!({
                    "browser": format!("{}", p.browser),
                    "running": running.is_bundle_running(p.browser.bundle_id()),
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
    })
}

/// Clean privacy data. `request_json`:
/// `{"mode": "soft", "caches": true, "browser_cookies": false, "app_cookies": false}`.
#[no_mangle]
pub extern "C" fn tidymac_privacy_clean(request_json: *const c_char) -> *mut c_char {
    ffi_safe!({
        let req = json_arg(request_json).unwrap_or(serde_json::json!({}));
        let b = |k: &str, d: bool| req.get(k).and_then(|v| v.as_bool()).unwrap_or(d);
        let opts = crate::privacy::PrivacyCleanOptions {
            mode: mode_from(req.get("mode").and_then(|m| m.as_str())),
            caches: b("caches", true),
            browser_cookies: b("browser_cookies", false),
            app_cookies: b("app_cookies", false),
        };
        let report = crate::privacy::run_privacy_audit(true, true);
        match crate::privacy::clean_privacy(&report, &opts) {
            Ok(r) => json_to_c(&clean_report_json(&r)),
            Err(e) => error_c(&e.to_string()),
        }
    })
}

// ─── Docker ──────────────────────────────────────────────────────────────────

/// Get Docker usage. Returns JSON string.
#[no_mangle]
pub extern "C" fn tidymac_docker_usage() -> *mut c_char {
    ffi_safe!({
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
    })
}

// ─── Undo / purge ────────────────────────────────────────────────────────────

/// List undo sessions. Returns JSON string.
#[no_mangle]
pub extern "C" fn tidymac_undo_list() -> *mut c_char {
    ffi_safe!({
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
                    "staged_size": s.staged_size,
                    "staged_size_formatted": format::format_size(s.staged_size),
                    "expires_at": s.expires_at.map(|e| e.to_rfc3339()),
                    "restored": s.restored,
                    "is_expired": s.is_expired,
                })
            })
            .collect();
        json_to_c(&response)
    })
}

/// Restore a session by ID. Returns JSON string.
#[no_mangle]
pub extern "C" fn tidymac_undo_session(session_id: *const c_char) -> *mut c_char {
    ffi_safe!({
        let session_id = match arg(session_id) {
            Some(s) => s,
            None => return error_c("session_id is required"),
        };
        let report = match crate::cleaner::restore_session(&session_id, false) {
            Ok(r) => r,
            Err(e) => return error_c(&format!("Restore failed: {}", e)),
        };
        json_to_c(&serde_json::json!({
            "session_id": report.session_id,
            "restored_count": report.restored_count,
            "restored_bytes": report.restored_bytes,
            "restored_bytes_formatted": format::format_size(report.restored_bytes),
            "errors": report.errors,
        }))
    })
}

/// Permanently free staged space. `which`: "expired" (default) or "all".
#[no_mangle]
pub extern "C" fn tidymac_purge(which: *const c_char) -> *mut c_char {
    ffi_safe!({
        let all = arg(which).as_deref() == Some("all");
        let result = if all {
            crate::cleaner::purge_all()
        } else {
            crate::cleaner::purge_expired()
        };
        match result {
            Ok(r) => json_to_c(&serde_json::json!({
                "purged_sessions": r.purged_sessions.len(),
                "bytes_freed": r.total_bytes_freed,
                "bytes_freed_formatted": format::format_size(r.total_bytes_freed),
                "errors": r.errors,
            })),
            Err(e) => error_c(&e.to_string()),
        }
    })
}

// ─── Startup ─────────────────────────────────────────────────────────────────

/// List all startup items. Returns JSON string.
#[no_mangle]
pub extern "C" fn tidymac_startup_list() -> *mut c_char {
    ffi_safe!({
        let items = crate::startup::discover_startup_items();
        let response: Vec<_> = items
            .iter()
            .map(|item| {
                serde_json::json!({
                    "label": item.label,
                    "kind": format!("{}", item.kind),
                    "program": item.program,
                    "enabled": item.enabled,
                    "file_path": item.path.display().to_string(),
                })
            })
            .collect();
        json_to_c(&response)
    })
}

/// Toggle a startup item's enabled state. `label` must match exactly.
#[no_mangle]
pub extern "C" fn tidymac_startup_toggle(label: *const c_char, enable: bool) -> *mut c_char {
    ffi_safe!({
        let label_str = match arg(label) {
            Some(l) => l,
            None => return error_c("label is required"),
        };
        let items = crate::startup::discover_startup_items();
        let item = match crate::startup::find_item_by_label(&items, &label_str) {
            Some(i) => i,
            None => return error_c(&format!("No startup item with label '{}'", label_str)),
        };
        let result = if enable {
            crate::startup::enable_item(item)
        } else {
            crate::startup::disable_item(item)
        };
        match result {
            Ok(msg) => json_to_c(&serde_json::json!({ "success": true, "message": msg })),
            Err(e) => error_c(&e.to_string()),
        }
    })
}

// ─── Profiles ────────────────────────────────────────────────────────────────

/// List available profiles. Returns JSON string.
#[no_mangle]
pub extern "C" fn tidymac_profiles_list() -> *mut c_char {
    ffi_safe!({
        let profiles = Profile::available_profiles();
        let response: Vec<_> = profiles
            .iter()
            .map(|name| {
                let info = Profile::load(name).ok().map(|p| {
                    serde_json::json!({
                        "name": name,
                        "description": p.profile.description,
                        "aggression": format!("{:?}", p.profile.aggression),
                        "rules": p.rules.len(),
                        "rule_problems": p.rule_problems(),
                    })
                });
                info.unwrap_or_else(|| serde_json::json!({ "name": name }))
            })
            .collect();
        json_to_c(&response)
    })
}

// ─── Schedule ────────────────────────────────────────────────────────────────

/// Current scheduled report (or `{"installed": false}`), plus the latest report.
#[no_mangle]
pub extern "C" fn tidymac_schedule_status() -> *mut c_char {
    ffi_safe!({
        let status = crate::schedule::status();
        json_to_c(&serde_json::json!({
            "installed": status.is_some(),
            "profile": status.as_ref().map(|s| s.profile.clone()),
            "interval": status.as_ref().map(|s| s.interval.to_string()),
            "latest_report": crate::schedule::latest_report(),
        }))
    })
}

/// Install a scheduled dry-run report.
/// `request_json`: `{"profile": "developer", "interval": "weekly", "binary": "/path/to/tidymac"}`.
#[no_mangle]
pub extern "C" fn tidymac_schedule_install(request_json: *const c_char) -> *mut c_char {
    ffi_safe!({
        let req = match json_arg(request_json) {
            Some(v) => v,
            None => return error_c("Invalid request"),
        };
        let profile = req.get("profile").and_then(|p| p.as_str()).unwrap_or("quick");
        let interval = match crate::schedule::Interval::parse(
            req.get("interval").and_then(|i| i.as_str()).unwrap_or("weekly"),
        ) {
            Some(i) => i,
            None => return error_c("interval must be daily, weekly or monthly"),
        };
        let binary = req.get("binary").and_then(|b| b.as_str()).map(PathBuf::from);
        match crate::schedule::install(profile, interval, binary) {
            Ok(s) => json_to_c(&serde_json::json!({
                "installed": true,
                "profile": s.profile,
                "interval": s.interval.to_string(),
            })),
            Err(e) => error_c(&e.to_string()),
        }
    })
}

/// Remove the scheduled report.
#[no_mangle]
pub extern "C" fn tidymac_schedule_remove() -> *mut c_char {
    ffi_safe!({
        match crate::schedule::remove() {
            Ok(existed) => json_to_c(&serde_json::json!({ "removed": existed })),
            Err(e) => error_c(&e.to_string()),
        }
    })
}

// ─── Version / cancel ────────────────────────────────────────────────────────

/// Get TidyMac version. Returns a C string (not JSON).
#[no_mangle]
pub extern "C" fn tidymac_version() -> *mut c_char {
    ffi_safe!({ to_c_string(env!("CARGO_PKG_VERSION")) })
}

/// Signal the current scan to cancel.
#[no_mangle]
pub extern "C" fn tidymac_cancel_scan() {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        CANCEL_FLAG.store(true, Ordering::SeqCst);
    }));
}
