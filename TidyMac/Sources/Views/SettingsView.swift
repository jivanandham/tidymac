import SwiftUI

struct SettingsView: View {
    @AppStorage("defaultProfile") private var selectedProfile = "developer"
    @AppStorage("staleDays") private var staleDays: Double = 30
    @AppStorage("retentionDays") private var retentionDays: Double = 7
    @AppStorage("showLargeFiles") private var showLargeFiles = true
    @AppStorage("largeFileThresholdMB") private var largeFileThresholdMB: Double = 100
    @AppStorage("accentColor") private var accentColor = "emerald"

    @ObservedObject var appViewModel: AppViewModel

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

            ScheduleSettingsTab(viewModel: appViewModel)
            .tabItem {
                Label("Schedule", systemImage: "clock")
            }

            AboutTab()
            .tabItem {
                Label("About", systemImage: "info.circle")
            }
        }
        .frame(width: 520, height: 420)
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
            Section("Full Disk Access") {
                VStack(alignment: .leading, spacing: 8) {
                    Text("TidyMac isn't sandboxed. To see caches and app data that macOS protects (Mail, Safari, app containers), add TidyMac under System Settings › Privacy & Security › Full Disk Access.")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    if appViewModel.hasFullDiskAccess {
                        Label("Full Disk Access granted", systemImage: "checkmark.seal.fill")
                            .foregroundColor(.green)
                    } else {
                        Label("Full Disk Access not granted", systemImage: "exclamationmark.triangle.fill")
                            .foregroundColor(.orange)
                    }
                    HStack {
                        Button("Open Privacy Settings…") { appViewModel.openFullDiskAccessSettings() }
                        Button("Check Again") { appViewModel.checkFullDiskAccess() }
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

struct ScheduleSettingsTab: View {
    @ObservedObject var viewModel: AppViewModel
    @State private var profile = "developer"
    @State private var interval = "weekly"
    @AppStorage("showMenuBarExtra") private var showMenuBarExtra = true

    var body: some View {
        Form {
            Section("Scheduled dry-run report") {
                Text("TidyMac can scan on a schedule and record what could be cleaned. Scheduled reports never delete anything.")
                    .font(.caption).foregroundStyle(.secondary)
                Picker("Profile", selection: $profile) {
                    Text("Quick Sweep").tag("quick")
                    Text("Developer").tag("developer")
                    Text("Creative").tag("creative")
                    Text("Deep Clean").tag("deep")
                }
                Picker("Every", selection: $interval) {
                    Text("Day").tag("daily")
                    Text("Week").tag("weekly")
                    Text("Month").tag("monthly")
                }
                .pickerStyle(.segmented)
                HStack {
                    Button(viewModel.schedule?.installed == true ? "Update Schedule" : "Turn On") {
                        viewModel.installSchedule(profile: profile, interval: interval)
                    }
                    if viewModel.schedule?.installed == true {
                        Button("Turn Off") { viewModel.removeSchedule() }
                    }
                }
                if let info = viewModel.schedule, info.installed {
                    Label("Runs \(info.interval ?? "") with the \(info.profile ?? "") profile", systemImage: "clock")
                        .font(.caption)
                }
                if let error = viewModel.scheduleError {
                    Text(error).font(.caption).foregroundStyle(.orange)
                }
                if let report = viewModel.schedule?.latestReport {
                    Text("Latest report: \(ByteCountFormatter.string(fromByteCount: Int64(report.totalReclaimable), countStyle: .file)) reclaimable")
                        .font(.caption)
                }
            }
            Section("Menu bar") {
                Toggle("Show TidyMac in the menu bar", isOn: $showMenuBarExtra)
            }
        }
        .formStyle(.grouped)
        .onAppear {
            viewModel.loadSchedule()
            if let p = viewModel.schedule?.profile { profile = p }
            if let i = viewModel.schedule?.interval { interval = i }
        }
    }
}
