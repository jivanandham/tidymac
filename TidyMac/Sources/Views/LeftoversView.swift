import SwiftUI

/// Leftovers of apps that are no longer installed.
struct LeftoversView: View {
    @ObservedObject var viewModel: AppViewModel
    @State private var selected: Set<String> = []
    @State private var confirm = false

    private var selectedPaths: [String] {
        viewModel.orphans.filter { selected.contains($0.bundleId) }.flatMap { $0.files.map { $0.path } }
    }

    private var selectedSize: UInt64 {
        viewModel.orphans.filter { selected.contains($0.bundleId) }.reduce(0) { $0 + $1.totalSize }
    }

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Leftovers").font(.largeTitle).fontWeight(.bold)
                    Text("Data left behind by apps that are no longer installed")
                        .foregroundStyle(.secondary)
                }
                Spacer()
                Button {
                    viewModel.loadOrphans()
                } label: {
                    Label("Scan", systemImage: "arrow.clockwise")
                }
                .disabled(viewModel.isLoadingOrphans)
                Button {
                    confirm = true
                } label: {
                    Label("Remove Selected (\(ByteCountFormatter.string(fromByteCount: Int64(selectedSize), countStyle: .file)))", systemImage: "trash")
                }
                .buttonStyle(.borderedProminent)
                .tint(TidyTheme.amber)
                .disabled(selected.isEmpty || viewModel.isLoadingOrphans)
            }
            .padding()

            Text("Only items untouched for 30+ days, whose bundle identifier matches no installed app and no app from the same vendor, are listed. Review before removing — everything goes to the undo area first.")
                .font(.caption)
                .foregroundStyle(.secondary)
                .padding(.horizontal)
                .padding(.bottom, 8)

            Divider()

            if viewModel.isLoadingOrphans {
                Spacer()
                ProgressView("Looking for leftovers…")
                Spacer()
            } else if viewModel.orphans.isEmpty {
                Spacer()
                ContentUnavailableView("No leftovers found", systemImage: "sparkles",
                                       description: Text("Click Scan to check again."))
                Spacer()
            } else {
                List(viewModel.orphans) { group in
                    DisclosureGroup {
                        ForEach(group.files) { f in AssociatedFileRow(file: f) }
                    } label: {
                        HStack {
                            Toggle("", isOn: Binding(
                                get: { selected.contains(group.bundleId) },
                                set: { on in if on { selected.insert(group.bundleId) } else { selected.remove(group.bundleId) } }
                            ))
                            .labelsHidden()
                            Text(group.bundleId).font(.body.monospaced())
                            Spacer()
                            Text("\(group.files.count) items").font(.caption).foregroundStyle(.secondary)
                            Text(group.totalSizeFormatted).fontWeight(.semibold)
                        }
                    }
                }
                .listStyle(.inset(alternatesRowBackgrounds: true))
            }
        }
        .onAppear {
            if viewModel.orphans.isEmpty && !viewModel.isLoadingOrphans {
                viewModel.loadOrphans()
            }
        }
        .alert("Remove leftovers of \(selected.count) apps?", isPresented: $confirm) {
            Button("Cancel", role: .cancel) {}
            Button("Move to Undo Area") {
                viewModel.cleanOrphans(paths: selectedPaths)
                selected = []
            }
        } message: {
            Text("They can be restored from History for 7 days.")
        }
        .sheet(isPresented: $viewModel.showActionResult) {
            CleanResultSheet(result: viewModel.lastActionResult)
        }
    }
}
