import SwiftUI

struct SettingsView: View {
    @State private var selectedProfile = "developer"
    @State private var staleDays: Double = 30
    @State private var retentionDays: Double = 7
    @State private var showLargeFiles = true
    @State private var largeFileThresholdMB: Double = 100

    var body: some View {
        TabView {
            GeneralSettingsTab(
                selectedProfile: $selectedProfile,
                staleDays: $staleDays,
                retentionDays: $retentionDays
            )
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

    var body: some View {
        Form {
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
                    Text("\(Int(staleDays)) days")
                        .frame(width: 60, alignment: .trailing)
                        .monospacedDigit()
                }
            }

            Section("Undo Window") {
                HStack {
                    Text("Keep soft-deleted files for")
                    Slider(value: $retentionDays, in: 1...30, step: 1)
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

                if showLargeFiles {
                    HStack {
                        Text("Minimum size")
                        Slider(value: $largeFileThresholdMB, in: 10...500, step: 10)
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
                .foregroundStyle(.green)

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
