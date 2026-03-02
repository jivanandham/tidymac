import SwiftUI
#if canImport(Sparkle)
import Sparkle
#endif

@main
struct TidyMacApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) var appDelegate
    
    #if canImport(Sparkle)
    private let updaterController: SPUStandardUpdaterController
    
    init() {
        updaterController = SPUStandardUpdaterController(startingUpdater: true, updaterDelegate: nil, userDriverDelegate: nil)
    }
    #endif

    var body: some Scene {
        WindowGroup {
            ContentView()
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
            SettingsView()
        }
    }
}

class AppDelegate: NSObject, NSApplicationDelegate {
    func applicationDidFinishLaunching(_ notification: Notification) {
        // Initialize observability (logging + sentry)
        TidyMacBridge.shared.initObservability(verbose: true, sentryDsn: nil)
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        true
    }
}
