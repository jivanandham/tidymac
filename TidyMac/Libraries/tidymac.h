// TidyMac FFI C Header
// Auto-generated interface for Swift interop

#ifndef TIDYMAC_H
#define TIDYMAC_H

#include <stdbool.h>
#include <stdint.h>

// Memory management - caller must free all returned strings
void tidymac_free_string(char *ptr);

// Scan with a profile. Returns JSON string.
// profile_name: "quick", "developer", "creative", "deep"
char *tidymac_scan(const char *profile_name);

// Get disk usage breakdown. Returns JSON string.
char *tidymac_disk_usage(void);

// List installed applications. Returns JSON string.
char *tidymac_apps_list(void);

// Clean leftovers (caches, logs, etc.) for a specific app. Does NOT remove the
// app itself.
char *tidymac_app_clean_leftovers(const char *app_name);

// Run a clean operation on selected items. Returns JSON string.
// mode: "dry_run", "soft", "hard"
// selected_names_json: JSON array of item names to clean, e.g. '["npm
// Cache","pip Cache"]'
//                      Pass NULL to clean ALL items.
char *tidymac_clean(const char *profile_name, const char *mode,
                    const char *selected_names_json);

// Run privacy audit. Returns JSON string.
char *tidymac_privacy_scan(void);

// Get Docker usage. Returns JSON string.
char *tidymac_docker_usage(void);

// List undo sessions. Returns JSON string.
char *tidymac_undo_list(void);

// Restore a session by ID. Returns JSON string.
char *tidymac_undo_session(const char *session_id);

// List available profiles. Returns JSON string.
char *tidymac_profiles_list(void);

// Get version string.
char *tidymac_version(void);

// Signal the current scan to cancel.
void tidymac_cancel_scan(void);

// Initialize logging and crash reporting.
// sentry_dsn: Optional Sentry DSN string.
void tidymac_init_observability(bool verbose, const char *sentry_dsn);

// List startup items. Returns JSON string.
char *tidymac_startup_list(void);

// Toggle a startup item's enabled state. Returns JSON string
// {"success":true,"message":"..."}
char *tidymac_startup_toggle(const char *label, bool enable);

#endif // TIDYMAC_H
