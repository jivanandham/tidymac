import SwiftUI

struct ScanView: View {
    @ObservedObject var viewModel: AppViewModel

    var body: some View {
        VStack(spacing: 0) {
            // Toolbar
            HStack {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Scan & Clean")
                        .font(.largeTitle)
                        .fontWeight(.bold)
                    Text("Find and remove junk files safely")
                        .foregroundStyle(.secondary)
                }

                Spacer()

                Picker("Profile", selection: $viewModel.selectedProfile) {
                    Text("Quick Sweep").tag("quick")
                    Text("Developer").tag("developer")
                    Text("Creative").tag("creative")
                    Text("Deep Clean").tag("deep")
                }
                .pickerStyle(.segmented)
                .frame(width: 400)

                Button {
                    viewModel.runScan()
                } label: {
                    Label("Scan", systemImage: "magnifyingglass")
                }
                .buttonStyle(.borderedProminent)
                .tint(TidyTheme.sapphire)
                .disabled(viewModel.isScanning)
                .accessibilityLabel("Start Scan")
            }
            .padding()

            Divider()

            if !viewModel.hasFullDiskAccess {
                PermissionBanner(viewModel: viewModel)
                    .transition(.move(edge: .top).combined(with: .opacity))
            }

            if let error = viewModel.scanError {
                HStack {
                    Image(systemName: "exclamationmark.triangle.fill").foregroundStyle(TidyTheme.amber)
                    Text(error).font(.callout)
                    Spacer()
                }
                .padding(10)
                .background(TidyTheme.amber.opacity(0.08))
            }

            if viewModel.isScanning {
                Spacer()
                ScanningAnimation(profile: viewModel.selectedProfile) {
                    viewModel.cancelScan()
                }
                Spacer()
            } else if let result = viewModel.scanResult {
                ScanResultsView(viewModel: viewModel, result: result)
            } else {
                Spacer()
                ContentUnavailableView(
                    "Ready to Scan",
                    systemImage: "magnifyingglass",
                    description: Text("Select a profile and click Scan to find reclaimable space")
                )
                Spacer()
            }
        }
        .alert("Confirm Clean", isPresented: $viewModel.showCleanConfirm) {
            Button("Cancel", role: .cancel) {}
            Button("Move to Undo Area (Recoverable)") {
                viewModel.runClean(mode: "soft")
            }
            Button("Delete Permanently", role: .destructive) {
                viewModel.runClean(mode: "hard")
            }
        } message: {
            Text(confirmMessage)
        }
        .sheet(isPresented: $viewModel.showCleanResult) {
            CleanResultSheet(result: viewModel.cleanResult)
        }
        .sheet(isPresented: $viewModel.showActionResult) {
            CleanResultSheet(result: viewModel.lastActionResult)
        }
    }
}

extension ScanView {
    var confirmMessage: String {
        var text = "Clean \(viewModel.selectedFileCount) items (\(viewModel.selectedSizeFormatted)) from \(viewModel.selectedItemCount) selected categories?\n\n"
        text += "\"Move to Undo Area\" keeps them for 7 days so you can restore them from History. Disk space is freed when they expire or when you empty the undo area."
        if viewModel.selectionIncludesTools {
            text += "\n\nSome items are cleaned by their own tool (simctl, docker, go) and can't be undone."
        }
        return text
    }
}

// MARK: - Scanning Animation

struct ScanningAnimation: View {
    let profile: String
    @State private var rotation: Double = 0
    @State private var scale: CGFloat = 1.0

    var body: some View {
        VStack(spacing: 20) {
            ZStack {
                // Outer pulsing ring
                Circle()
                    .stroke(TidyTheme.scanGradient, lineWidth: 3)
                    .frame(width: 80, height: 80)
                    .scaleEffect(scale)
                    .opacity(2.0 - Double(scale))

                // Rotating scan icon
                Image(systemName: "viewfinder")
                    .font(.system(size: 36))
                    .foregroundStyle(TidyTheme.scanGradient)
                    .rotationEffect(.degrees(rotation))
            }
            .accessibilityElement(children: .ignore)
            .accessibilityLabel("Scanning animation")
            .onAppear {
                withAnimation(.linear(duration: 3).repeatForever(autoreverses: false)) {
                    rotation = 360
                }
                withAnimation(.easeInOut(duration: 1.2).repeatForever(autoreverses: true)) {
                    scale = 1.3
                }
            }

            Text("Scanning your Mac...")
                .font(.title3)
                .fontWeight(.medium)
                .foregroundStyle(.secondary)

            HStack(spacing: 6) {
                PulsingDot(color: TidyTheme.teal)
                PulsingDot(color: TidyTheme.sapphire)
                PulsingDot(color: TidyTheme.emerald)
            }

            Text("Profile: \(profile)")
                .font(.caption)
                .foregroundStyle(.tertiary)

            Button(role: .destructive, action: onCancel) {
                Label("Cancel Scan", systemImage: "xmark.circle")
            }
            .buttonStyle(.bordered)
            .padding(.top, 10)
        }
    }

    let onCancel: () -> Void
}

// MARK: - Scan Results

struct ScanResultsView: View {
    @ObservedObject var viewModel: AppViewModel
    let result: ScanResult

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 24) {
                SummaryStat(icon: "externaldrive.fill", color: TidyTheme.amber, value: result.totalReclaimableFormatted, label: "Reclaimable")
                Divider().frame(height: 36)
                SummaryStat(icon: "doc.on.doc", color: TidyTheme.sapphire, value: "\(result.totalFiles)", label: "Files")
                Divider().frame(height: 36)
                SummaryStat(icon: "timer", color: TidyTheme.emerald, value: String(format: "%.1fs", result.durationSecs), label: "Scan Time")

                Spacer()

                HStack(spacing: 8) {
                    Button("Select All") {
                        viewModel.selectedItems = Set(result.items.filter { $0.cleanable }.map { $0.id })
                    }
                    .buttonStyle(.bordered).controlSize(.small)

                    Button("Safe Only") {
                        viewModel.selectedItems = Set(result.items.filter { $0.safety == "Safe" && $0.cleanable }.map { $0.id })
                    }
                    .buttonStyle(.bordered).controlSize(.small)

                    Button("None") { viewModel.selectedItems.removeAll() }
                        .buttonStyle(.bordered).controlSize(.small)
                }

                Button {
                    viewModel.showCleanConfirm = true
                } label: {
                    Label(
                        viewModel.selectedItemCount > 0
                            ? "Clean \(viewModel.selectedItemCount) Selected (\(viewModel.selectedSizeFormatted))"
                            : "Clean Selected",
                        systemImage: "trash"
                    )
                }
                .buttonStyle(.borderedProminent)
                .tint(TidyTheme.amber)
                .disabled(viewModel.isCleaning || viewModel.selectedItems.isEmpty)

                if viewModel.isCleaning {
                    ProgressView().scaleEffect(0.8)
                }
            }
            .padding()
            .background(.ultraThinMaterial)

            Divider()

            List {
                section(title: "Safe to Remove", icon: "checkmark.shield.fill", color: TidyTheme.emerald,
                        items: result.items.filter { $0.safety == "Safe" && !$0.reviewOnly })
                section(title: "Review Recommended", icon: "exclamationmark.triangle.fill", color: TidyTheme.amber,
                        items: result.items.filter { $0.safety != "Safe" && !$0.reviewOnly })
                section(title: "Large Files — review and remove one by one", icon: "doc.zipper", color: TidyTheme.sapphire,
                        items: result.items.filter { $0.reviewOnly })

                if !result.errors.isEmpty {
                    Section("Notes") {
                        ForEach(result.errors, id: \.self) { e in
                            Text(e).font(.caption).foregroundStyle(.secondary)
                        }
                    }
                }
            }
            .listStyle(.inset(alternatesRowBackgrounds: true))
        }
    }

    @ViewBuilder
    private func section(title: String, icon: String, color: Color, items: [ScanItem]) -> some View {
        if !items.isEmpty {
            Section {
                ForEach(items) { item in
                    ScanItemRow(item: item, viewModel: viewModel)
                }
            } header: {
                HStack {
                    Image(systemName: icon).foregroundStyle(color)
                    Text(title).fontWeight(.semibold)
                    Spacer()
                    Text(formatBytes(items.reduce(UInt64(0)) { $0 + $1.sizeBytes }))
                        .foregroundStyle(.secondary)
                }
            }
        }
    }

    private func formatBytes(_ bytes: UInt64) -> String {
        ByteCountFormatter.string(fromByteCount: Int64(bytes), countStyle: .file)
    }
}

struct SummaryStat: View {
    let icon: String
    let color: Color
    let value: String
    let label: String

    var body: some View {
        HStack(spacing: 8) {
            Image(systemName: icon).foregroundStyle(color)
            VStack(alignment: .leading) {
                Text(value).font(.title3).fontWeight(.bold)
                Text(label).font(.caption).foregroundStyle(.secondary)
            }
        }
    }
}

struct ScanItemRow: View {
    let item: ScanItem
    @ObservedObject var viewModel: AppViewModel
    @State private var expanded = false

    private var isSelected: Bool { viewModel.selectedItems.contains(item.id) }

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 12) {
                if item.cleanable {
                    Button {
                        if isSelected { viewModel.selectedItems.remove(item.id) } else { viewModel.selectedItems.insert(item.id) }
                    } label: {
                        Image(systemName: isSelected ? "checkmark.circle.fill" : "circle")
                            .foregroundStyle(isSelected ? TidyTheme.sapphire : .secondary)
                            .font(.title3)
                    }
                    .buttonStyle(.plain)
                    .accessibilityLabel(isSelected ? "Selected" : "Not selected")
                } else {
                    Image(systemName: "eye").foregroundStyle(.secondary).font(.title3)
                        .help("Review only — remove items individually below")
                }

                Image(systemName: item.icon)
                    .foregroundStyle(safetyColor)
                    .frame(width: 24)

                VStack(alignment: .leading, spacing: 2) {
                    Text(item.name).fontWeight(.medium)
                    Text(item.reason)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .lineLimit(2)
                    if let tool = item.toolAction {
                        Label("Runs \(tool) — can't be undone", systemImage: "wrench.adjustable")
                            .font(.caption2)
                            .foregroundStyle(TidyTheme.amber)
                    }
                    if item.hasSharedBlocks {
                        Label("About \(item.reclaimableFormatted) will actually be freed — the rest is shared with APFS clones", systemImage: "square.on.square")
                            .font(.caption2)
                            .foregroundStyle(.secondary)
                    }
                }

                Spacer()

                VStack(alignment: .trailing, spacing: 2) {
                    Text(item.sizeFormatted).fontWeight(.semibold)
                    Text("\(item.entryCount) items").font(.caption).foregroundStyle(.secondary)
                }

                SafetyBadge(safety: item.safety)

                if !item.entries.isEmpty {
                    Button {
                        withAnimation(.easeInOut(duration: 0.15)) { expanded.toggle() }
                    } label: {
                        Image(systemName: expanded ? "chevron.down" : "chevron.right")
                    }
                    .buttonStyle(.plain)
                    .accessibilityLabel(expanded ? "Hide items" : "Show items")
                }
            }

            if expanded {
                VStack(alignment: .leading, spacing: 4) {
                    ForEach(item.entries) { entry in
                        EntryRow(entry: entry, item: item, viewModel: viewModel)
                    }
                    if item.entryCount > item.entries.count {
                        Text("… and \(item.entryCount - item.entries.count) smaller items")
                            .font(.caption2).foregroundStyle(.tertiary)
                    }
                }
                .padding(.leading, 48)
            }
        }
        .padding(.vertical, 4)
    }

    private var safetyColor: Color {
        switch item.safety {
        case "Safe": return TidyTheme.emerald
        case "Caution": return TidyTheme.amber
        default: return TidyTheme.coral
        }
    }
}

struct EntryRow: View {
    let entry: ScanEntry
    let item: ScanItem
    @ObservedObject var viewModel: AppViewModel
    @State private var confirmRemove = false

    var body: some View {
        HStack(spacing: 8) {
            if item.cleanable {
                let included = !viewModel.excludedEntries.contains(entry.path)
                Button { viewModel.toggleEntry(entry.path) } label: {
                    Image(systemName: included ? "checkmark.square" : "square")
                        .foregroundStyle(included ? TidyTheme.sapphire : .secondary)
                }
                .buttonStyle(.plain)
                .help(included ? "Will be cleaned — click to keep" : "Will be kept")
            }
            VStack(alignment: .leading, spacing: 1) {
                Text(entry.name).font(.caption)
                Text(entry.path).font(.caption2).foregroundStyle(.tertiary).lineLimit(1).truncationMode(.middle)
            }
            Spacer()
            Text(entry.sizeFormatted).font(.caption).foregroundStyle(.secondary)
            Button {
                NSWorkspace.shared.activateFileViewerSelecting([URL(fileURLWithPath: entry.path)])
            } label: { Image(systemName: "magnifyingglass") }
                .buttonStyle(.plain)
                .help("Show in Finder")
            if item.reviewOnly {
                Button("Remove…") { confirmRemove = true }
                    .buttonStyle(.bordered)
                    .controlSize(.mini)
            }
        }
        .alert("Remove \(entry.name)?", isPresented: $confirmRemove) {
            Button("Cancel", role: .cancel) {}
            Button("Move to Undo Area") {
                viewModel.removeFiles([entry.path], label: "large file")
            }
        } message: {
            Text("\(entry.sizeFormatted). You can restore it from History for 7 days.")
        }
    }
}

struct SafetyBadge: View {
    let safety: String

    var body: some View {
        Text(safety)
            .font(.caption2)
            .fontWeight(.medium)
            .padding(.horizontal, 8)
            .padding(.vertical, 3)
            .background(color.opacity(0.15), in: Capsule())
            .foregroundStyle(color)
    }

    private var color: Color {
        switch safety {
        case "Safe": return TidyTheme.emerald
        case "Caution": return TidyTheme.amber
        default: return TidyTheme.coral
        }
    }
}

// MARK: - Permission Banner

struct PermissionBanner: View {
    @ObservedObject var viewModel: AppViewModel
    
    var body: some View {
        HStack(spacing: 16) {
            Image(systemName: "lock.shield.fill")
                .font(.system(size: 24))
                .foregroundStyle(TidyTheme.sapphire)
            
            VStack(alignment: .leading, spacing: 2) {
                Text("Full Disk Access Recommended")
                    .fontWeight(.bold)
                Text("Grant Full Disk Access in macOS System Settings for deep cleaning of browser data and system logs.")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            }
            
            Spacer()
            
            Button("Open Settings") {
                viewModel.openFullDiskAccessSettings()
            }
            .buttonStyle(.borderedProminent)
            .tint(TidyTheme.sapphire)
            .controlSize(.regular)

            Button("Refresh") {
                viewModel.checkFullDiskAccess()
            }
            .buttonStyle(.bordered)
            .controlSize(.regular)
        }
        .padding()
        .background(TidyTheme.sapphire.opacity(0.08))
        .overlay(Rectangle().frame(height: 1).foregroundStyle(TidyTheme.sapphire.opacity(0.15)), alignment: .bottom)
        .animation(.spring(), value: viewModel.hasFullDiskAccess)
    }
}

struct CleanResultSheet: View {
    let result: CleanResult?
    @Environment(\.dismiss) var dismiss

    var body: some View {
        VStack(spacing: 16) {
            if let result = result {
                if let error = result.error {
                    Image(systemName: "xmark.octagon.fill").font(.system(size: 44)).foregroundStyle(TidyTheme.coral)
                    Text("Nothing was cleaned").font(.title2).fontWeight(.bold)
                    Text(error).multilineTextAlignment(.center).foregroundStyle(.secondary)
                } else {
                    let ok = result.errors.isEmpty
                    Image(systemName: ok ? "checkmark.circle.fill" : "exclamationmark.triangle.fill")
                        .font(.system(size: 44))
                        .foregroundStyle(ok ? TidyTheme.emerald : TidyTheme.amber)

                    Text(result.mode == "dry_run" ? "Dry Run" : "Clean Complete")
                        .font(.title2).fontWeight(.bold)

                    VStack(spacing: 8) {
                        row("Items:", "\(result.filesRemoved)")
                        if result.isSoftDelete {
                            row("Moved to undo area:", result.bytesStagedFormatted, color: TidyTheme.sapphire)
                            Text("Disk space is freed when the session expires or when you empty the undo area in History.")
                                .font(.caption).foregroundStyle(.secondary)
                        } else {
                            row("Space freed:", result.bytesFreedFormatted, color: TidyTheme.emerald)
                        }
                        if let sid = result.sessionId {
                            row("Session:", sid, color: .secondary)
                        }
                    }
                    .padding()
                    .background(TidyTheme.cardBackground, in: RoundedRectangle(cornerRadius: 8))

                    ScrollView {
                        VStack(alignment: .leading, spacing: 8) {
                            list("Ran (can't be undone)", result.toolActions, color: .secondary)
                            list("Warnings", result.warnings, color: TidyTheme.amber)
                            list("Errors", result.errors, color: TidyTheme.coral)
                            if !result.skipped.isEmpty {
                                Text("Skipped (\(result.skipped.count))").font(.caption).fontWeight(.semibold)
                                ForEach(result.skipped.prefix(30)) { s in
                                    Text("• \((s.path as NSString).abbreviatingWithTildeInPath) — \(s.reason)")
                                        .font(.caption2).foregroundStyle(.secondary)
                                }
                            }
                        }
                        .frame(maxWidth: .infinity, alignment: .leading)
                    }
                    .frame(maxHeight: 200)
                }
            }

            Button("Done") { dismiss() }
                .buttonStyle(.borderedProminent)
                .tint(TidyTheme.emerald)
        }
        .padding(28)
        .frame(width: 480)
    }

    private func row(_ label: String, _ value: String, color: Color = .primary) -> some View {
        HStack {
            Text(label)
            Spacer()
            Text(value).fontWeight(.semibold).foregroundStyle(color).lineLimit(1).truncationMode(.middle)
        }
    }

    @ViewBuilder
    private func list(_ title: String, _ lines: [String], color: Color) -> some View {
        if !lines.isEmpty {
            Text(title).font(.caption).fontWeight(.semibold)
            ForEach(lines, id: \.self) { l in
                Text("• \(l)").font(.caption2).foregroundStyle(color)
            }
        }
    }
}
