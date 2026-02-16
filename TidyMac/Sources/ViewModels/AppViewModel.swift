import SwiftUI
import Combine

@MainActor
class AppViewModel: ObservableObject {
    // MARK: - Dashboard
    @Published var diskUsage: DiskUsageResult?
    @Published var isLoadingDisk = false

    // MARK: - Scan
    @Published var scanResult: ScanResult?
    @Published var isScanning = false
    @Published var selectedProfile = "developer"
    @Published var selectedItems: Set<UUID> = []

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

    // MARK: - History
    @Published var undoSessions: [UndoSession] = []
    @Published var isLoadingHistory = false

    // MARK: - Profiles
    @Published var profiles: [ProfileInfo] = []

    private let bridge = TidyMacBridge.shared

    // MARK: - Actions

    func loadDiskUsage() {
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
        isScanning = true
        scanResult = nil
        let profile = selectedProfile
        Task.detached { [bridge] in
            let result = bridge.scan(profile: profile)
            await MainActor.run { [weak self] in
                self?.scanResult = result
                self?.isScanning = false
                if let items = result?.items {
                    self?.selectedItems = Set(items.filter { $0.safety == "Safe" }.map { $0.id })
                }
            }
        }
    }

    func runClean(mode: String) {
        isCleaning = true
        let profile = selectedProfile

        // Resolve selected UUIDs to item names
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
        isCleaningApp = true
        Task.detached { [bridge] in
            let result = bridge.appCleanLeftovers(appName: appName)
            await MainActor.run { [weak self] in
                self?.appCleanResult = result
                self?.isCleaningApp = false
                self?.showAppCleanResult = true
                // Refresh apps list to show updated sizes
                self?.loadApps()
            }
        }
    }

    func loadApps() {
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
        Task.detached { [bridge] in
            let _ = bridge.undoSession(id: id)
            await MainActor.run { [weak self] in
                self?.loadHistory()
                self?.loadDiskUsage()
            }
        }
    }
}
