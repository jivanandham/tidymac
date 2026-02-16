import SwiftUI

struct AppsView: View {
    @ObservedObject var viewModel: AppViewModel
    @State private var searchText = ""
    @State private var sortBy: AppSort = .size
    @State private var selectedApp: AppInfo?

    enum AppSort: String, CaseIterable {
        case size = "Size"
        case name = "Name"
    }

    var filteredApps: [AppInfo] {
        var apps = viewModel.apps
        if !searchText.isEmpty {
            apps = apps.filter { $0.name.localizedCaseInsensitiveContains(searchText) }
        }
        switch sortBy {
        case .size:
            apps.sort { $0.totalSize > $1.totalSize }
        case .name:
            apps.sort { $0.name.localizedCaseInsensitiveCompare($1.name) == .orderedAscending }
        }
        return apps
    }

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Applications")
                        .font(.largeTitle)
                        .fontWeight(.bold)
                    Text("\(viewModel.apps.count) apps installed")
                        .foregroundStyle(.secondary)
                }

                Spacer()

                Picker("Sort", selection: $sortBy) {
                    ForEach(AppSort.allCases, id: \.self) { sort in
                        Text(sort.rawValue).tag(sort)
                    }
                }
                .pickerStyle(.segmented)
                .frame(width: 160)

                Button {
                    viewModel.loadApps()
                } label: {
                    Label("Refresh", systemImage: "arrow.clockwise")
                }
                .disabled(viewModel.isLoadingApps)
            }
            .padding()

            Divider()

            if viewModel.isLoadingApps {
                Spacer()
                ProgressView("Scanning applications...")
                Spacer()
            } else if viewModel.apps.isEmpty {
                Spacer()
                ContentUnavailableView(
                    "No Apps Loaded",
                    systemImage: "app.badge",
                    description: Text("Click Refresh to scan installed applications")
                )
                Spacer()
            } else {
                HSplitView {
                    // App List
                    List(filteredApps, selection: $selectedApp) { app in
                        AppRow(app: app)
                            .tag(app)
                            .onTapGesture { selectedApp = app }
                    }
                    .listStyle(.inset(alternatesRowBackgrounds: true))
                    .searchable(text: $searchText, prompt: "Search apps...")
                    .frame(minWidth: 400)

                    // Detail Panel
                    if let app = selectedApp {
                        AppDetailView(app: app, viewModel: viewModel)
                            .frame(minWidth: 300)
                    } else {
                        VStack {
                            Spacer()
                            ContentUnavailableView(
                                "Select an App",
                                systemImage: "sidebar.right",
                                description: Text("Choose an app to see details")
                            )
                            Spacer()
                        }
                        .frame(minWidth: 300)
                    }
                }
            }
        }
        .onAppear {
            if viewModel.apps.isEmpty {
                viewModel.loadApps()
            }
        }
    }
}

struct AppRow: View {
    let app: AppInfo

    var body: some View {
        HStack(spacing: 12) {
            AppIconView(name: app.name)
                .frame(width: 36, height: 36)

            VStack(alignment: .leading, spacing: 2) {
                Text(app.name)
                    .fontWeight(.medium)
                HStack(spacing: 8) {
                    if let version = app.version {
                        Text("v\(version)")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                    Text(app.source)
                        .font(.caption2)
                        .padding(.horizontal, 6)
                        .padding(.vertical, 2)
                        .background(.blue.opacity(0.1), in: Capsule())
                        .foregroundStyle(.blue)
                }
            }

            Spacer()

            VStack(alignment: .trailing, spacing: 2) {
                Text(app.totalSizeFormatted)
                    .fontWeight(.semibold)
                if app.leftoversSize > 0 {
                    Text("+ \(app.leftoversFormatted) leftovers")
                        .font(.caption2)
                        .foregroundStyle(.orange)
                }
            }
        }
        .padding(.vertical, 4)
    }
}

struct AppIconView: View {
    let name: String

    var body: some View {
        ZStack {
            RoundedRectangle(cornerRadius: 8)
                .fill(iconColor.gradient)
            Text(String(name.prefix(1)).uppercased())
                .font(.system(size: 16, weight: .bold, design: .rounded))
                .foregroundStyle(.white)
        }
    }

    private var iconColor: Color {
        let colors: [Color] = [.blue, .purple, .orange, .green, .red, .teal, .pink, .indigo]
        let hash = name.unicodeScalars.reduce(0) { $0 + Int($1.value) }
        return colors[hash % colors.count]
    }
}

struct AppDetailView: View {
    let app: AppInfo
    @ObservedObject var viewModel: AppViewModel
    @State private var showCleanConfirm = false

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 20) {
                // Header
                HStack(spacing: 16) {
                    AppIconView(name: app.name)
                        .frame(width: 56, height: 56)

                    VStack(alignment: .leading, spacing: 4) {
                        Text(app.name)
                            .font(.title2)
                            .fontWeight(.bold)
                        if let bid = app.bundleId {
                            Text(bid)
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                        if let version = app.version {
                            Text("Version \(version)")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                    }

                    Spacer()

                    if app.leftoversSize > 0 {
                        Button {
                            showCleanConfirm = true
                        } label: {
                            Label("Clean Leftovers", systemImage: "trash")
                        }
                        .buttonStyle(.borderedProminent)
                        .tint(.orange)
                        .disabled(viewModel.isCleaningApp)
                    }
                }

                Divider()

                // Size Breakdown
                VStack(alignment: .leading, spacing: 12) {
                    Text("Size Breakdown")
                        .font(.headline)

                    HStack(spacing: 24) {
                        VStack {
                            Text(app.appSizeFormatted)
                                .font(.title3)
                                .fontWeight(.bold)
                                .foregroundStyle(.blue)
                            Text("App Bundle")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }

                        VStack {
                            Text(app.leftoversFormatted)
                                .font(.title3)
                                .fontWeight(.bold)
                                .foregroundStyle(.orange)
                            Text("Leftovers")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }

                        VStack {
                            Text(app.totalSizeFormatted)
                                .font(.title3)
                                .fontWeight(.bold)
                            Text("Total")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                    }

                    // Size bar
                    if app.totalSize > 0 {
                        GeometryReader { geo in
                            HStack(spacing: 2) {
                                let appPct = CGFloat(app.appSize) / CGFloat(app.totalSize)
                                RoundedRectangle(cornerRadius: 4)
                                    .fill(.blue)
                                    .frame(width: max(geo.size.width * appPct, 4))
                                RoundedRectangle(cornerRadius: 4)
                                    .fill(.orange)
                                    .frame(width: max(geo.size.width * (1 - appPct), 4))
                            }
                        }
                        .frame(height: 8)
                    }
                }

                // Associated Files
                if !app.associatedFiles.isEmpty {
                    Divider()

                    VStack(alignment: .leading, spacing: 8) {
                        Text("Associated Files")
                            .font(.headline)

                        ForEach(app.associatedFiles) { file in
                            HStack {
                                Image(systemName: iconForKind(file.kind))
                                    .foregroundStyle(.secondary)
                                    .frame(width: 20)

                                VStack(alignment: .leading, spacing: 1) {
                                    Text(file.kind)
                                        .font(.subheadline)
                                    Text(file.path)
                                        .font(.caption2)
                                        .foregroundStyle(.tertiary)
                                        .lineLimit(1)
                                        .truncationMode(.middle)
                                }

                                Spacer()

                                Text(file.sizeFormatted)
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            }
                            .padding(.vertical, 2)
                        }
                    }
                }

                // Path
                Divider()
                VStack(alignment: .leading, spacing: 4) {
                    Text("Location")
                        .font(.headline)
                    Text(app.path)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .textSelection(.enabled)
                }
            }
            .padding()
        }
        .alert("Clean Leftovers?", isPresented: $showCleanConfirm) {
            Button("Cancel", role: .cancel) {}
            Button("Clean Leftovers", role: .destructive) {
                viewModel.cleanAppLeftovers(appName: app.name)
            }
        } message: {
            Text("Remove \(app.associatedFiles.count) leftover files (\(app.leftoversFormatted)) for \(app.name)?\n\nThis removes caches, logs, and app support data. The app itself will NOT be removed.")
        }
        .sheet(isPresented: $viewModel.showAppCleanResult) {
            AppCleanResultSheet(result: viewModel.appCleanResult)
        }
    }

    private func iconForKind(_ kind: String) -> String {
        switch kind {
        case "Cache": return "folder"
        case "App Support": return "doc.fill"
        case "Preferences": return "gearshape"
        case "Saved State": return "bookmark.fill"
        case "Container": return "shippingbox"
        case "Logs": return "doc.text"
        case "Cookies": return "globe"
        default: return "doc"
        }
    }
}

struct AppCleanResultSheet: View {
    let result: AppCleanResult?
    @Environment(\.dismiss) var dismiss

    var body: some View {
        VStack(spacing: 16) {
            if let result = result {
                let hasIssues = !result.errors.isEmpty || !result.skipped.isEmpty
                let icon = result.filesRemoved > 0
                    ? "checkmark.circle.fill"
                    : (hasIssues ? "exclamationmark.triangle.fill" : "checkmark.circle.fill")
                let iconColor: Color = result.filesRemoved > 0 ? .green : (hasIssues ? .orange : .green)

                Image(systemName: icon)
                    .font(.system(size: 48))
                    .foregroundStyle(iconColor)

                Text("Leftovers Cleaned")
                    .font(.title2)
                    .fontWeight(.bold)

                VStack(spacing: 8) {
                    HStack {
                        Text("App:")
                        Spacer()
                        Text(result.appName)
                            .fontWeight(.semibold)
                    }
                    HStack {
                        Text("Items removed:")
                        Spacer()
                        Text("\(result.filesRemoved)")
                            .fontWeight(.semibold)
                    }
                    HStack {
                        Text("Space freed:")
                        Spacer()
                        Text(result.bytesFreedFormatted)
                            .fontWeight(.semibold)
                            .foregroundStyle(result.bytesFreed > 0 ? .green : .secondary)
                    }
                }
                .padding()
                .background(Color.primary.opacity(0.04), in: RoundedRectangle(cornerRadius: 8))

                if !result.skipped.isEmpty {
                    VStack(alignment: .leading, spacing: 6) {
                        HStack(spacing: 6) {
                            Image(systemName: "lock.shield")
                                .foregroundStyle(.blue)
                            Text("Protected by macOS")
                                .font(.subheadline)
                                .fontWeight(.semibold)
                        }
                        Text("These items require Full Disk Access permission. Go to **System Settings > Privacy & Security > Full Disk Access** and add TidyMac.")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                        ForEach(result.skipped, id: \.self) { item in
                            Text("• \(item)")
                                .font(.caption2)
                                .foregroundStyle(.blue)
                        }

                        Button("Open System Settings") {
                            if let url = URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles") {
                                NSWorkspace.shared.open(url)
                            }
                        }
                        .buttonStyle(.bordered)
                        .controlSize(.small)
                    }
                    .padding(10)
                    .background(.blue.opacity(0.06), in: RoundedRectangle(cornerRadius: 8))
                }

                if !result.errors.isEmpty {
                    VStack(alignment: .leading, spacing: 4) {
                        Text("Errors:")
                            .font(.caption)
                            .fontWeight(.semibold)
                        ForEach(result.errors, id: \.self) { error in
                            Text("• \(error)")
                                .font(.caption2)
                                .foregroundStyle(.orange)
                        }
                    }
                }
            }

            Button("Done") { dismiss() }
                .buttonStyle(.borderedProminent)
        }
        .padding(32)
        .frame(width: 480)
    }
}
