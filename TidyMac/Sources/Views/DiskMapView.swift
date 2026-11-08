import SwiftUI

/// Disk map (squarified treemap) plus the "System Data" explainer.
struct DiskMapView: View {
    @ObservedObject var viewModel: AppViewModel
    @State private var hovered: String?
    @State private var showSystemData = false

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Disk Map").font(.largeTitle).fontWeight(.bold)
                    Text("Click a folder to look inside. Sizes count APFS clones and hard links once.")
                        .foregroundStyle(.secondary)
                }
                Spacer()
                Button {
                    viewModel.drillUp()
                } label: {
                    Label("Up", systemImage: "chevron.up")
                }
                .disabled(viewModel.diskMapPath.isEmpty || viewModel.isLoadingDiskTree)

                Button {
                    viewModel.diskMapPath = []
                    viewModel.loadDiskTree(path: nil)
                } label: {
                    Label("Home", systemImage: "house")
                }
                .disabled(viewModel.isLoadingDiskTree)

                Button {
                    let panel = NSOpenPanel()
                    panel.canChooseDirectories = true
                    panel.canChooseFiles = false
                    if panel.runModal() == .OK, let url = panel.url {
                        viewModel.diskMapPath = []
                        viewModel.loadDiskTree(path: url.path)
                    }
                } label: {
                    Label("Choose Folder…", systemImage: "folder")
                }
                .disabled(viewModel.isLoadingDiskTree)

                Button {
                    showSystemData = true
                    viewModel.loadSystemData()
                } label: {
                    Label("What's in System Data?", systemImage: "questionmark.circle")
                }
            }
            .padding()

            Divider()

            if viewModel.isLoadingDiskTree {
                Spacer()
                ProgressView("Measuring folders…")
                Button("Cancel") { viewModel.cancelScan() }.padding(.top, 8)
                Spacer()
            } else if let tree = viewModel.diskTree {
                HStack(spacing: 0) {
                    VStack(alignment: .leading, spacing: 6) {
                        Text((tree.path as NSString).abbreviatingWithTildeInPath)
                            .font(.headline).lineLimit(1).truncationMode(.middle)
                        Text(ByteCountFormatter.string(fromByteCount: Int64(tree.size), countStyle: .file))
                            .foregroundStyle(.secondary)
                        TreemapView(nodes: tree.children, hovered: $hovered) { node in
                            viewModel.drillDown(into: node)
                        }
                    }
                    .padding()

                    Divider()

                    List(tree.children) { node in
                        HStack {
                            Image(systemName: node.isOther ? "ellipsis.circle" : (node.isDir ? "folder.fill" : "doc"))
                                .foregroundStyle(node.isDir ? TidyTheme.sapphire : .secondary)
                            VStack(alignment: .leading) {
                                Text(node.name).lineLimit(1)
                                if node.reclaimable + 1_048_576 < node.size {
                                    Text("~\(ByteCountFormatter.string(fromByteCount: Int64(node.reclaimable), countStyle: .file)) unique (rest shared)")
                                        .font(.caption2).foregroundStyle(.secondary)
                                }
                            }
                            Spacer()
                            Text(ByteCountFormatter.string(fromByteCount: Int64(node.size), countStyle: .file))
                                .font(.callout).foregroundStyle(.secondary)
                        }
                        .contentShape(Rectangle())
                        .onTapGesture { viewModel.drillDown(into: node) }
                        .contextMenu {
                            Button("Show in Finder") {
                                NSWorkspace.shared.activateFileViewerSelecting([URL(fileURLWithPath: node.path)])
                            }
                        }
                        .background(hovered == node.id ? TidyTheme.sapphire.opacity(0.08) : .clear)
                    }
                    .frame(width: 300)
                }
            } else {
                Spacer()
                ContentUnavailableView(
                    "Map your disk",
                    systemImage: "square.grid.3x3.square",
                    description: Text("See which folders take up space.")
                )
                Button("Map Home Folder") { viewModel.loadDiskTree(path: nil) }
                    .buttonStyle(.borderedProminent)
                Spacer()
            }
        }
        .sheet(isPresented: $showSystemData) {
            SystemDataSheet(viewModel: viewModel)
        }
    }
}

// MARK: - Treemap

struct TreemapTile: Identifiable {
    var id: String { node.id }
    let node: DiskNode
    let rect: CGRect
}

/// Squarified treemap layout (Bruls, Huizing & van Wijk).
enum Treemap {
    static func layout(_ nodes: [DiskNode], in bounds: CGRect) -> [TreemapTile] {
        let items = nodes.filter { $0.size > 0 }.sorted { $0.size > $1.size }
        let total = items.reduce(0.0) { $0 + Double($1.size) }
        guard total > 0, bounds.width > 0, bounds.height > 0 else { return [] }
        let scale = Double(bounds.width * bounds.height) / total
        var remaining: [(DiskNode, Double)] = items.map { ($0, Double($0.size) * scale) }
        var rect = bounds
        var tiles: [TreemapTile] = []

        while !remaining.isEmpty {
            let side = Double(min(rect.width, rect.height))
            var row: [(DiskNode, Double)] = []
            var index = 0
            while index < remaining.count {
                let candidate = row + [remaining[index]]
                if row.isEmpty || worst(candidate, side) <= worst(row, side) {
                    row = candidate
                    index += 1
                } else {
                    break
                }
            }
            let rowArea = row.reduce(0.0) { $0 + $1.1 }
            if rect.width >= rect.height {
                let w = CGFloat(rowArea / Double(rect.height))
                var y = rect.minY
                for (node, area) in row {
                    let h = CGFloat(area) / max(w, 0.0001)
                    tiles.append(TreemapTile(node: node, rect: CGRect(x: rect.minX, y: y, width: w, height: h)))
                    y += h
                }
                rect = CGRect(x: rect.minX + w, y: rect.minY, width: max(0, rect.width - w), height: rect.height)
            } else {
                let h = CGFloat(rowArea / Double(rect.width))
                var x = rect.minX
                for (node, area) in row {
                    let w = CGFloat(area) / max(h, 0.0001)
                    tiles.append(TreemapTile(node: node, rect: CGRect(x: x, y: rect.minY, width: w, height: h)))
                    x += w
                }
                rect = CGRect(x: rect.minX, y: rect.minY + h, width: rect.width, height: max(0, rect.height - h))
            }
            remaining.removeFirst(row.count)
        }
        return tiles
    }

    /// Worst aspect ratio in a row laid along a side of length `side`.
    static func worst(_ row: [(DiskNode, Double)], _ side: Double) -> Double {
        let areas = row.map { $0.1 }
        guard let maxA = areas.max(), let minA = areas.min(), minA > 0, side > 0 else { return .infinity }
        let sum = areas.reduce(0, +)
        let s2 = sum * sum
        let w2 = side * side
        return max(w2 * maxA / s2, s2 / (w2 * minA))
    }
}

struct TreemapView: View {
    let nodes: [DiskNode]
    @Binding var hovered: String?
    let onOpen: (DiskNode) -> Void

    private let palette: [Color] = [TidyTheme.sapphire, TidyTheme.teal, TidyTheme.emerald, TidyTheme.lavender, TidyTheme.amber, TidyTheme.coral]

    var body: some View {
        GeometryReader { geo in
            let tiles = Treemap.layout(nodes, in: CGRect(origin: .zero, size: geo.size))
            ZStack(alignment: .topLeading) {
                ForEach(Array(tiles.enumerated()), id: \.element.id) { idx, tile in
                    let color = tile.node.isOther ? Color.gray : palette[idx % palette.count]
                    RoundedRectangle(cornerRadius: 4)
                        .fill(color.opacity(hovered == tile.id ? 0.85 : 0.6))
                        .overlay(alignment: .topLeading) {
                            if tile.rect.width > 70 && tile.rect.height > 34 {
                                VStack(alignment: .leading, spacing: 1) {
                                    Text(tile.node.name).font(.caption).fontWeight(.semibold).lineLimit(1)
                                    Text(ByteCountFormatter.string(fromByteCount: Int64(tile.node.size), countStyle: .file))
                                        .font(.caption2)
                                }
                                .foregroundStyle(.white)
                                .padding(5)
                            }
                        }
                        .frame(width: max(0, tile.rect.width - 2), height: max(0, tile.rect.height - 2))
                        .offset(x: tile.rect.minX + 1, y: tile.rect.minY + 1)
                        .onHover { inside in hovered = inside ? tile.id : (hovered == tile.id ? nil : hovered) }
                        .onTapGesture { onOpen(tile.node) }
                        .help("\(tile.node.name) — \(ByteCountFormatter.string(fromByteCount: Int64(tile.node.size), countStyle: .file))")
                        .contextMenu {
                            Button("Show in Finder") {
                                NSWorkspace.shared.activateFileViewerSelecting([URL(fileURLWithPath: tile.node.path)])
                            }
                        }
                }
            }
        }
        .frame(minWidth: 300, minHeight: 300)
    }
}

// MARK: - System Data

struct SystemDataSheet: View {
    @ObservedObject var viewModel: AppViewModel
    @Environment(\.dismiss) var dismiss

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("What's in System Data").font(.title2).fontWeight(.bold)
            Text("macOS groups these under System Data in Storage settings. Free space available right now: \(viewModel.freeSpaceFormatted).")
                .font(.callout).foregroundStyle(.secondary)

            if viewModel.isLoadingSystemData {
                Spacer()
                ProgressView("Measuring…").frame(maxWidth: .infinity)
                Spacer()
            } else if let info = viewModel.systemData {
                List(info.entries) { e in
                    VStack(alignment: .leading, spacing: 3) {
                        HStack {
                            Text(e.name).fontWeight(.semibold)
                            if e.tidymacCanClean {
                                Text("TidyMac can help").font(.caption2)
                                    .padding(.horizontal, 6).padding(.vertical, 2)
                                    .background(TidyTheme.emerald.opacity(0.15), in: Capsule())
                                    .foregroundStyle(TidyTheme.emerald)
                            }
                            Spacer()
                            Text(e.size.map { ByteCountFormatter.string(fromByteCount: Int64($0), countStyle: .file) } ?? "—")
                                .foregroundStyle(.secondary)
                        }
                        Text(e.description).font(.caption).foregroundStyle(.secondary)
                        Text(e.advice).font(.caption)
                    }
                    .padding(.vertical, 3)
                }
                ForEach(info.notes, id: \.self) { n in
                    Label(n, systemImage: "info.circle").font(.caption).foregroundStyle(.secondary)
                }
            }

            HStack {
                Spacer()
                Button("Done") { dismiss() }.buttonStyle(.borderedProminent)
            }
        }
        .padding(24)
        .frame(width: 620, height: 600)
    }
}
