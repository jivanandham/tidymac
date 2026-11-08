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
                .accessibilityLabel("Refresh Application List")
            }
            .padding()

            Divider()

            if !viewModel.hasFullDiskAccess {
                PermissionBanner(viewModel: viewModel)
            }

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
        .accessibilityElement(children: .combine)
        .accessibilityLabel("\(app.name), \(app.totalSizeFormatted)\(app.leftoversSize > 0 ? ", including \(app.leftoversFormatted) leftovers" : "")")
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
    @State private var showCacheConfirm = false
    @State private var showUninstall = false

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 20) {
                HStack(spacing: 16) {
                    AppIconView(name: app.name)
                        .frame(width: 56, height: 56)

                    VStack(alignment: .leading, spacing: 4) {
                        Text(app.name).font(.title2).fontWeight(.bold)
                        if let bid = app.bundleId {
                            Text(bid).font(.caption).foregroundStyle(.secondary).textSelection(.enabled)
                        }
                        if let version = app.version {
                            Text("Version \(version)").font(.caption).foregroundStyle(.secondary)
                        }
                    }

                    Spacer()

                    if app.isApple {
                        Label("Part of macOS", systemImage: "applelogo")
                            .font(.caption).foregroundStyle(.secondary)
                    } else {
                        VStack(alignment: .trailing, spacing: 6) {
                            if app.regenerableSize > 0 {
                                Button {
                                    showCacheConfirm = true
                                } label: {
                                    Label("Clear Caches", systemImage: "sparkles")
                                }
                                .buttonStyle(.bordered)
                                .disabled(viewModel.isCleaningApp)
                                .help("Removes caches, logs and saved window state. Settings and documents are kept.")
                            }
                            Button {
                                viewModel.loadInstallInfo(app: app)
                                showUninstall = true
                            } label: {
                                Label("Uninstall…", systemImage: "trash")
                            }
                            .buttonStyle(.borderedProminent)
                            .tint(.red)
                            .disabled(viewModel.isCleaningApp)
                        }
                    }
                }

                Divider()

                VStack(alignment: .leading, spacing: 12) {
                    Text("Size Breakdown").font(.headline)
                    HStack(spacing: 24) {
                        stat(app.appSizeFormatted, "App Bundle", .blue)
                        stat(app.leftoversFormatted, "App Data", .orange)
                        stat(app.totalSizeFormatted, "Total", .primary)
                    }
                }

                if !app.associatedFiles.isEmpty {
                    Divider()
                    VStack(alignment: .leading, spacing: 8) {
                        Text("App Data").font(.headline)
                        ForEach(app.associatedFiles) { file in
                            AssociatedFileRow(file: file)
                        }
                    }
                }

                Divider()
                VStack(alignment: .leading, spacing: 4) {
                    Text("Location").font(.headline)
                    Text(app.path).font(.caption).foregroundStyle(.secondary).textSelection(.enabled)
                }
            }
            .padding()
        }
        .alert("Clear caches of \(app.name)?", isPresented: $showCacheConfirm) {
            Button("Cancel", role: .cancel) {}
            Button("Clear Caches") { viewModel.cleanAppCaches(app: app) }
        } message: {
            Text("Removes caches, logs, saved window state and crash reports (\(ByteCountFormatter.string(fromByteCount: Int64(app.regenerableSize), countStyle: .file))). The app rebuilds them. Settings, documents and the app itself are kept. Items in use by the running app are skipped.")
        }
        .sheet(isPresented: $showUninstall) {
            UninstallSheet(app: app, viewModel: viewModel, info: viewModel.installInfo[app.path])
        }
        .sheet(isPresented: $viewModel.showAppCleanResult) {
            AppCleanResultSheet(result: viewModel.appCleanResult)
        }
    }

    private func stat(_ value: String, _ label: String, _ color: Color) -> some View {
        VStack {
            Text(value).font(.title3).fontWeight(.bold).foregroundStyle(color)
            Text(label).font(.caption).foregroundStyle(.secondary)
        }
    }
}

struct AssociatedFileRow: View {
    let file: AssociatedFileInfo

    var body: some View {
        HStack {
            Image(systemName: iconForKind(file.kind))
                .foregroundStyle(.secondary)
                .frame(width: 20)
            VStack(alignment: .leading, spacing: 1) {
                HStack(spacing: 6) {
                    Text(file.kind).font(.subheadline)
                    if file.needsConfirmation {
                        Text("name match").font(.caption2)
                            .padding(.horizontal, 5).padding(.vertical, 1)
                            .background(.orange.opacity(0.15), in: Capsule())
                            .foregroundStyle(.orange)
                    }
                    if file.regenerable {
                        Text("rebuildable").font(.caption2).foregroundStyle(.secondary)
                    }
                }
                Text(file.path).font(.caption2).foregroundStyle(.tertiary).lineLimit(1).truncationMode(.middle)
            }
            Spacer()
            Text(file.sizeFormatted).font(.caption).foregroundStyle(.secondary)
        }
        .padding(.vertical, 2)
    }
}

func iconForKind(_ kind: String) -> String {
    switch kind {
    case "Cache": return "folder"
    case "App Support": return "doc.fill"
    case "Preferences": return "gearshape"
    case "Saved State": return "bookmark.fill"
    case "Container", "Group Container": return "shippingbox"
    case "Logs", "Crash Reports": return "doc.text"
    case "Cookies", "HTTP Storage", "WebKit Data": return "globe"
    case "Launch Agent": return "bolt.circle"
    default: return "doc"
    }
}

/// Review exactly what an uninstall removes before doing it.
struct UninstallSheet: View {
    let app: AppInfo
    @ObservedObject var viewModel: AppViewModel
    let info: InstallInfoResult?
    @Environment(\.dismiss) var dismiss
    @State private var selected: Set<String> = []
    @State private var initialized = false

    private var allFiles: [AssociatedFileInfo] {
        var files = app.associatedFiles
        let known = Set(files.map { $0.path })
        files += (viewModel.installInfo[app.path]?.brewZap ?? []).filter { !known.contains($0.path) }
        return files
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("Uninstall \(app.name)").font(.title2).fontWeight(.bold)
            Text("Everything below is moved to the undo area; you can restore it from History for 7 days. Items matched only by the app's name are unticked — tick them if they really belong to \(app.name).")
                .font(.callout).foregroundStyle(.secondary)

            List {
                Section("Application") {
                    HStack {
                        Image(systemName: "app").frame(width: 20)
                        Text(app.path).font(.caption).lineLimit(1).truncationMode(.middle)
                        Spacer()
                        Text(app.appSizeFormatted).font(.caption)
                    }
                }
                Section("App data") {
                    ForEach(allFiles) { file in
                        Toggle(isOn: Binding(
                            get: { selected.contains(file.path) },
                            set: { on in if on { selected.insert(file.path) } else { selected.remove(file.path) } }
                        )) {
                            AssociatedFileRow(file: file)
                        }
                    }
                }
                if let info = viewModel.installInfo[app.path], info.brewCask != nil || !info.receipts.isEmpty {
                    Section("Installer") {
                        if let cask = info.brewCask {
                            Text("Installed with Homebrew cask '\(cask)'. Its zap list is included above. Consider also running `brew uninstall --cask \(cask)` so Homebrew forgets it.")
                                .font(.caption)
                        }
                        ForEach(info.receipts) { r in
                            VStack(alignment: .leading, spacing: 2) {
                                Text("Package receipt \(r.pkgId) (\(r.fileCount) files)").font(.caption)
                                Text("Files outside your home folder need admin rights. To forget the receipt: \(r.forgetCommand)")
                                    .font(.caption2).foregroundStyle(.secondary).textSelection(.enabled)
                            }
                        }
                    }
                }
            }
            .frame(minHeight: 280)

            HStack {
                Button("Cancel", role: .cancel) { dismiss() }
                Spacer()
                Button("Uninstall", role: .destructive) {
                    let includeNames = allFiles.contains { $0.needsConfirmation && selected.contains($0.path) }
                    viewModel.uninstallApp(app: app, includeNameMatches: includeNames, onlyPaths: Array(selected))
                    dismiss()
                }
                .buttonStyle(.borderedProminent)
                .tint(.red)
            }
        }
        .padding(24)
        .frame(width: 620, height: 560)
        .onAppear(perform: initialize)
        .onChange(of: viewModel.installInfo[app.path]?.brewZap.count ?? 0) { _, _ in
            for f in allFiles where !f.needsConfirmation { selected.insert(f.path) }
        }
    }

    private func initialize() {
        guard !initialized else { return }
        initialized = true
        selected = Set(allFiles.filter { !$0.needsConfirmation }.map { $0.path })
    }
}

struct AppCleanResultSheet: View {
    let result: AppCleanResult?
    @Environment(\.dismiss) var dismiss

    var body: some View {
        VStack(spacing: 16) {
            if let result = result {
                if let error = result.error {
                    Image(systemName: "xmark.octagon.fill").font(.system(size: 44)).foregroundStyle(.red)
                    Text("Nothing was removed").font(.title2).fontWeight(.bold)
                    Text(error).foregroundStyle(.secondary).multilineTextAlignment(.center)
                } else {
                    Image(systemName: result.errors.isEmpty ? "checkmark.circle.fill" : "exclamationmark.triangle.fill")
                        .font(.system(size: 44))
                        .foregroundStyle(result.errors.isEmpty ? .green : .orange)
                    Text(result.appName).font(.title2).fontWeight(.bold)

                    VStack(spacing: 8) {
                        HStack { Text("Items removed:"); Spacer(); Text("\(result.filesRemoved)").fontWeight(.semibold) }
                        if result.sessionId != nil {
                            HStack { Text("Moved to undo area:"); Spacer(); Text(result.bytesStagedFormatted).fontWeight(.semibold) }
                            Text("Restore it from History. Space is freed when the session expires or you empty the undo area.")
                                .font(.caption).foregroundStyle(.secondary)
                        } else {
                            HStack { Text("Space freed:"); Spacer(); Text(result.bytesFreedFormatted).fontWeight(.semibold) }
                        }
                    }
                    .padding()
                    .background(Color.primary.opacity(0.04), in: RoundedRectangle(cornerRadius: 8))

                    ScrollView {
                        VStack(alignment: .leading, spacing: 4) {
                            if !result.skipped.isEmpty {
                                Text("Skipped").font(.caption).fontWeight(.semibold)
                                ForEach(result.skipped, id: \.self) { Text("• \($0)").font(.caption2).foregroundStyle(.secondary) }
                                if result.skipped.contains(where: { $0.contains("privacy") || $0.contains("Container") }) {
                                    Button("Open Full Disk Access Settings") {
                                        if let url = URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles") {
                                            NSWorkspace.shared.open(url)
                                        }
                                    }
                                    .controlSize(.small)
                                }
                            }
                            if !result.errors.isEmpty {
                                Text("Errors").font(.caption).fontWeight(.semibold)
                                ForEach(result.errors, id: \.self) { Text("• \($0)").font(.caption2).foregroundStyle(.orange) }
                            }
                        }
                        .frame(maxWidth: .infinity, alignment: .leading)
                    }
                    .frame(maxHeight: 180)
                }
            }

            Button("Done") { dismiss() }.buttonStyle(.borderedProminent)
        }
        .padding(28)
        .frame(width: 480)
    }
}
