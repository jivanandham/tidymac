import SwiftUI

struct SettingsView: View {
    @AppStorage("defaultProfile") private var selectedProfile = "developer"
    @AppStorage("staleDays") private var staleDays: Double = 30
    @AppStorage("retentionDays") private var retentionDays: Double = 7
    @AppStorage("showLargeFiles") private var showLargeFiles = true
    @AppStorage("largeFileThresholdMB") private var largeFileThresholdMB: Double = 100
    @AppStorage("accentColor") private var accentColor = "emerald"

    @StateObject private var appViewModel = AppViewModel()

    var body: some View {
        TabView {
            GeneralSettingsTab(
                selectedProfile: $selectedProfile,
                staleDays: $staleDays,
                retentionDays: $retentionDays,
                accentColor: $accentColor
            )
            .environmentObject(appViewModel)
            .tabItem {
                Label("General", systemImage: "gearshape")
            }

            ScanSettingsTab(
                showLargeFiles: $showLargeFiles,
                largeFileThresholdMB: $largeFileThresholdMB
            )
            .tabItem {
                Label("Scanning", systemImage: "magnifyingglass")
            }

            AboutTab()
            .tabItem {
                Label("About", systemImage: "info.circle")
            }
        }
        .frame(width: 480, height: 360)
    }
}

struct GeneralSettingsTab: View {
    @Binding var selectedProfile: String
    @Binding var staleDays: Double
    @Binding var retentionDays: Double
    @Binding var accentColor: String
    @EnvironmentObject var appViewModel: AppViewModel

    var body: some View {
        Form {
            Section("App Sandbox Permissions") {
                VStack(alignment: .leading, spacing: 8) {
                    Text("TidyMac runs in a secure sandbox. You must explicitly grant access to folders you want to scan and clean.")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    
                    if appViewModel.hasFullDiskAccess {
                        HStack {
                            Image(systemName: "checkmark.seal.fill")
                                .foregroundColor(.green)
                            Text("Access granted to \(appViewModel.selectedFolders.count) folder(s)")
                        }
                    } else {
                        HStack {
                            Image(systemName: "exclamationmark.triangle.fill")
                                .foregroundColor(.orange)
                            Text("No folders authorized")
                        }
                    }
                    
                    Button("Manage Folder Access...") {
                        appViewModel.requestFolderAccess()
                    }
                }
                .padding(.vertical, 4)
            }
            
            Section("Appearance") {
                Picker("Accent Color", selection: $accentColor) {
                    Label("Emerald", systemImage: "circle.fill").tint(.green).tag("emerald")
                    Label("Teal", systemImage: "circle.fill").tint(.teal).tag("teal")
                    Label("Sapphire", systemImage: "circle.fill").tint(.blue).tag("sapphire")
                    Label("Coral", systemImage: "circle.fill").tint(.orange).tag("coral")
                    Label("Amber", systemImage: "circle.fill").tint(.yellow).tag("amber")
                    Label("Lavender", systemImage: "circle.fill").tint(.purple).tag("lavender")
                    Label("Rose", systemImage: "circle.fill").tint(.pink).tag("rose")
                }
            }
            
            Section("Default Profile") {
                Picker("Profile", selection: $selectedProfile) {
                    Text("Quick Sweep").tag("quick")
                    Text("Developer").tag("developer")
                    Text("Creative").tag("creative")
                    Text("Deep Clean").tag("deep")
                }
                .pickerStyle(.radioGroup)
            }

            Section("Stale Detection") {
                HStack {
                    Text("Consider files stale after")
                    Slider(value: $staleDays, in: 7...90, step: 1)
                        .accessibilityLabel("Stale threshold")
                        .accessibilityValue("\(Int(staleDays)) days")
                    Text("\(Int(staleDays)) days")
                        .frame(width: 60, alignment: .trailing)
                        .monospacedDigit()
                }
            }

            Section("Undo Window") {
                HStack {
                    Text("Keep soft-deleted files for")
                    Slider(value: $retentionDays, in: 1...30, step: 1)
                        .accessibilityLabel("Retention period")
                        .accessibilityValue("\(Int(retentionDays)) days")
                    Text("\(Int(retentionDays)) days")
                        .frame(width: 60, alignment: .trailing)
                        .monospacedDigit()
                }
            }
        }
        .formStyle(.grouped)
        .padding()
    }
}

struct ScanSettingsTab: View {
    @Binding var showLargeFiles: Bool
    @Binding var largeFileThresholdMB: Double

    var body: some View {
        Form {
            Section("Large Files") {
                Toggle("Scan for large files", isOn: $showLargeFiles)
                    .accessibilityLabel("Enable large file scanning")

                if showLargeFiles {
                    HStack {
                        Text("Minimum size")
                        Slider(value: $largeFileThresholdMB, in: 10...500, step: 10)
                            .accessibilityLabel("Large file threshold")
                            .accessibilityValue("\(Int(largeFileThresholdMB)) Megabytes")
                        Text("\(Int(largeFileThresholdMB)) MB")
                            .frame(width: 60, alignment: .trailing)
                            .monospacedDigit()
                    }
                }
            }

            Section("Exclusions") {
                Text("Configure excluded paths in ~/.tidymac/config.toml")
                    .font(.caption)
                    .foregroundStyle(.secondary)

                Button("Open Config File") {
                    let configPath = ("~/.tidymac/config.toml" as NSString).expandingTildeInPath
                    NSWorkspace.shared.open(URL(fileURLWithPath: configPath))
                }
            }
        }
        .formStyle(.grouped)
        .padding()
    }
}

struct AboutTab: View {
    var body: some View {
        VStack(spacing: 16) {
            Spacer()

            Image(systemName: "leaf.fill")
                .font(.system(size: 48))
                .foregroundStyle(TidyTheme.primaryGradient)

            Text("TidyMac")
                .font(.title)
                .fontWeight(.bold)

            Text("v\(TidyMacBridge.shared.version())")
                .font(.subheadline)
                .foregroundStyle(.secondary)

            Text("A developer-aware, privacy-first\nMac cleanup utility built in Rust")
                .font(.caption)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)

            Divider()
                .frame(width: 200)

            VStack(spacing: 4) {
                Text("100% Offline • Zero Telemetry")
                    .font(.caption2)
                    .foregroundStyle(.tertiary)
                Text("MIT License • Open Source")
                    .font(.caption2)
                    .foregroundStyle(.tertiary)
            }

            Button("View on GitHub") {
                if let url = URL(string: "https://github.com/jeevakrishnasamy/tidymac") {
                    NSWorkspace.shared.open(url)
                }
            }
            .buttonStyle(.link)

            Spacer()
        }
        .frame(maxWidth: .infinity)
    }
}
