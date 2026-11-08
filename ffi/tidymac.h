#ifndef TIDYMAC_H
#define TIDYMAC_H

#include <stdbool.h>
#include <stdint.h>

// TidyMac C API. Every char* returned must be freed with tidymac_free_string().
// Unless noted, return values are JSON; failures are {"error": "..."}.

void tidymac_free_string(char *ptr);
void tidymac_init_observability(bool verbose);
bool tidymac_has_full_disk_access(void);
char *tidymac_version(void);                 // plain string, not JSON
void tidymac_cancel_scan(void);

// Scan & clean (clean acts on the last scan only; empty selection removes nothing)
char *tidymac_scan(const char *profile);
char *tidymac_clean(const char *profile, const char *mode, const char *selection_json);
char *tidymac_delete_paths(const char *request_json);

// Disk
char *tidymac_disk_usage(void);
char *tidymac_disk_tree(const char *path, uint32_t max_depth, uint32_t max_children);
char *tidymac_system_data(void);

// Apps
char *tidymac_apps_list(void);
char *tidymac_app_clean_leftovers(const char *app_identifier);
char *tidymac_app_uninstall(const char *request_json);
char *tidymac_app_install_info(const char *app_path);
char *tidymac_orphans_scan(void);
char *tidymac_orphans_clean(const char *request_json);

// Duplicates
char *tidymac_duplicates(const char *request_json);

// Privacy
char *tidymac_privacy_scan(void);
char *tidymac_privacy_clean(const char *request_json);

// Docker
char *tidymac_docker_usage(void);

// Undo
char *tidymac_undo_list(void);
char *tidymac_undo_session(const char *session_id);
char *tidymac_purge(const char *which);      // "expired" or "all"

// Startup items
char *tidymac_startup_list(void);
char *tidymac_startup_toggle(const char *label, bool enable);

// Profiles & schedule
char *tidymac_profiles_list(void);
char *tidymac_schedule_status(void);
char *tidymac_schedule_install(const char *request_json);
char *tidymac_schedule_remove(void);

#endif /* TIDYMAC_H */
