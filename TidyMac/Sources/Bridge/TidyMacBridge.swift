import Foundation

// MARK: - FFI Function Declarations

// These map to the C functions exported by the Rust library
@_silgen_name("tidymac_free_string")
func tidymac_free_string(_ ptr: UnsafeMutablePointer<CChar>?)

@_silgen_name("tidymac_scan")
func tidymac_scan(_ profile: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?

@_silgen_name("tidymac_disk_usage")
func tidymac_disk_usage() -> UnsafeMutablePointer<CChar>?

@_silgen_name("tidymac_apps_list")
func tidymac_apps_list() -> UnsafeMutablePointer<CChar>?

@_silgen_name("tidymac_app_clean_leftovers")
func tidymac_app_clean_leftovers(_ appName: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?

@_silgen_name("tidymac_clean")
func tidymac_clean(_ profile: UnsafePointer<CChar>?, _ mode: UnsafePointer<CChar>?, _ selectedNamesJson: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?

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

    func scan(profile: String) -> ScanResult? {
        let result = profile.withCString { tidymac_scan($0) }
        guard let dict = callFFI(result) else { return nil }
        return ScanResult(from: dict)
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

    func appCleanLeftovers(appName: String) -> AppCleanResult? {
        let result = appName.withCString { tidymac_app_clean_leftovers($0) }
        guard let dict = callFFI(result) else { return nil }
        return AppCleanResult(from: dict)
    }

    func clean(profile: String, mode: String, selectedNames: [String]? = nil) -> CleanResult? {
        // Serialize selected names to JSON
        let namesJson: String? = if let names = selectedNames {
            (try? JSONSerialization.data(withJSONObject: names)).flatMap { String(data: $0, encoding: .utf8) }
        } else {
            nil
        }

        let result = profile.withCString { p in
            mode.withCString { m in
                if let json = namesJson {
                    return json.withCString { s in
                        tidymac_clean(p, m, s)
                    }
                } else {
                    return tidymac_clean(p, m, nil)
                }
            }
        }
        guard let dict = callFFI(result) else { return nil }
        return CleanResult(from: dict)
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
}

// MARK: - Data Models

struct ScanResult {
    let profile: String
    let durationSecs: Double
    let totalReclaimable: UInt64
    let totalReclaimableFormatted: String
    let totalFiles: Int
    let items: [ScanItem]
    let errors: [String]

    init?(from dict: [String: Any]) {
        guard let profile = dict["profile"] as? String else { return nil }
        self.profile = profile
        self.durationSecs = dict["duration_secs"] as? Double ?? 0
        self.totalReclaimable = dict["total_reclaimable"] as? UInt64 ?? 0
        self.totalReclaimableFormatted = dict["total_reclaimable_formatted"] as? String ?? "0 B"
        self.totalFiles = dict["total_files"] as? Int ?? 0
        self.errors = dict["errors"] as? [String] ?? []

        if let itemDicts = dict["items"] as? [[String: Any]] {
            self.items = itemDicts.compactMap { ScanItem(from: $0) }
        } else {
            self.items = []
        }
    }
}

struct ScanItem: Identifiable {
    let id = UUID()
    let name: String
    let category: String
    let path: String
    let sizeBytes: UInt64
    let sizeFormatted: String
    let fileCount: Int
    let safety: String
    let reason: String

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

    init?(from dict: [String: Any]) {
        guard let name = dict["name"] as? String else { return nil }
        self.name = name
        self.category = dict["category"] as? String ?? ""
        self.path = dict["path"] as? String ?? ""
        self.sizeBytes = dict["size_bytes"] as? UInt64 ?? 0
        self.sizeFormatted = dict["size_formatted"] as? String ?? "0 B"
        self.fileCount = dict["file_count"] as? Int ?? 0
        self.safety = dict["safety"] as? String ?? "Safe"
        self.reason = dict["reason"] as? String ?? ""
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

    let id = UUID()
    let name: String
    let bundleId: String?
    let version: String?
    let path: String
    let appSize: UInt64
    let appSizeFormatted: String
    let totalSize: UInt64
    let totalSizeFormatted: String
    let leftoversSize: UInt64
    let leftoversFormatted: String
    let source: String
    let associatedFiles: [AssociatedFileInfo]

    init?(from dict: [String: Any]) {
        guard let name = dict["name"] as? String else { return nil }
        self.name = name
        self.bundleId = dict["bundle_id"] as? String
        self.version = dict["version"] as? String
        self.path = dict["path"] as? String ?? ""
        self.appSize = dict["app_size"] as? UInt64 ?? 0
        self.appSizeFormatted = dict["app_size_formatted"] as? String ?? "0 B"
        self.totalSize = dict["total_size"] as? UInt64 ?? 0
        self.totalSizeFormatted = dict["total_size_formatted"] as? String ?? "0 B"
        self.leftoversSize = dict["leftovers_size"] as? UInt64 ?? 0
        self.leftoversFormatted = dict["leftovers_formatted"] as? String ?? "0 B"
        self.source = dict["source"] as? String ?? ""

        if let files = dict["associated_files"] as? [[String: Any]] {
            self.associatedFiles = files.compactMap { AssociatedFileInfo(from: $0) }
        } else {
            self.associatedFiles = []
        }
    }
}

struct AssociatedFileInfo: Identifiable {
    let id = UUID()
    let path: String
    let size: UInt64
    let sizeFormatted: String
    let kind: String

    init?(from dict: [String: Any]) {
        self.path = dict["path"] as? String ?? ""
        self.size = dict["size"] as? UInt64 ?? 0
        self.sizeFormatted = dict["size_formatted"] as? String ?? "0 B"
        self.kind = dict["kind"] as? String ?? ""
    }
}

struct CleanResult {
    let mode: String
    let filesRemoved: Int
    let bytesFreed: UInt64
    let bytesFreedFormatted: String
    let sessionId: String?
    let errors: [String]

    init?(from dict: [String: Any]) {
        self.mode = dict["mode"] as? String ?? ""
        self.filesRemoved = dict["files_removed"] as? Int ?? 0
        self.bytesFreed = dict["bytes_freed"] as? UInt64 ?? 0
        self.bytesFreedFormatted = dict["bytes_freed_formatted"] as? String ?? "0 B"
        self.sessionId = dict["session_id"] as? String
        self.errors = dict["errors"] as? [String] ?? []
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
    let appName: String
    let filesRemoved: Int
    let bytesFreed: UInt64
    let bytesFreedFormatted: String
    let removedPaths: [String]
    let skipped: [String]
    let errors: [String]

    init?(from dict: [String: Any]) {
        self.appName = dict["app_name"] as? String ?? ""
        self.filesRemoved = dict["files_removed"] as? Int ?? 0
        self.bytesFreed = dict["bytes_freed"] as? UInt64 ?? 0
        self.bytesFreedFormatted = dict["bytes_freed_formatted"] as? String ?? "0 B"
        self.removedPaths = dict["removed_paths"] as? [String] ?? []
        self.skipped = dict["skipped"] as? [String] ?? []
        self.errors = dict["errors"] as? [String] ?? []
    }
}
