import Foundation

// MARK: - FFI Function Declarations

// These map to the C functions exported by the Rust library
@_silgen_name("tidymac_free_string")
func tidymac_free_string(_ ptr: UnsafeMutablePointer<CChar>?)

@_silgen_name("tidymac_startup_list")
func tidymac_startup_list() -> UnsafeMutablePointer<CChar>?

@_silgen_name("tidymac_startup_toggle")
func tidymac_startup_toggle(_ label: UnsafePointer<CChar>?, _ enable: Bool) -> UnsafeMutablePointer<CChar>?

@_silgen_name("tidymac_scan")
func tidymac_scan(_ profile: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?

@_silgen_name("tidymac_disk_usage")
func tidymac_disk_usage() -> UnsafeMutablePointer<CChar>?

@_silgen_name("tidymac_apps_list")
func tidymac_apps_list() -> UnsafeMutablePointer<CChar>?

@_silgen_name("tidymac_app_clean_leftovers")
func tidymac_app_clean_leftovers(_ appIdentifier: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?

@_silgen_name("tidymac_clean")
func tidymac_clean(_ profile: UnsafePointer<CChar>?, _ mode: UnsafePointer<CChar>?, _ selectionJson: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?

@_silgen_name("tidymac_privacy_scan")
func tidymac_privacy_scan() -> UnsafeMutablePointer<CChar>?

@_silgen_name("tidymac_docker_usage")
func tidymac_docker_usage() -> UnsafeMutablePointer<CChar>?

@_silgen_name("tidymac_undo_list")
func tidymac_undo_list() -> UnsafeMutablePointer<CChar>?

@_silgen_name("tidymac_undo_session")
func tidymac_undo_session(_ sessionId: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?

@_silgen_name("tidymac_profiles_list")
func tidymac_profiles_list() -> UnsafeMutablePointer<CChar>?

@_silgen_name("tidymac_version")
func tidymac_version() -> UnsafeMutablePointer<CChar>?

@_silgen_name("tidymac_cancel_scan")
func tidymac_cancel_scan()

@_silgen_name("tidymac_init_observability")
func tidymac_init_observability(_ verbose: Bool)

@_silgen_name("tidymac_has_full_disk_access")
func tidymac_has_full_disk_access() -> Bool

@_silgen_name("tidymac_delete_paths")
func tidymac_delete_paths(_ requestJson: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?

@_silgen_name("tidymac_disk_tree")
func tidymac_disk_tree(_ path: UnsafePointer<CChar>?, _ maxDepth: UInt32, _ maxChildren: UInt32) -> UnsafeMutablePointer<CChar>?

@_silgen_name("tidymac_system_data")
func tidymac_system_data() -> UnsafeMutablePointer<CChar>?

@_silgen_name("tidymac_app_uninstall")
func tidymac_app_uninstall(_ requestJson: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?

@_silgen_name("tidymac_app_install_info")
func tidymac_app_install_info(_ appPath: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?

@_silgen_name("tidymac_orphans_scan")
func tidymac_orphans_scan() -> UnsafeMutablePointer<CChar>?

@_silgen_name("tidymac_orphans_clean")
func tidymac_orphans_clean(_ requestJson: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?

@_silgen_name("tidymac_duplicates")
func tidymac_duplicates(_ requestJson: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?

@_silgen_name("tidymac_privacy_clean")
func tidymac_privacy_clean(_ requestJson: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?

@_silgen_name("tidymac_purge")
func tidymac_purge(_ which: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?

@_silgen_name("tidymac_schedule_status")
func tidymac_schedule_status() -> UnsafeMutablePointer<CChar>?

@_silgen_name("tidymac_schedule_install")
func tidymac_schedule_install(_ requestJson: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?

@_silgen_name("tidymac_schedule_remove")
func tidymac_schedule_remove() -> UnsafeMutablePointer<CChar>?

// MARK: - Swift Bridge

class TidyMacBridge {
    static let shared = TidyMacBridge()

    private init() {}

    // MARK: - Helper

    private func callFFI(_ ptr: UnsafeMutablePointer<CChar>?) -> [String: Any]? {
        guard let ptr = ptr else { return nil }
        let str = String(cString: ptr)
        tidymac_free_string(ptr)
        guard let data = str.data(using: .utf8) else { return nil }
        return try? JSONSerialization.jsonObject(with: data) as? [String: Any]
    }

    private func callFFIArray(_ ptr: UnsafeMutablePointer<CChar>?) -> [[String: Any]]? {
        guard let ptr = ptr else { return nil }
        let str = String(cString: ptr)
        tidymac_free_string(ptr)
        guard let data = str.data(using: .utf8) else { return nil }
        return try? JSONSerialization.jsonObject(with: data) as? [[String: Any]]
    }

    private func callFFIString(_ ptr: UnsafeMutablePointer<CChar>?) -> String? {
        guard let ptr = ptr else { return nil }
        let str = String(cString: ptr)
        tidymac_free_string(ptr)
        return str
    }

    // MARK: - Public API

    /// Run a fresh scan. Returns the result or an error message ("CANCELLED" if cancelled).
    func scan(profile: String) -> (ScanResult?, String?) {
        let result = profile.withCString { tidymac_scan($0) }
        guard let dict = callFFI(result) else { return (nil, "No response from the scanner") }
        if let error = dict["error"] as? String { return (nil, error) }
        return (ScanResult(from: dict), nil)
    }

    func diskUsage() -> DiskUsageResult? {
        let result = tidymac_disk_usage()
        guard let dict = callFFI(result) else { return nil }
        return DiskUsageResult(from: dict)
    }

    func appsList() -> [AppInfo] {
        let result = tidymac_apps_list()
        guard let arr = callFFIArray(result) else { return [] }
        return arr.compactMap { AppInfo(from: $0) }
    }

    /// Clear regenerable data (caches, logs…) of an installed app. `identifier` is its bundle id or path.
    func appCleanLeftovers(identifier: String) -> AppCleanResult? {
        let result = identifier.withCString { tidymac_app_clean_leftovers($0) }
        guard let dict = callFFI(result) else { return nil }
        return AppCleanResult(from: dict)
    }

    /// Clean items of the scan identified by `scanId`. Only entries from that scan
    /// are touched; `excludePaths` are entries the user unticked.
    func clean(profile: String, mode: String, scanId: String, itemIds: [Int], excludePaths: [String]) -> CleanResult? {
        let selection: [String: Any] = ["scan_id": scanId, "items": itemIds, "exclude_paths": excludePaths]
        guard let json = jsonString(selection) else { return nil }
        let result = profile.withCString { p in
            mode.withCString { m in
                json.withCString { s in tidymac_clean(p, m, s) }
            }
        }
        guard let dict = callFFI(result) else { return nil }
        return CleanResult(from: dict)
    }

    /// Remove individually chosen files (duplicates, large files). Soft delete by default.
    func deletePaths(_ paths: [String], mode: String = "soft", label: String) -> CleanResult? {
        guard let dict = callJSON(["paths": paths, "mode": mode, "label": label], tidymac_delete_paths) else { return nil }
        return CleanResult(from: dict)
    }

    func hasFullDiskAccess() -> Bool {
        tidymac_has_full_disk_access()
    }

    // MARK: - Disk map / System Data

    func diskTree(path: String?, depth: Int = 3, maxChildren: Int = 30) -> DiskNode? {
        let result: UnsafeMutablePointer<CChar>?
        if let path = path {
            result = path.withCString { tidymac_disk_tree($0, UInt32(depth), UInt32(maxChildren)) }
        } else {
            result = tidymac_disk_tree(nil, UInt32(depth), UInt32(maxChildren))
        }
        guard let dict = callFFI(result), dict["error"] == nil else { return nil }
        return DiskNode(from: dict)
    }

    func systemData() -> SystemDataInfo? {
        guard let dict = callFFI(tidymac_system_data()) else { return nil }
        return SystemDataInfo(from: dict)
    }

    // MARK: - Apps

    func uninstallApp(identifier: String, mode: String, includeNameMatches: Bool, onlyPaths: [String]) -> AppCleanResult? {
        let req: [String: Any] = [
            "app": identifier,
            "mode": mode,
            "include_name_matches": includeNameMatches,
            "include_brew_zap": true,
            "only_paths": onlyPaths,
        ]
        guard let dict = callJSON(req, tidymac_app_uninstall) else { return nil }
        return AppCleanResult(from: dict)
    }

    func appInstallInfo(path: String) -> InstallInfoResult? {
        let result = path.withCString { tidymac_app_install_info($0) }
        guard let dict = callFFI(result) else { return nil }
        return InstallInfoResult(from: dict)
    }

    func orphans() -> [OrphanGroupInfo] {
        guard let arr = callFFIArray(tidymac_orphans_scan()) else { return [] }
        return arr.compactMap { OrphanGroupInfo(from: $0) }
    }

    func cleanOrphans(paths: [String], mode: String = "soft") -> CleanResult? {
        guard let dict = callJSON(["paths": paths, "mode": mode], tidymac_orphans_clean) else { return nil }
        return CleanResult(from: dict)
    }

    // MARK: - Duplicates

    func duplicates(path: String, perceptual: Bool, threshold: Double = 0.9, minSize: Int = 1024) -> DupResult? {
        let req: [String: Any] = ["path": path, "perceptual": perceptual, "threshold": threshold, "min_size": minSize]
        guard let dict = callJSON(req, tidymac_duplicates) else { return nil }
        return DupResult(from: dict)
    }

    // MARK: - Privacy / purge / schedule

    func privacyClean(caches: Bool, browserCookies: Bool, appCookies: Bool, mode: String = "soft") -> CleanResult? {
        let req: [String: Any] = ["caches": caches, "browser_cookies": browserCookies, "app_cookies": appCookies, "mode": mode]
        guard let dict = callJSON(req, tidymac_privacy_clean) else { return nil }
        return CleanResult(from: dict)
    }

    func purge(all: Bool) -> [String: Any]? {
        let which = all ? "all" : "expired"
        return callFFI(which.withCString { tidymac_purge($0) })
    }

    func scheduleStatus() -> ScheduleInfo? {
        guard let dict = callFFI(tidymac_schedule_status()) else { return nil }
        return ScheduleInfo(from: dict)
    }

    /// Install a scheduled report using the CLI bundled in the app (Contents/Resources/bin/tidymac).
    func installSchedule(profile: String, interval: String) -> String? {
        var req: [String: Any] = ["profile": profile, "interval": interval]
        if let cli = Bundle.main.url(forResource: "tidymac", withExtension: nil, subdirectory: "bin") {
            req["binary"] = cli.path
        }
        guard let dict = callJSON(req, tidymac_schedule_install) else { return "Failed to install schedule" }
        return dict["error"] as? String
    }

    func removeSchedule() {
        _ = callFFI(tidymac_schedule_remove())
    }

    // MARK: - JSON helpers

    private func jsonString(_ obj: Any) -> String? {
        guard let data = try? JSONSerialization.data(withJSONObject: obj) else { return nil }
        return String(data: data, encoding: .utf8)
    }

    private func callJSON(_ obj: [String: Any], _ fn: (UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?) -> [String: Any]? {
        guard let json = jsonString(obj) else { return nil }
        let result = json.withCString { fn($0) }
        return callFFI(result)
    }

    func privacyScan() -> PrivacyResult? {
        let result = tidymac_privacy_scan()
        guard let dict = callFFI(result) else { return nil }
        return PrivacyResult(from: dict)
    }

    func dockerUsage() -> DockerResult? {
        let result = tidymac_docker_usage()
        guard let dict = callFFI(result) else { return nil }
        return DockerResult(from: dict)
    }

    func undoList() -> [UndoSession] {
        let result = tidymac_undo_list()
        guard let arr = callFFIArray(result) else { return [] }
        return arr.compactMap { UndoSession(from: $0) }
    }

    func undoSession(id: String) -> UndoResult? {
        let result = id.withCString { tidymac_undo_session($0) }
        guard let dict = callFFI(result) else { return nil }
        return UndoResult(from: dict)
    }

    func profilesList() -> [ProfileInfo] {
        let result = tidymac_profiles_list()
        guard let arr = callFFIArray(result) else { return [] }
        return arr.compactMap { ProfileInfo(from: $0) }
    }

    func version() -> String {
        callFFIString(tidymac_version()) ?? "unknown"
    }

    func cancelScan() {
        tidymac_cancel_scan()
    }

    func initObservability(verbose: Bool) {
        tidymac_init_observability(verbose)
    }

    // MARK: - Startup
    func startupList() -> [StartupItemInfo] {
        guard let resultCStr = tidymac_startup_list() else { return [] }
        defer { tidymac_free_string(resultCStr) }
        
        let jsonStr = String(cString: resultCStr)
        guard let data = jsonStr.data(using: .utf8) else { return [] }
        
        do {
            return try JSONDecoder().decode([StartupItemInfo].self, from: data)
        } catch {
            print("Failed to decode startup list: \(error) - JSON: \(jsonStr)")
            return []
        }
    }

    func toggleStartupItem(label: String, enable: Bool) -> Bool {
        let result = label.withCString { tidymac_startup_toggle($0, enable) }
        guard let dict = callFFI(result) else { return false }
        return dict["success"] as? Bool ?? false
    }
}

// MARK: - Startup Data Model
struct StartupItemInfo: Codable, Identifiable {
    var id: String { return label }
    var label: String
    var kind: String
    var program: String?
    var enabled: Bool
    var file_path: String
}

// MARK: - Data Models

struct ScanResult {
    let scanId: String
    let profile: String
    let durationSecs: Double
    let totalReclaimable: UInt64
    let totalReclaimableFormatted: String
    let totalFiles: Int
    let items: [ScanItem]
    let errors: [String]

    init?(from dict: [String: Any]) {
        guard let profile = dict["profile"] as? String else { return nil }
        self.scanId = dict["scan_id"] as? String ?? ""
        self.profile = profile
        self.durationSecs = dict["duration_secs"] as? Double ?? 0
        self.totalReclaimable = dict["total_reclaimable"] as? UInt64 ?? 0
        self.totalReclaimableFormatted = dict["total_reclaimable_formatted"] as? String ?? "0 B"
        self.totalFiles = dict["total_files"] as? Int ?? 0
        self.errors = dict["errors"] as? [String] ?? []
        self.items = (dict["items"] as? [[String: Any]] ?? []).compactMap { ScanItem(from: $0) }
    }
}

struct ScanEntry: Identifiable, Hashable {
    var id: String { path }
    let path: String
    let sizeBytes: UInt64
    let sizeFormatted: String
    let modified: String?

    init?(from dict: [String: Any]) {
        guard let path = dict["path"] as? String else { return nil }
        self.path = path
        self.sizeBytes = dict["size_bytes"] as? UInt64 ?? 0
        self.sizeFormatted = dict["size_formatted"] as? String ?? "0 B"
        self.modified = dict["modified"] as? String
    }

    var name: String { (path as NSString).lastPathComponent }
}

struct ScanItem: Identifiable {
    /// Index of the item in its scan (stable for that scan_id).
    let id: Int
    let name: String
    let category: String
    let path: String
    let sizeBytes: UInt64
    let sizeFormatted: String
    let reclaimableBytes: UInt64
    let reclaimableFormatted: String
    let fileCount: Int
    let entryCount: Int
    let safety: String
    let reason: String
    let reviewOnly: Bool
    let cleanable: Bool
    let toolAction: String?
    let entries: [ScanEntry]

    var safetyColor: String {
        switch safety {
        case "Safe": return "green"
        case "Caution": return "orange"
        case "Dangerous": return "red"
        default: return "gray"
        }
    }

    var icon: String {
        if category.contains("Dev:") { return "wrench.and.screwdriver" }
        if category.contains("Cache") { return "folder" }
        if category.contains("Log") { return "doc.text" }
        if category.contains("Temp") { return "trash" }
        if category.contains("Crash") { return "exclamationmark.triangle" }
        if category.contains("Large") { return "doc.zipper" }
        return "folder"
    }

    /// True when APFS clones make the real savings noticeably smaller than the size.
    var hasSharedBlocks: Bool { reclaimableBytes + 1_048_576 < sizeBytes }

    init?(from dict: [String: Any]) {
        guard let name = dict["name"] as? String else { return nil }
        self.id = dict["id"] as? Int ?? -1
        self.name = name
        self.category = dict["category"] as? String ?? ""
        self.path = dict["path"] as? String ?? ""
        self.sizeBytes = dict["size_bytes"] as? UInt64 ?? 0
        self.sizeFormatted = dict["size_formatted"] as? String ?? "0 B"
        self.reclaimableBytes = dict["reclaimable_bytes"] as? UInt64 ?? self.sizeBytes
        self.reclaimableFormatted = dict["reclaimable_formatted"] as? String ?? self.sizeFormatted
        self.fileCount = dict["file_count"] as? Int ?? 0
        self.entryCount = dict["entry_count"] as? Int ?? 0
        self.safety = dict["safety"] as? String ?? "Caution"
        self.reason = dict["reason"] as? String ?? ""
        self.reviewOnly = dict["review_only"] as? Bool ?? false
        self.cleanable = dict["cleanable"] as? Bool ?? true
        self.toolAction = dict["tool_action"] as? String
        self.entries = (dict["entries"] as? [[String: Any]] ?? []).compactMap { ScanEntry(from: $0) }
    }
}

struct DiskUsageResult {
    let totalCapacity: UInt64
    let totalCapacityFormatted: String
    let used: UInt64
    let usedFormatted: String
    let available: UInt64
    let availableFormatted: String
    let usedPercentage: Int
    let categories: [DiskCategory]

    init?(from dict: [String: Any]) {
        self.totalCapacity = dict["total_capacity"] as? UInt64 ?? 0
        self.totalCapacityFormatted = dict["total_capacity_formatted"] as? String ?? "0 B"
        self.used = dict["used"] as? UInt64 ?? 0
        self.usedFormatted = dict["used_formatted"] as? String ?? "0 B"
        self.available = dict["available"] as? UInt64 ?? 0
        self.availableFormatted = dict["available_formatted"] as? String ?? "0 B"
        self.usedPercentage = dict["used_percentage"] as? Int ?? 0

        if let cats = dict["categories"] as? [[String: Any]] {
            self.categories = cats.compactMap { DiskCategory(from: $0) }
        } else {
            self.categories = []
        }
    }
}

struct DiskCategory: Identifiable {
    let id = UUID()
    let name: String
    let icon: String
    let path: String
    let size: UInt64
    let sizeFormatted: String

    init?(from dict: [String: Any]) {
        guard let name = dict["name"] as? String else { return nil }
        self.name = name
        self.icon = dict["icon"] as? String ?? "📁"
        self.path = dict["path"] as? String ?? ""
        self.size = dict["size"] as? UInt64 ?? 0
        self.sizeFormatted = dict["size_formatted"] as? String ?? "0 B"
    }
}

struct AppInfo: Identifiable, Hashable {
    static func == (lhs: AppInfo, rhs: AppInfo) -> Bool { lhs.id == rhs.id }
    func hash(into hasher: inout Hasher) { hasher.combine(id) }

    var id: String { path }
    let name: String
    let bundleId: String?
    let version: String?
    let path: String
    let isApple: Bool
    let appSize: UInt64
    let appSizeFormatted: String
    let totalSize: UInt64
    let totalSizeFormatted: String
    let leftoversSize: UInt64
    let leftoversFormatted: String
    let source: String
    let lastOpened: String?
    let associatedFiles: [AssociatedFileInfo]

    /// Identifier passed to the core: bundle id when known, else the bundle path.
    var identifier: String { bundleId ?? path }

    var regenerableSize: UInt64 {
        associatedFiles.filter { $0.regenerable }.reduce(0) { $0 + $1.size }
    }

    init?(from dict: [String: Any]) {
        guard let name = dict["name"] as? String else { return nil }
        self.name = name
        self.bundleId = dict["bundle_id"] as? String
        self.version = dict["version"] as? String
        self.path = dict["path"] as? String ?? ""
        self.isApple = dict["is_apple"] as? Bool ?? false
        self.appSize = dict["app_size"] as? UInt64 ?? 0
        self.appSizeFormatted = dict["app_size_formatted"] as? String ?? "0 B"
        self.totalSize = dict["total_size"] as? UInt64 ?? 0
        self.totalSizeFormatted = dict["total_size_formatted"] as? String ?? "0 B"
        self.leftoversSize = dict["leftovers_size"] as? UInt64 ?? 0
        self.leftoversFormatted = dict["leftovers_formatted"] as? String ?? "0 B"
        self.source = dict["source"] as? String ?? ""
        self.lastOpened = dict["last_opened"] as? String
        self.associatedFiles = (dict["associated_files"] as? [[String: Any]] ?? []).compactMap { AssociatedFileInfo(from: $0) }
    }
}

struct AssociatedFileInfo: Identifiable, Hashable {
    var id: String { path }
    let path: String
    let size: UInt64
    let sizeFormatted: String
    let kind: String
    let matchedBy: String
    let needsConfirmation: Bool
    let regenerable: Bool

    init?(from dict: [String: Any]) {
        self.path = dict["path"] as? String ?? ""
        self.size = dict["size"] as? UInt64 ?? 0
        self.sizeFormatted = dict["size_formatted"] as? String ?? "0 B"
        self.kind = dict["kind"] as? String ?? ""
        self.matchedBy = dict["matched_by"] as? String ?? ""
        self.needsConfirmation = dict["needs_confirmation"] as? Bool ?? false
        self.regenerable = dict["regenerable"] as? Bool ?? false
    }
}

struct SkippedInfo: Identifiable, Hashable {
    var id: String { path + reason }
    let path: String
    let reason: String
}

struct CleanResult {
    let error: String?
    let mode: String
    let filesRemoved: Int
    let bytesFreed: UInt64
    let bytesFreedFormatted: String
    let bytesStaged: UInt64
    let bytesStagedFormatted: String
    let sessionId: String?
    let expiresAt: String?
    let errors: [String]
    let warnings: [String]
    let toolActions: [String]
    let skipped: [SkippedInfo]

    var isSoftDelete: Bool { mode == "soft_delete" }

    init?(from dict: [String: Any]) {
        self.error = dict["error"] as? String
        self.mode = dict["mode"] as? String ?? ""
        self.filesRemoved = dict["files_removed"] as? Int ?? 0
        self.bytesFreed = dict["bytes_freed"] as? UInt64 ?? 0
        self.bytesFreedFormatted = dict["bytes_freed_formatted"] as? String ?? "0 B"
        self.bytesStaged = dict["bytes_staged"] as? UInt64 ?? 0
        self.bytesStagedFormatted = dict["bytes_staged_formatted"] as? String ?? "0 B"
        self.sessionId = dict["session_id"] as? String
        self.expiresAt = dict["expires_at"] as? String
        self.errors = dict["errors"] as? [String] ?? []
        self.warnings = dict["warnings"] as? [String] ?? []
        self.toolActions = dict["tool_actions"] as? [String] ?? []
        self.skipped = (dict["skipped"] as? [[String: Any]] ?? []).compactMap {
            guard let p = $0["path"] as? String else { return nil }
            return SkippedInfo(path: p, reason: $0["reason"] as? String ?? "")
        }
    }
}

struct PrivacyResult {
    let browserProfiles: [BrowserProfileInfo]
    let trackingApps: [TrackingAppInfo]
    let totalPrivacyDataSize: UInt64
    let totalPrivacyDataFormatted: String
    let cookieLocationsCount: Int

    init?(from dict: [String: Any]) {
        self.totalPrivacyDataSize = dict["total_privacy_data_size"] as? UInt64 ?? 0
        self.totalPrivacyDataFormatted = dict["total_privacy_data_formatted"] as? String ?? "0 B"
        self.cookieLocationsCount = dict["cookie_locations_count"] as? Int ?? 0

        if let profiles = dict["browser_profiles"] as? [[String: Any]] {
            self.browserProfiles = profiles.compactMap { BrowserProfileInfo(from: $0) }
        } else {
            self.browserProfiles = []
        }

        if let apps = dict["tracking_apps"] as? [[String: Any]] {
            self.trackingApps = apps.compactMap { TrackingAppInfo(from: $0) }
        } else {
            self.trackingApps = []
        }
    }
}

struct BrowserProfileInfo: Identifiable {
    let id = UUID()
    let browser: String
    let running: Bool
    let cookiesSize: UInt64
    let cookiesSizeFormatted: String
    let historySize: UInt64
    let cacheSize: UInt64
    let cacheSizeFormatted: String
    let totalSize: UInt64
    let totalSizeFormatted: String

    init?(from dict: [String: Any]) {
        guard let browser = dict["browser"] as? String else { return nil }
        self.browser = browser
        self.running = dict["running"] as? Bool ?? false
        self.cookiesSize = dict["cookies_size"] as? UInt64 ?? 0
        self.cookiesSizeFormatted = dict["cookies_size_formatted"] as? String ?? "0 B"
        self.historySize = dict["history_size"] as? UInt64 ?? 0
        self.cacheSize = dict["cache_size"] as? UInt64 ?? 0
        self.cacheSizeFormatted = dict["cache_size_formatted"] as? String ?? "0 B"
        self.totalSize = dict["total_size"] as? UInt64 ?? 0
        self.totalSizeFormatted = dict["total_size_formatted"] as? String ?? "0 B"
    }
}

struct TrackingAppInfo: Identifiable {
    let id = UUID()
    let name: String
    let kind: String
    let dataSize: UInt64
    let dataSizeFormatted: String

    init?(from dict: [String: Any]) {
        guard let name = dict["name"] as? String else { return nil }
        self.name = name
        self.kind = dict["kind"] as? String ?? ""
        self.dataSize = dict["data_size"] as? UInt64 ?? 0
        self.dataSizeFormatted = dict["data_size_formatted"] as? String ?? "0 B"
    }
}

struct DockerResult {
    let installed: Bool
    let running: Bool
    let totalSize: UInt64
    let totalSizeFormatted: String
    let reclaimable: UInt64
    let reclaimableFormatted: String

    init?(from dict: [String: Any]) {
        self.installed = dict["installed"] as? Bool ?? false
        self.running = dict["running"] as? Bool ?? false
        self.totalSize = dict["total_size"] as? UInt64 ?? 0
        self.totalSizeFormatted = dict["total_size_formatted"] as? String ?? "0 B"
        self.reclaimable = dict["reclaimable"] as? UInt64 ?? 0
        self.reclaimableFormatted = dict["reclaimable_formatted"] as? String ?? "0 B"
    }
}

struct UndoSession: Identifiable {
    let id = UUID()
    let sessionId: String
    let profile: String
    let timestamp: String
    let mode: String
    let totalFiles: Int
    let totalBytes: UInt64
    let totalBytesFormatted: String
    let restored: Bool
    let isExpired: Bool

    init?(from dict: [String: Any]) {
        guard let sid = dict["session_id"] as? String else { return nil }
        self.sessionId = sid
        self.profile = dict["profile"] as? String ?? ""
        self.timestamp = dict["timestamp"] as? String ?? ""
        self.mode = dict["mode"] as? String ?? ""
        self.totalFiles = dict["total_files"] as? Int ?? 0
        self.totalBytes = dict["total_bytes"] as? UInt64 ?? 0
        self.totalBytesFormatted = dict["total_bytes_formatted"] as? String ?? "0 B"
        self.restored = dict["restored"] as? Bool ?? false
        self.isExpired = dict["is_expired"] as? Bool ?? false
    }
}

struct UndoResult {
    let sessionId: String
    let restoredCount: Int
    let restoredBytes: UInt64
    let restoredBytesFormatted: String
    let errors: [String]

    init?(from dict: [String: Any]) {
        self.sessionId = dict["session_id"] as? String ?? ""
        self.restoredCount = dict["restored_count"] as? Int ?? 0
        self.restoredBytes = dict["restored_bytes"] as? UInt64 ?? 0
        self.restoredBytesFormatted = dict["restored_bytes_formatted"] as? String ?? "0 B"
        self.errors = dict["errors"] as? [String] ?? []
    }
}

struct ProfileInfo: Identifiable {
    let id = UUID()
    let name: String
    let description: String
    let aggression: String

    init?(from dict: [String: Any]) {
        guard let name = dict["name"] as? String else { return nil }
        self.name = name
        self.description = dict["description"] as? String ?? ""
        self.aggression = dict["aggression"] as? String ?? ""
    }
}

struct AppCleanResult {
    let error: String?
    let appName: String
    let filesRemoved: Int
    let bytesFreed: UInt64
    let bytesFreedFormatted: String
    let bytesStaged: UInt64
    let bytesStagedFormatted: String
    let sessionId: String?
    let removedPaths: [String]
    let skipped: [String]
    let errors: [String]

    init?(from dict: [String: Any]) {
        self.error = dict["error"] as? String
        self.appName = dict["app_name"] as? String ?? ""
        self.filesRemoved = dict["files_removed"] as? Int ?? 0
        self.bytesFreed = dict["bytes_freed"] as? UInt64 ?? 0
        self.bytesFreedFormatted = dict["bytes_freed_formatted"] as? String ?? "0 B"
        self.bytesStaged = dict["bytes_staged"] as? UInt64 ?? 0
        self.bytesStagedFormatted = dict["bytes_staged_formatted"] as? String ?? "0 B"
        self.sessionId = dict["session_id"] as? String
        self.removedPaths = dict["removed_paths"] as? [String] ?? []
        self.skipped = dict["skipped"] as? [String] ?? []
        self.errors = dict["errors"] as? [String] ?? []
    }
}

// MARK: - Install info / orphans

struct ReceiptInfo: Identifiable {
    var id: String { pkgId }
    let pkgId: String
    let version: String?
    let fileCount: Int
    let forgetCommand: String
}

struct InstallInfoResult {
    let receipts: [ReceiptInfo]
    let brewCask: String?
    let brewZap: [AssociatedFileInfo]

    init?(from dict: [String: Any]) {
        self.brewCask = dict["brew_cask"] as? String
        self.brewZap = (dict["brew_zap"] as? [[String: Any]] ?? []).compactMap { AssociatedFileInfo(from: $0) }
        self.receipts = (dict["receipts"] as? [[String: Any]] ?? []).compactMap {
            guard let id = $0["pkg_id"] as? String else { return nil }
            return ReceiptInfo(
                pkgId: id,
                version: $0["version"] as? String,
                fileCount: $0["file_count"] as? Int ?? 0,
                forgetCommand: $0["forget_command"] as? String ?? ""
            )
        }
    }
}

struct OrphanGroupInfo: Identifiable {
    var id: String { bundleId }
    let bundleId: String
    let totalSize: UInt64
    let totalSizeFormatted: String
    let files: [AssociatedFileInfo]

    init?(from dict: [String: Any]) {
        guard let id = dict["bundle_id"] as? String else { return nil }
        self.bundleId = id
        self.totalSize = dict["total_size"] as? UInt64 ?? 0
        self.totalSizeFormatted = dict["total_size_formatted"] as? String ?? "0 B"
        self.files = (dict["files"] as? [[String: Any]] ?? []).compactMap { AssociatedFileInfo(from: $0) }
    }
}

// MARK: - Duplicates

struct DupMember: Identifiable, Hashable {
    var id: String { path }
    let path: String
    let sizeBytes: UInt64
    let sizeFormatted: String
    let similarity: Double
}

struct DupGroup: Identifiable {
    let id = UUID()
    let matchType: String
    let wastedBytes: UInt64
    let wastedFormatted: String
    let members: [DupMember]

    init?(from dict: [String: Any]) {
        self.matchType = dict["match_type"] as? String ?? ""
        self.wastedBytes = dict["wasted_bytes"] as? UInt64 ?? 0
        self.wastedFormatted = dict["wasted_formatted"] as? String ?? "0 B"
        self.members = (dict["members"] as? [[String: Any]] ?? []).compactMap {
            guard let p = $0["path"] as? String else { return nil }
            return DupMember(
                path: p,
                sizeBytes: $0["size_bytes"] as? UInt64 ?? 0,
                sizeFormatted: $0["size_formatted"] as? String ?? "0 B",
                similarity: $0["similarity"] as? Double ?? 1
            )
        }
        if members.count < 2 { return nil }
    }
}

struct DupResult {
    let error: String?
    let filesScanned: Int
    let totalWasted: UInt64
    let totalWastedFormatted: String
    let exactGroups: [DupGroup]
    let similarGroups: [DupGroup]

    init?(from dict: [String: Any]) {
        self.error = dict["error"] as? String
        self.filesScanned = dict["files_scanned"] as? Int ?? 0
        self.totalWasted = dict["total_wasted"] as? UInt64 ?? 0
        self.totalWastedFormatted = dict["total_wasted_formatted"] as? String ?? "0 B"
        self.exactGroups = (dict["exact_groups"] as? [[String: Any]] ?? []).compactMap { DupGroup(from: $0) }
        self.similarGroups = (dict["similar_groups"] as? [[String: Any]] ?? []).compactMap { DupGroup(from: $0) }
    }
}

// MARK: - Disk map / System Data / Schedule

struct DiskNode: Identifiable {
    let id: String
    let name: String
    let path: String
    let size: UInt64
    let reclaimable: UInt64
    let files: Int
    let isDir: Bool
    let isOther: Bool
    let children: [DiskNode]

    init?(from dict: [String: Any]) {
        guard let name = dict["name"] as? String else { return nil }
        self.name = name
        self.path = dict["path"] as? String ?? ""
        self.size = dict["size"] as? UInt64 ?? 0
        self.reclaimable = dict["reclaimable"] as? UInt64 ?? self.size
        self.files = dict["files"] as? Int ?? 0
        self.isDir = dict["is_dir"] as? Bool ?? false
        self.isOther = dict["is_other"] as? Bool ?? false
        self.id = path + (isOther ? "#other" : "") + "|" + name
        self.children = (dict["children"] as? [[String: Any]] ?? []).compactMap { DiskNode(from: $0) }
    }
}

struct SystemDataEntryInfo: Identifiable {
    var id: String { name }
    let name: String
    let description: String
    let size: UInt64?
    let advice: String
    let tidymacCanClean: Bool
}

struct SystemDataInfo {
    let entries: [SystemDataEntryInfo]
    let notes: [String]
    let snapshotCount: Int

    init?(from dict: [String: Any]) {
        self.notes = dict["notes"] as? [String] ?? []
        self.snapshotCount = (dict["local_snapshots"] as? [String] ?? []).count
        self.entries = (dict["entries"] as? [[String: Any]] ?? []).compactMap {
            guard let n = $0["name"] as? String else { return nil }
            return SystemDataEntryInfo(
                name: n,
                description: $0["description"] as? String ?? "",
                size: $0["size"] as? UInt64,
                advice: $0["advice"] as? String ?? "",
                tidymacCanClean: $0["tidymac_can_clean"] as? Bool ?? false
            )
        }
    }
}

struct ReportInfo {
    let timestamp: String
    let profile: String
    let totalReclaimable: UInt64
}

struct ScheduleInfo {
    let installed: Bool
    let profile: String?
    let interval: String?
    let latestReport: ReportInfo?

    init?(from dict: [String: Any]) {
        self.installed = dict["installed"] as? Bool ?? false
        self.profile = dict["profile"] as? String
        self.interval = dict["interval"] as? String
        if let r = dict["latest_report"] as? [String: Any] {
            self.latestReport = ReportInfo(
                timestamp: r["timestamp"] as? String ?? "",
                profile: r["profile"] as? String ?? "",
                totalReclaimable: r["total_reclaimable"] as? UInt64 ?? 0
            )
        } else {
            self.latestReport = nil
        }
    }
}
