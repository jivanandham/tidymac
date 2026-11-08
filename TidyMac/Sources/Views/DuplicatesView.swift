import SwiftUI

/// Find duplicate files and remove extra copies (always keeping one).
struct DuplicatesView: View {
    @ObservedObject var viewModel: AppViewModel
    @State private var perceptual = false
    @State private var selected: Set<String> = []
    @State private var confirm = false

    private var allGroups: [DupGroup] {
        (viewModel.dupResult?.exactGroups ?? []) + (viewModel.dupResult?.similarGroups ?? [])
    }

    /// Selected paths, excluding any group where every copy is selected.
    private var removablePaths: [String] {
        allGroups.flatMap { g -> [String] in
            let chosen = g.members.filter { selected.contains($0.path) }.map { $0.path }
            return chosen.count >= g.members.count ? [] : chosen
        }
    }

    private var groupsFullySelected: Int {
        allGroups.filter { g in g.members.allSatisfy { selected.contains($0.path) } }.count
    }

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Duplicates").font(.largeTitle).fontWeight(.bold)
                    Text(viewModel.dupFolder.map { ($0.path as NSString).abbreviatingWithTildeInPath } ?? "Choose a folder")
                        .foregroundStyle(.secondary)
                }
                Spacer()
                Toggle("Similar photos", isOn: $perceptual)
                    .help("Also group visually similar images (slower). Similar images are never pre-selected.")
                Button("Choose Folder…") { viewModel.chooseDuplicateFolder() }
                Button {
                    selected = []
                    viewModel.findDuplicates(perceptual: perceptual)
                } label: {
                    Label("Find", systemImage: "doc.on.doc")
                }
                .buttonStyle(.borderedProminent)
                .disabled(viewModel.isFindingDuplicates || viewModel.dupFolder == nil)
            }
            .padding()

            Divider()

            if viewModel.isFindingDuplicates {
                Spacer()
                ProgressView("Comparing files (size → quick hash → SHA-256)…")
                Button("Cancel") { viewModel.cancelScan() }.padding(.top, 8)
                Spacer()
            } else if let result = viewModel.dupResult {
                if let error = result.error {
                    Spacer()
                    Text(error).foregroundStyle(.secondary)
                    Spacer()
                } else if allGroups.isEmpty {
                    Spacer()
                    ContentUnavailableView("No duplicates", systemImage: "checkmark.seal",
                                           description: Text("Scanned \(result.filesScanned) files."))
                    Spacer()
                } else {
                    HStack {
                        Text("\(allGroups.count) groups • \(result.totalWastedFormatted) in extra copies (APFS clones counted by their unique bytes)")
                            .font(.callout).foregroundStyle(.secondary)
                        Spacer()
                        Button("Select extra copies") {
                            selected = Set((viewModel.dupResult?.exactGroups ?? []).flatMap { $0.members.dropFirst().map { $0.path } })
                        }
                        .controlSize(.small)
                        Button("Clear") { selected = [] }.controlSize(.small)
                        Button {
                            confirm = true
                        } label: {
                            Label("Remove \(removablePaths.count) Selected", systemImage: "trash")
                        }
                        .buttonStyle(.borderedProminent)
                        .tint(TidyTheme.amber)
                        .disabled(removablePaths.isEmpty)
                    }
                    .padding(.horizontal)
                    .padding(.vertical, 8)

                    if groupsFullySelected > 0 {
                        Label("\(groupsFullySelected) group(s) have every copy selected — those are skipped so one copy is always kept.",
                              systemImage: "exclamationmark.triangle")
                            .font(.caption).foregroundStyle(TidyTheme.amber)
                            .padding(.horizontal)
                    }

                    List(allGroups) { group in
                        Section {
                            ForEach(group.members) { m in
                                HStack {
                                    Toggle("", isOn: Binding(
                                        get: { selected.contains(m.path) },
                                        set: { on in if on { selected.insert(m.path) } else { selected.remove(m.path) } }
                                    ))
                                    .labelsHidden()
                                    VStack(alignment: .leading, spacing: 1) {
                                        Text((m.path as NSString).lastPathComponent)
                                        Text((m.path as NSString).abbreviatingWithTildeInPath)
                                            .font(.caption2).foregroundStyle(.tertiary)
                                            .lineLimit(1).truncationMode(.middle)
                                    }
                                    Spacer()
                                    if m.similarity < 1 {
                                        Text("\(Int(m.similarity * 100))% similar").font(.caption2).foregroundStyle(.secondary)
                                    }
                                    Text(m.sizeFormatted).font(.caption)
                                    Button {
                                        NSWorkspace.shared.activateFileViewerSelecting([URL(fileURLWithPath: m.path)])
                                    } label: { Image(systemName: "magnifyingglass") }
                                        .buttonStyle(.plain)
                                }
                            }
                        } header: {
                            HStack {
                                Text(group.matchType)
                                Spacer()
                                Text("\(group.wastedFormatted) extra")
                            }
                        }
                    }
                    .listStyle(.inset(alternatesRowBackgrounds: true))
                }
            } else {
                Spacer()
                ContentUnavailableView("Find duplicate files", systemImage: "doc.on.doc",
                                       description: Text("Choose a folder and click Find. Nothing is removed until you select copies and confirm."))
                Spacer()
            }
        }
        .alert("Remove \(removablePaths.count) duplicate copies?", isPresented: $confirm) {
            Button("Cancel", role: .cancel) {}
            Button("Move to Undo Area") {
                let paths = removablePaths
                viewModel.removeFiles(paths, label: "duplicates") {
                    selected = []
                    viewModel.findDuplicates(perceptual: perceptual)
                }
            }
        } message: {
            Text("At least one copy of every file is kept. Removed copies can be restored from History for 7 days.")
        }
        .sheet(isPresented: $viewModel.showActionResult) {
            CleanResultSheet(result: viewModel.lastActionResult)
        }
    }
}
