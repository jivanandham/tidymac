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
            Button("Soft Delete (Recoverable)") {
                viewModel.runClean(mode: "soft")
            }
            Button("Permanent Delete", role: .destructive) {
                viewModel.runClean(mode: "hard")
            }
        } message: {
            Text("Clean \(viewModel.selectedFileCount) files (\(viewModel.selectedSizeFormatted)) from \(viewModel.selectedItemCount) selected categories?\n\nSoft delete moves files to staging with 7-day undo window.")
        }
        .sheet(isPresented: $viewModel.showCleanResult) {
            CleanResultSheet(result: viewModel.cleanResult)
        }
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
            // Summary bar
            HStack(spacing: 24) {
                HStack(spacing: 8) {
                    Image(systemName: "externaldrive.fill")
                        .foregroundStyle(TidyTheme.amber)
                    VStack(alignment: .leading) {
                        Text(result.totalReclaimableFormatted)
                            .font(.title3)
                            .fontWeight(.bold)
                        Text("Reclaimable")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                }

                Divider().frame(height: 36)

                HStack(spacing: 8) {
                    Image(systemName: "doc.on.doc")
                        .foregroundStyle(TidyTheme.sapphire)
                    VStack(alignment: .leading) {
                        Text("\(result.totalFiles)")
                            .font(.title3)
                            .fontWeight(.bold)
                        Text("Files")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                }

                Divider().frame(height: 36)

                HStack(spacing: 8) {
                    Image(systemName: "timer")
                        .foregroundStyle(TidyTheme.emerald)
                    VStack(alignment: .leading) {
                        Text(String(format: "%.1fs", result.durationSecs))
                            .font(.title3)
                            .fontWeight(.bold)
                        Text("Scan Time")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                }

                Spacer()

                // Select All / Deselect All
                HStack(spacing: 8) {
                    Button("Select All") {
                        viewModel.selectedItems = Set(result.items.map { $0.id })
                    }
                    .buttonStyle(.bordered)
                    .controlSize(.small)

                    Button("Safe Only") {
                        viewModel.selectedItems = Set(result.items.filter { $0.safety == "Safe" }.map { $0.id })
                    }
                    .buttonStyle(.bordered)
                    .controlSize(.small)

                    Button("None") {
                        viewModel.selectedItems.removeAll()
                    }
                    .buttonStyle(.bordered)
                    .controlSize(.small)
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
                    ProgressView()
                        .scaleEffect(0.8)
                }
            }
            .padding()
            .background(.ultraThinMaterial)

            Divider()

            // Items list
            List {
                let safeItems = result.items.filter { $0.safety == "Safe" }
                let cautionItems = result.items.filter { $0.safety != "Safe" }

                if !safeItems.isEmpty {
                    Section {
                        ForEach(safeItems) { item in
                            ScanItemRow(item: item, isSelected: viewModel.selectedItems.contains(item.id)) {
                                if viewModel.selectedItems.contains(item.id) {
                                    viewModel.selectedItems.remove(item.id)
                                } else {
                                    viewModel.selectedItems.insert(item.id)
                                }
                            }
                        }
                    } header: {
                        HStack {
                            Image(systemName: "checkmark.shield.fill")
                                .foregroundStyle(TidyTheme.emerald)
                            Text("Safe to Remove")
                                .fontWeight(.semibold)
                            Spacer()
                            let totalSafe = safeItems.reduce(UInt64(0)) { $0 + $1.sizeBytes }
                            Text(formatBytes(totalSafe))
                                .foregroundStyle(.secondary)
                        }
                    }
                }

                if !cautionItems.isEmpty {
                    Section {
                        ForEach(cautionItems) { item in
                            ScanItemRow(item: item, isSelected: viewModel.selectedItems.contains(item.id)) {
                                if viewModel.selectedItems.contains(item.id) {
                                    viewModel.selectedItems.remove(item.id)
                                } else {
                                    viewModel.selectedItems.insert(item.id)
                                }
                            }
                        }
                    } header: {
                        HStack {
                            Image(systemName: "exclamationmark.triangle.fill")
                                .foregroundStyle(TidyTheme.amber)
                            Text("Review Recommended")
                                .fontWeight(.semibold)
                            Spacer()
                            let totalCaution = cautionItems.reduce(UInt64(0)) { $0 + $1.sizeBytes }
                            Text(formatBytes(totalCaution))
                                .foregroundStyle(.secondary)
                        }
                    }
                }
            }
            .listStyle(.inset(alternatesRowBackgrounds: true))
        }
    }

    private func formatBytes(_ bytes: UInt64) -> String {
        let formatter = ByteCountFormatter()
        formatter.countStyle = .file
        return formatter.string(fromByteCount: Int64(bytes))
    }
}

struct ScanItemRow: View {
    let item: ScanItem
    let isSelected: Bool
    let onToggle: () -> Void
    @State private var isHovered = false

    var body: some View {
        HStack(spacing: 12) {
            Button(action: onToggle) {
                Image(systemName: isSelected ? "checkmark.circle.fill" : "circle")
                    .foregroundStyle(isSelected ? TidyTheme.sapphire : .secondary)
                    .font(.title3)
            }
            .buttonStyle(.plain)
            .accessibilityLabel(isSelected ? "Selected" : "Not selected")

            Image(systemName: item.icon)
                .foregroundStyle(safetyColor)
                .frame(width: 24)

            VStack(alignment: .leading, spacing: 2) {
                Text(item.name)
                    .fontWeight(.medium)
                Text(item.reason)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }

            Spacer()

            // Size bar
            GeometryReader { _ in
                RoundedRectangle(cornerRadius: 2)
                    .fill(safetyColor.opacity(0.3))
                    .frame(width: isHovered ? 60 : 50, height: 4)
            }
            .frame(width: 60, height: 4)

            VStack(alignment: .trailing, spacing: 2) {
                Text(item.sizeFormatted)
                    .fontWeight(.semibold)
                    .foregroundStyle(.primary)
                Text("\(item.fileCount) files")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            SafetyBadge(safety: item.safety)
        }
        .padding(.vertical, 4)
        .onHover { hovering in
            withAnimation(.easeInOut(duration: 0.15)) { isHovered = hovering }
        }
    }

    private var safetyColor: Color {
        switch item.safety {
        case "Safe": return TidyTheme.emerald
        case "Caution": return TidyTheme.amber
        default: return TidyTheme.coral
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
                .foregroundStyle(TidyTheme.amber)
            
            VStack(alignment: .leading, spacing: 2) {
                Text("Sandbox Access Required")
                    .fontWeight(.bold)
                Text("TidyMac is sandboxed for your security. Please grant access to your Home folder or specific directories to start cleaning.")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            }
            
            Spacer()
            
            Button("Grant Access...") {
                viewModel.requestFolderAccess()
            }
            .buttonStyle(.bordered)
            .controlSize(.regular)
        }
        .padding()
        .background(TidyTheme.amber.opacity(0.1))
        .overlay(Rectangle().frame(height: 1).foregroundStyle(TidyTheme.amber.opacity(0.2)), alignment: .bottom)
        .animation(.spring(), value: viewModel.hasFullDiskAccess)
    }
}

struct CleanResultSheet: View {
    let result: CleanResult?
    @Environment(\.dismiss) var dismiss

    var body: some View {
        VStack(spacing: 20) {
            if let result = result {
                Image(systemName: result.errors.isEmpty ? "checkmark.circle.fill" : "exclamationmark.triangle.fill")
                    .font(.system(size: 48))
                    .foregroundStyle(result.errors.isEmpty ? TidyTheme.emerald : TidyTheme.amber)

                Text("Clean Complete")
                    .font(.title2)
                    .fontWeight(.bold)

                VStack(spacing: 8) {
                    HStack {
                        Text("Files removed:")
                        Spacer()
                        Text("\(result.filesRemoved)")
                            .fontWeight(.semibold)
                    }
                    HStack {
                        Text("Space freed:")
                        Spacer()
                        Text(result.bytesFreedFormatted)
                            .fontWeight(.semibold)
                            .foregroundStyle(TidyTheme.emerald)
                    }
                    HStack {
                        Text("Mode:")
                        Spacer()
                        Text(result.mode)
                            .fontWeight(.semibold)
                    }
                    if let sid = result.sessionId {
                        HStack {
                            Text("Session:")
                            Spacer()
                            Text(sid)
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                    }
                }
                .padding()
                .background(TidyTheme.cardBackground, in: RoundedRectangle(cornerRadius: 8))

                if !result.errors.isEmpty {
                    VStack(alignment: .leading, spacing: 4) {
                        Text("Warnings:")
                            .font(.caption)
                            .fontWeight(.semibold)
                        ForEach(result.errors, id: \.self) { error in
                            Text("• \(error)")
                                .font(.caption)
                                .foregroundStyle(TidyTheme.amber)
                        }
                    }
                }
            }

            Button("Done") { dismiss() }
                .buttonStyle(.borderedProminent)
                .tint(TidyTheme.emerald)
        }
        .padding(32)
        .frame(width: 400)
    }
}
