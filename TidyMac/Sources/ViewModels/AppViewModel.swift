import SwiftUI
import Combine

@MainActor
class AppViewModel: ObservableObject {
    // MARK: - App Sandbox Bookmarks
    @Published var hasFullDiskAccess = false
    @Published var selectedFolders: [URL] = []
    
    // MARK: - Dashboard
    @Published var diskUsage: DiskUsageResult?
    @Published var isLoadingDisk = false

    // MARK: - Scan
    @Published var scanResult: ScanResult?
    @Published var isScanning = false
    @Published var selectedProfile: String = UserDefaults.standard.string(forKey: "defaultProfile") ?? "developer"
    @Published var selectedItems: Set<UUID> = []
    @Published var lastScanDate: Double = UserDefaults.standard.double(forKey: "lastScanDate")

    // MARK: - Clean
    @Published var cleanResult: CleanResult?
    @Published var isCleaning = false
    @Published var showCleanConfirm = false
    @Published var showCleanResult = false

    var selectedItemCount: Int {
        selectedItems.count
    }

    var selectedFileCount: Int {
        guard let items = scanResult?.items else { return 0 }
        return items.filter { selectedItems.contains($0.id) }.reduce(0) { $0 + $1.fileCount }
    }

    var selectedBytes: UInt64 {
        guard let items = scanResult?.items else { return 0 }
        return items.filter { selectedItems.contains($0.id) }.reduce(UInt64(0)) { $0 + $1.sizeBytes }
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
        loadSavedBookmarks()
        loadProfiles()
        loadDiskUsage()
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
            let result = bridge.scan(profile: profile)
            let now = Date().timeIntervalSince1970
            UserDefaults.standard.set(now, forKey: "lastScanDate")
            
            await MainActor.run { [weak self] in
                self?.scanResult = result
                self?.isScanning = false
                self?.lastScanDate = now
                if let items = result?.items {
                    self?.selectedItems = Set(items.filter { $0.safety == "Safe" }.map { $0.id })
                }
            }
        }
    }

    func cancelScan() {
        bridge.cancelScan()
    }

    func runClean(mode: String) {
        startAccessingSecurityScopedResources()
        isCleaning = true
        let profile = selectedProfile

        var selectedNames: [String]? = nil
        if let items = scanResult?.items {
            let names = items.filter { selectedItems.contains($0.id) }.map { $0.name }
            if !names.isEmpty {
                selectedNames = names
            }
        }

        Task.detached { [bridge] in
            let result = bridge.clean(profile: profile, mode: mode, selectedNames: selectedNames)
            await MainActor.run { [weak self] in
                self?.cleanResult = result
                self?.isCleaning = false
                self?.showCleanResult = true
                self?.loadDiskUsage()
            }
        }
    }

    func cleanAppLeftovers(appName: String) {
        startAccessingSecurityScopedResources()
        isCleaningApp = true
        Task.detached { [bridge] in
            let result = bridge.appCleanLeftovers(appName: appName)
            await MainActor.run { [weak self] in
                self?.appCleanResult = result
                self?.isCleaningApp = false
                self?.showAppCleanResult = true
                self?.loadApps()
            }
        }
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
        
        self.selectedFolders = loadedURLs
        self.hasFullDiskAccess = !loadedURLs.isEmpty
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
