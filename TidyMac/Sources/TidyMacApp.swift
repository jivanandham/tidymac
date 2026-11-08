import SwiftUI
#if canImport(Sparkle)
import Sparkle
#endif

@main
struct TidyMacApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) var appDelegate
    /// One model shared by the main window, the menu bar extra and Settings.
    @StateObject private var model = AppViewModel()
    @AppStorage("showMenuBarExtra") private var showMenuBarExtra = true
    
    #if canImport(Sparkle)
    private let updaterController: SPUStandardUpdaterController
    
    init() {
        updaterController = SPUStandardUpdaterController(startingUpdater: true, updaterDelegate: nil, userDriverDelegate: nil)
    }
    #endif

    var body: some Scene {
        WindowGroup {
            ContentView(viewModel: model)
                .frame(minWidth: 900, minHeight: 600)
        }
        .windowStyle(.titleBar)
        .windowToolbarStyle(.unified(showsTitle: true))
        .defaultSize(width: 1100, height: 720)
        .commands {
            CommandGroup(replacing: .newItem) {}
            CommandGroup(after: .appInfo) {
                #if canImport(Sparkle)
                Button("Check for Updates...") {
                    updaterController.checkForUpdates(nil)
                }
                #else
                Button("Check for Updates...") {
                    // Fallback if Sparkle not linked
                }
                .disabled(true)
                #endif
            }
        }

        Settings {
            SettingsView(appViewModel: model)
        }

        MenuBarExtra(isInserted: $showMenuBarExtra) {
            MenuBarContent(viewModel: model)
        } label: {
            Image(systemName: "leaf")
        }
        .menuBarExtraStyle(.window)
    }
}

class AppDelegate: NSObject, NSApplicationDelegate {
    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApp.setActivationPolicy(.regular)
        // Local file logging only — TidyMac makes no network connections.
        TidyMacBridge.shared.initObservability(verbose: false)
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        false
    }

    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
        if !flag {
            for window in sender.windows {
                window.makeKeyAndOrderFront(self)
            }
        }
        return true
    }
}

/// Small popover in the menu bar: free space, last scheduled report, quick actions.
struct MenuBarContent: View {
    @ObservedObject var viewModel: AppViewModel

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Image(systemName: "leaf.fill").foregroundStyle(TidyTheme.emerald)
                Text("TidyMac").font(.headline)
                Spacer()
            }
            HStack {
                Text("Available")
                Spacer()
                Text(viewModel.freeSpaceFormatted).fontWeight(.semibold)
            }
            if let usage = viewModel.diskUsage {
                ProgressView(value: Double(usage.usedPercentage), total: 100)
            }
            if let report = viewModel.schedule?.latestReport {
                Divider()
                Text("Last scheduled report (\(report.profile))").font(.caption).foregroundStyle(.secondary)
                Text("\(ByteCountFormatter.string(fromByteCount: Int64(report.totalReclaimable), countStyle: .file)) could be cleaned")
                    .font(.callout)
            }
            Divider()
            Button {
                viewModel.selectedTab = .scan
                activateMainWindow()
                viewModel.runScan()
            } label: {
                Label("Scan with \(viewModel.selectedProfile) profile", systemImage: "magnifyingglass")
            }
            Button {
                viewModel.selectedTab = .diskMap
                activateMainWindow()
            } label: {
                Label("Open Disk Map", systemImage: "square.grid.3x3.square")
            }
            Button {
                activateMainWindow()
            } label: {
                Label("Open TidyMac", systemImage: "macwindow")
            }
            Divider()
            Button("Quit TidyMac") { NSApp.terminate(nil) }
        }
        .buttonStyle(.plain)
        .padding(14)
        .frame(width: 260)
        .onAppear {
            viewModel.loadDiskUsage()
            viewModel.loadSchedule()
        }
    }

    private func activateMainWindow() {
        NSApp.activate(ignoringOtherApps: true)
        if let window = NSApp.windows.first(where: { $0.canBecomeMain }) {
            window.makeKeyAndOrderFront(nil)
        }
    }
}
