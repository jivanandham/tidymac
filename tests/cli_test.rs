use assert_cmd::Command;
use predicates::prelude::*;

fn tidymac() -> Command {
    Command::cargo_bin("tidymac").unwrap()
}

// ─── Help & version ──────────────────────────────────────────────────────────

#[test]
fn test_help_flag() {
    tidymac()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("developer caches"))
        .stdout(predicate::str::contains("scan"))
        .stdout(predicate::str::contains("clean"))
        .stdout(predicate::str::contains("dup"))
        .stdout(predicate::str::contains("apps"))
        .stdout(predicate::str::contains("startup"))
        .stdout(predicate::str::contains("privacy"))
        .stdout(predicate::str::contains("viz"))
        .stdout(predicate::str::contains("undo"))
        .stdout(predicate::str::contains("purge"));
}

#[test]
fn test_version_flag() {
    tidymac()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("tidymac"));
}

// ─── Scan command ────────────────────────────────────────────────────────────

#[test]
fn test_scan_quiet_mode() {
    tidymac()
        .args(["scan", "--quiet"])
        .assert()
        .success();
}

#[test]
fn test_scan_json_output() {
    tidymac()
        .args(["scan", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("total_reclaimable"));
}

#[test]
fn test_scan_with_profile() {
    tidymac()
        .args(["scan", "--profile", "quick"])
        .assert()
        .success();
}

#[test]
fn test_scan_invalid_profile() {
    tidymac()
        .args(["scan", "--profile", "nonexistent_xyz"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

// ─── Config command ──────────────────────────────────────────────────────────

#[test]
fn test_config_show() {
    tidymac()
        .args(["config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("staging_retention_days"));
}

// ─── Status command ──────────────────────────────────────────────────────────

#[test]
fn test_status() {
    tidymac()
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("TidyMac Status"));
}

// ─── Undo command ────────────────────────────────────────────────────────────

#[test]
fn test_undo_list() {
    tidymac()
        .args(["undo", "--list"])
        .assert()
        .success();
}

#[test]
fn test_undo_no_flags_shows_help() {
    tidymac()
        .arg("undo")
        .assert()
        .success()
        .stdout(predicate::str::contains("Undo"));
}

// ─── Purge command ───────────────────────────────────────────────────────────

#[test]
fn test_purge_no_flags_shows_help() {
    tidymac()
        .arg("purge")
        .assert()
        .success()
        .stdout(predicate::str::contains("Purge"));
}

#[test]
fn test_purge_expired() {
    tidymac()
        .args(["purge", "--expired"])
        .assert()
        .success();
}

// ─── Apps command ────────────────────────────────────────────────────────────

#[test]
fn test_apps_list() {
    tidymac()
        .args(["apps", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Installed Applications"));
}

#[test]
fn test_apps_list_json() {
    tidymac()
        .args(["apps", "list", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("["));
}

#[test]
fn test_apps_info_nonexistent() {
    tidymac()
        .args(["apps", "info", "ThisAppDoesNotExist12345"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No app found"));
}

// ─── Startup command ─────────────────────────────────────────────────────────

#[test]
fn test_startup_list() {
    tidymac()
        .args(["startup", "list"])
        .assert()
        .success();
}

// ─── Privacy command ─────────────────────────────────────────────────────────

#[test]
fn test_privacy_scan() {
    tidymac()
        .args(["privacy", "scan"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Privacy"));
}

#[test]
fn test_privacy_scan_json() {
    tidymac()
        .args(["privacy", "scan", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("browser_profiles"));
}

// ─── Viz command ─────────────────────────────────────────────────────────────

#[test]
fn test_viz() {
    tidymac()
        .arg("viz")
        .assert()
        .success()
        .stdout(predicate::str::contains("Storage"));
}

#[test]
fn test_viz_json() {
    tidymac()
        .args(["viz", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("total_capacity"));
}

// ─── Clean command (dry-run only in tests) ───────────────────────────────────

#[test]
fn test_clean_dry_run() {
    tidymac()
        .args(["clean", "--dry-run", "--profile", "quick"])
        .assert()
        .success();
}

// ─── Dup command ─────────────────────────────────────────────────────────────

#[test]
fn test_dup_nonexistent_path() {
    tidymac()
        .args(["dup", "/nonexistent/path/xyz123"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not exist"));
}

// ─── Invalid commands ────────────────────────────────────────────────────────

#[test]
fn test_no_subcommand_shows_help() {
    tidymac()
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage"));
}
