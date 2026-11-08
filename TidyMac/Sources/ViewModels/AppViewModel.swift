import SwiftUI
import Combine

@MainActor
class AppViewModel: ObservableObject {
    // MARK: - Navigation
    @Published var selectedTab: SidebarItem = .dashboard
    
    // MARK: - Permissions
    @Published var hasFullDiskAccess = false
    @Published var selectedFolders: [URL] = []
    
    // MARK: - Dashboard
    @Published var diskUsage: DiskUsageResult?
    @Published var isLoadingDisk = false

    // MARK: - Scan
    @Published var scanResult: ScanResult?
    @Published var isScanning = false
    @Published var selectedProfile: String = UserDefaults.standard.string(forKey: "defaultProfile") ?? "developer"
    /// Selected item ids (indices) of the current scan.
    @Published var selectedItems: Set<Int> = []
    /// Entries the user unticked inside selected items.
    @Published var excludedEntries: Set<String> = []
    @Published var scanError: String?
    @Published var lastScanDate: Double = UserDefaults.standard.double(forKey: "lastScanDate")

    // MARK: - Clean
    @Published var cleanResult: CleanResult?
    @Published var isCleaning = false
    @Published var showCleanConfirm = false
    @Published var showCleanResult = false

    var selectedItemCount: Int {
        selectedItems.count
    }

    private var selectedCleanableItems: [ScanItem] {
        guard let items = scanResult?.items else { return [] }
        return items.filter { selectedItems.contains($0.id) && $0.cleanable }
    }

    var selectedFileCount: Int {
        selectedCleanableItems.reduce(0) { total, item in
            total + item.entries.filter { !excludedEntries.contains($0.path) }.count
                + max(0, item.entryCount - item.entries.count)
        }
    }

    var selectedBytes: UInt64 {
        selectedCleanableItems.reduce(UInt64(0)) { total, item in
            let excluded = item.entries.filter { excludedEntries.contains($0.path) }.reduce(UInt64(0)) { $0 + $1.sizeBytes }
            return total + (item.sizeBytes > excluded ? item.sizeBytes - excluded : 0)
        }
    }

    var selectionIncludesTools: Bool {
        selectedCleanableItems.contains { $0.toolAction != nil }
    }

    var selectedSizeFormatted: String {
        let bytes = selectedBytes
        if bytes >= 1_073_741_824 {
            return String(format: "%.2f GB", Double(bytes) / 1_073_741_824)
        } else if bytes >= 1_048_576 {
            return String(format: "%.1f MB", Double(bytes) / 1_048_576)
        } else if bytes >= 1024 {
            return String(format: "%.0f KB", Double(bytes) / 1024)
        }
        return "\(bytes) B"
    }

    // MARK: - Apps
    @Published var apps: [AppInfo] = []
    @Published var isLoadingApps = false
    @Published var appCleanResult: AppCleanResult?
    @Published var showAppCleanResult = false
    @Published var isCleaningApp = false
    @Published var installInfo: [String: InstallInfoResult] = [:]

    // MARK: - Leftovers of uninstalled apps
    @Published var orphans: [OrphanGroupInfo] = []
    @Published var isLoadingOrphans = false
    @Published var orphanCleanResult: CleanResult?

    // MARK: - Duplicates
    @Published var dupResult: DupResult?
    @Published var isFindingDuplicates = false
    @Published var dupFolder: URL? = FileManager.default.homeDirectoryForCurrentUser
    @Published var dupCleanResult: CleanResult?

    // MARK: - Disk map
    @Published var diskTree: DiskNode?
    @Published var diskMapPath: [String] = []
    @Published var isLoadingDiskTree = false
    @Published var systemData: SystemDataInfo?
    @Published var isLoadingSystemData = false

    // MARK: - Schedule
    @Published var schedule: ScheduleInfo?
    @Published var scheduleError: String?

    // MARK: - Generic result sheet (large files, privacy, purge)
    @Published var lastActionResult: CleanResult?
    @Published var showActionResult = false

    // MARK: - Privacy
    @Published var privacyResult: PrivacyResult?
    @Published var isLoadingPrivacy = false

    // MARK: - Docker
    @Published var dockerResult: DockerResult?
    @Published var isLoadingDocker = false

    // MARK: - Startup
    @Published var startupItems: [StartupItemInfo] = []
    @Published var isLoadingStartup = false

    // MARK: - History
    @Published var undoSessions: [UndoSession] = []
    @Published var isLoadingHistory = false
    @Published var undoResult: UndoResult?
    @Published var showUndoResult = false
    @Published var isLoadingHistoryDetail = false

    // MARK: - Profiles
    @Published var profiles: [ProfileInfo] = []

    private let bridge = TidyMacBridge.shared

    init() {
        checkFullDiskAccess()
        loadSavedBookmarks()
        loadProfiles()
        loadDiskUsage()
        loadSchedule()
    }

    // MARK: - Actions

    func loadDiskUsage() {
        startAccessingSecurityScopedResources()
        isLoadingDisk = true
        Task.detached { [bridge] in
            let result = bridge.diskUsage()
            await MainActor.run { [weak self] in
                self?.diskUsage = result
                self?.isLoadingDisk = false
            }
        }
    }

    func runScan() {
        startAccessingSecurityScopedResources()
        isScanning = true
        scanResult = nil
        let profile = selectedProfile
        Task.detached { [bridge] in
            let (result, error) = bridge.scan(profile: profile)
            let now = Date().timeIntervalSince1970
            UserDefaults.standard.set(now, forKey: "lastScanDate")
            
            await MainActor.run { [weak self] in
                self?.scanResult = result
                self?.scanError = (error == "CANCELLED") ? nil : error
                self?.isScanning = false
                self?.lastScanDate = now
                self?.excludedEntries = []
                if let items = result?.items {
                    self?.selectedItems = Set(items.filter { $0.safety == "Safe" && $0.cleanable }.map { $0.id })
                }
            }
        }
    }

    func cancelScan() {
        bridge.cancelScan()
    }

    /// Clean exactly what the user reviewed: the selected items of the current
    /// scan, minus unticked entries. The core rejects stale scans.
    func runClean(mode: String) {
        guard let scan = scanResult else { return }
        let ids = selectedCleanableItems.map { $0.id }
        guard !ids.isEmpty else { return }
        let excluded = Array(excludedEntries)
        let profile = scan.profile
        let scanId = scan.scanId
        isCleaning = true

        Task.detached { [bridge] in
            let result = bridge.clean(profile: profile, mode: mode, scanId: scanId, itemIds: ids, excludePaths: excluded)
            await MainActor.run { [weak self] in
                self?.cleanResult = result
                self?.isCleaning = false
                self?.showCleanResult = true
                if result?.error == nil {
                    // The core forgets a scan once it has been cleaned.
                    self?.scanResult = nil
                    self?.selectedItems = []
                    self?.excludedEntries = []
                }
                self?.loadDiskUsage()
            }
        }
    }

    func toggleEntry(_ path: String) {
        if excludedEntries.contains(path) {
            excludedEntries.remove(path)
        } else {
            excludedEntries.insert(path)
        }
    }

    /// Remove hand-picked files (large files, duplicates). Soft delete → undoable.
    func removeFiles(_ paths: [String], label: String, completion: (() -> Void)? = nil) {
        guard !paths.isEmpty else { return }
        Task.detached { [bridge] in
            let result = bridge.deletePaths(paths, mode: "soft", label: label)
            await MainActor.run { [weak self] in
                self?.lastActionResult = result
                self?.showActionResult = true
                self?.loadDiskUsage()
                completion?()
            }
        }
    }

    // MARK: - Apps

    /// Clear caches/logs/saved state of an installed app (keeps settings and data).
    func cleanAppCaches(app: AppInfo) {
        isCleaningApp = true
        let id = app.identifier
        Task.detached { [bridge] in
            let result = bridge.appCleanLeftovers(identifier: id)
            await MainActor.run { [weak self] in
                self?.appCleanResult = result
                self?.isCleaningApp = false
                self?.showAppCleanResult = true
                self?.loadApps()
            }
        }
    }

    /// Uninstall an app (bundle + data). Soft delete by default, so it can be undone from History.
    func uninstallApp(app: AppInfo, includeNameMatches: Bool, onlyPaths: [String], mode: String = "soft") {
        isCleaningApp = true
        let id = app.identifier
        Task.detached { [bridge] in
            let result = bridge.uninstallApp(identifier: id, mode: mode, includeNameMatches: includeNameMatches, onlyPaths: onlyPaths)
            await MainActor.run { [weak self] in
                self?.appCleanResult = result
                self?.isCleaningApp = false
                self?.showAppCleanResult = true
                self?.loadApps()
                self?.loadDiskUsage()
            }
        }
    }

    func loadInstallInfo(app: AppInfo) {
        guard installInfo[app.path] == nil else { return }
        let path = app.path
        Task.detached { [bridge] in
            let info = bridge.appInstallInfo(path: path)
            await MainActor.run { [weak self] in
                if let info = info { self?.installInfo[path] = info }
            }
        }
    }

    func loadOrphans() {
        isLoadingOrphans = true
        Task.detached { [bridge] in
            let result = bridge.orphans()
            await MainActor.run { [weak self] in
                self?.orphans = result
                self?.isLoadingOrphans = false
            }
        }
    }

    func cleanOrphans(paths: [String]) {
        guard !paths.isEmpty else { return }
        isLoadingOrphans = true
        Task.detached { [bridge] in
            let result = bridge.cleanOrphans(paths: paths)
            let refreshed = bridge.orphans()
            await MainActor.run { [weak self] in
                self?.orphanCleanResult = result
                self?.lastActionResult = result
                self?.showActionResult = true
                self?.orphans = refreshed
                self?.isLoadingOrphans = false
            }
        }
    }

    // MARK: - Duplicates

    func findDuplicates(perceptual: Bool) {
        guard let folder = dupFolder else { return }
        isFindingDuplicates = true
        dupResult = nil
        let path = folder.path
        Task.detached { [bridge] in
            let result = bridge.duplicates(path: path, perceptual: perceptual)
            await MainActor.run { [weak self] in
                self?.dupResult = result
                self?.isFindingDuplicates = false
            }
        }
    }

    func chooseDuplicateFolder() {
        let panel = NSOpenPanel()
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.allowsMultipleSelection = false
        panel.prompt = "Choose"
        if panel.runModal() == .OK, let url = panel.url {
            dupFolder = url
        }
    }

    // MARK: - Disk map / System Data

    func loadDiskTree(path: String? = nil) {
        isLoadingDiskTree = true
        Task.detached { [bridge] in
            let tree = bridge.diskTree(path: path, depth: 3, maxChildren: 30)
            await MainActor.run { [weak self] in
                self?.diskTree = tree
                self?.isLoadingDiskTree = false
            }
        }
    }

    func drillDown(into node: DiskNode) {
        guard node.isDir, !node.isOther else { return }
        if let current = diskTree?.path { diskMapPath.append(current) }
        loadDiskTree(path: node.path)
    }

    func drillUp() {
        guard let previous = diskMapPath.popLast() else { return }
        loadDiskTree(path: previous)
    }

    func loadSystemData() {
        isLoadingSystemData = true
        Task.detached { [bridge] in
            let info = bridge.systemData()
            await MainActor.run { [weak self] in
                self?.systemData = info
                self?.isLoadingSystemData = false
            }
        }
    }

    // MARK: - Privacy / purge / schedule

    func cleanPrivacy(caches: Bool, browserCookies: Bool, appCookies: Bool) {
        Task.detached { [bridge] in
            let result = bridge.privacyClean(caches: caches, browserCookies: browserCookies, appCookies: appCookies)
            await MainActor.run { [weak self] in
                self?.lastActionResult = result
                self?.showActionResult = true
                self?.loadPrivacy()
            }
        }
    }

    func purgeStaging(all: Bool) {
        Task.detached { [bridge] in
            _ = bridge.purge(all: all)
            await MainActor.run { [weak self] in
                self?.loadHistory()
                self?.loadDiskUsage()
            }
        }
    }

    func loadSchedule() {
        Task.detached { [bridge] in
            let info = bridge.scheduleStatus()
            await MainActor.run { [weak self] in self?.schedule = info }
        }
    }

    func installSchedule(profile: String, interval: String) {
        Task.detached { [bridge] in
            let error = bridge.installSchedule(profile: profile, interval: interval)
            let info = bridge.scheduleStatus()
            await MainActor.run { [weak self] in
                self?.scheduleError = error
                self?.schedule = info
            }
        }
    }

    func removeSchedule() {
        Task.detached { [bridge] in
            bridge.removeSchedule()
            let info = bridge.scheduleStatus()
            await MainActor.run { [weak self] in self?.schedule = info }
        }
    }

    /// Space available for "important" use (what Finder shows, including purgeable space).
    var freeSpaceFormatted: String {
        let url = URL(fileURLWithPath: NSHomeDirectory())
        guard let values = try? url.resourceValues(forKeys: [.volumeAvailableCapacityForImportantUsageKey]),
              let bytes = values.volumeAvailableCapacityForImportantUsage else { return "—" }
        return ByteCountFormatter.string(fromByteCount: bytes, countStyle: .file)
    }

    func loadApps() {
        startAccessingSecurityScopedResources()
        isLoadingApps = true
        Task.detached { [bridge] in
            let result = bridge.appsList()
            await MainActor.run { [weak self] in
                self?.apps = result
                self?.isLoadingApps = false
            }
        }
    }

    func loadPrivacy() {
        startAccessingSecurityScopedResources()
        isLoadingPrivacy = true
        Task.detached { [bridge] in
            let result = bridge.privacyScan()
            await MainActor.run { [weak self] in
                self?.privacyResult = result
                self?.isLoadingPrivacy = false
            }
        }
    }

    func loadDocker() {
        isLoadingDocker = true
        Task.detached { [bridge] in
            let result = bridge.dockerUsage()
            await MainActor.run { [weak self] in
                self?.dockerResult = result
                self?.isLoadingDocker = false
            }
        }
    }

    func loadStartupItems() {
        isLoadingStartup = true
        Task.detached { [bridge] in
            let result = bridge.startupList()
            await MainActor.run { [weak self] in
                self?.startupItems = result
                self?.isLoadingStartup = false
            }
        }
    }

    func toggleStartupItem(label: String, enable: Bool) {
        Task.detached { [bridge] in
            let success = bridge.toggleStartupItem(label: label, enable: enable)
            if success {
                await MainActor.run { [weak self] in
                    self?.loadStartupItems()
                }
            }
        }
    }

    func loadHistory() {
        isLoadingHistory = true
        Task.detached { [bridge] in
            let result = bridge.undoList()
            await MainActor.run { [weak self] in
                self?.undoSessions = result
                self?.isLoadingHistory = false
            }
        }
    }

    func loadProfiles() {
        Task.detached { [bridge] in
            let result = bridge.profilesList()
            await MainActor.run { [weak self] in
                self?.profiles = result
            }
        }
    }

    func undoSession(id: String) {
        startAccessingSecurityScopedResources()
        Task.detached { [bridge] in
            let result = bridge.undoSession(id: id)
            await MainActor.run { [weak self] in
                self?.undoResult = result
                self?.showUndoResult = true
                self?.loadHistory()
                self?.loadDiskUsage()
            }
        }
    }
    
    func navigateTo(_ tab: SidebarItem) {
        selectedTab = tab
    }

    /// Asks the core, which probes the TCC database (readable only with Full Disk Access).
    func checkFullDiskAccess() {
        self.hasFullDiskAccess = bridge.hasFullDiskAccess()
    }

    func openFullDiskAccessSettings() {
        if let url = URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles") {
            NSWorkspace.shared.open(url)
        }
    }

    // MARK: - Security Scoped Bookmarks
    
    func requestFolderAccess() {
        let panel = NSOpenPanel()
        panel.message = "Select folders to grant TidyMac access for scanning."
        panel.prompt = "Grant Access"
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.allowsMultipleSelection = true
        panel.canCreateDirectories = false
        
        if panel.runModal() == .OK {
            for url in panel.urls {
                saveBookmark(for: url)
            }
            loadSavedBookmarks()
            startAccessingSecurityScopedResources()
        }
    }
    
    private func saveBookmark(for url: URL) {
        do {
            let bookmarkData = try url.bookmarkData(options: .withSecurityScope, includingResourceValuesForKeys: nil, relativeTo: nil)
            var bookmarks = UserDefaults.standard.dictionary(forKey: "SecurityScopedBookmarks") as? [String: Data] ?? [:]
            bookmarks[url.path] = bookmarkData
            UserDefaults.standard.set(bookmarks, forKey: "SecurityScopedBookmarks")
        } catch {
            print("Failed to save bookmark: \(error.localizedDescription)")
        }
    }
    
    func loadSavedBookmarks() {
        guard let bookmarks = UserDefaults.standard.dictionary(forKey: "SecurityScopedBookmarks") as? [String: Data] else {
            return
        }
        
        var loadedURLs: [URL] = []
        for (_, data) in bookmarks {
            var isStale = false
            do {
                let url = try URL(resolvingBookmarkData: data, options: .withSecurityScope, relativeTo: nil, bookmarkDataIsStale: &isStale)
                if !isStale {
                    loadedURLs.append(url)
                }
            } catch {
                print("Failed to resolve bookmark: \(error.localizedDescription)")
            }
        }
        
        // Note: folder bookmarks don't grant Full Disk Access (TidyMac isn't sandboxed);
        // they're kept only for folders the user picked.
        self.selectedFolders = loadedURLs
    }
    
    func startAccessingSecurityScopedResources() {
        for url in selectedFolders {
            _ = url.startAccessingSecurityScopedResource()
        }
    }
    
    func stopAccessingSecurityScopedResources() {
        for url in selectedFolders {
            url.stopAccessingSecurityScopedResource()
        }
    }
}
