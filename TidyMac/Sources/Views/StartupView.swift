import SwiftUI

struct StartupView: View {
    @ObservedObject var viewModel: AppViewModel

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Startup Items")
                        .font(.largeTitle)
                        .fontWeight(.bold)
                    Text("Launch agents and daemons running on your Mac")
                        .foregroundStyle(.secondary)
                }
                Spacer()
                Button {
                    viewModel.loadStartupItems()
                } label: {
                    Label("Refresh", systemImage: "arrow.clockwise")
                }
                .disabled(viewModel.isLoadingStartup)
                .accessibilityLabel("Refresh Startup Items")
            }
            .padding()

            Divider()

            if viewModel.isLoadingStartup {
                Spacer()
                VStack(spacing: 12) {
                    ProgressView()
                        .scaleEffect(1.2)
                    Text("Discovering startup items...")
                        .foregroundStyle(.secondary)
                }
                Spacer()
            } else if viewModel.startupItems.isEmpty {
                Spacer()
                ContentUnavailableView(
                    "No Startup Items",
                    systemImage: "bolt.circle",
                    description: Text("Click Refresh to discover launch agents and daemons")
                )
                Spacer()
            } else {
                // Summary bar
                HStack(spacing: 24) {
                    HStack(spacing: 8) {
                        Image(systemName: "bolt.circle.fill")
                            .foregroundStyle(TidyTheme.amber)
                        VStack(alignment: .leading) {
                            Text("\(viewModel.startupItems.count)")
                                .font(.title3)
                                .fontWeight(.bold)
                            Text("Total Items")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                    }

                    Divider().frame(height: 36)

                    HStack(spacing: 8) {
                        Image(systemName: "checkmark.circle.fill")
                            .foregroundStyle(TidyTheme.emerald)
                        VStack(alignment: .leading) {
                            let enabled = viewModel.startupItems.filter { $0.enabled }.count
                            Text("\(enabled)")
                                .font(.title3)
                                .fontWeight(.bold)
                            Text("Enabled")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                    }

                    Divider().frame(height: 36)

                    HStack(spacing: 8) {
                        Image(systemName: "xmark.circle.fill")
                            .foregroundStyle(.secondary)
                        VStack(alignment: .leading) {
                            let disabled = viewModel.startupItems.filter { !$0.enabled }.count
                            Text("\(disabled)")
                                .font(.title3)
                                .fontWeight(.bold)
                            Text("Disabled")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                    }

                    Spacer()
                }
                .padding()
                .background(.ultraThinMaterial)

                Divider()

                // Items list
                List {
                    let userAgents = viewModel.startupItems.filter { $0.kind == "User Agent" }
                    let systemAgents = viewModel.startupItems.filter { $0.kind == "System Agent" }
                    let systemDaemons = viewModel.startupItems.filter { $0.kind == "System Daemon" }

                    if !userAgents.isEmpty {
                        Section("User Agents (\(userAgents.count))") {
                            ForEach(userAgents) { item in
                                StartupItemRow(item: item) { newValue in
                                    viewModel.toggleStartupItem(label: item.label, enable: newValue)
                                }
                            }
                        }
                    }

                    if !systemAgents.isEmpty {
                        Section("System Agents (\(systemAgents.count))") {
                            ForEach(systemAgents) { item in
                                StartupItemRow(item: item) { newValue in
                                    viewModel.toggleStartupItem(label: item.label, enable: newValue)
                                }
                            }
                        }
                    }

                    if !systemDaemons.isEmpty {
                        Section("System Daemons (\(systemDaemons.count))") {
                            ForEach(systemDaemons) { item in
                                StartupItemRow(item: item) { newValue in
                                    viewModel.toggleStartupItem(label: item.label, enable: newValue)
                                }
                            }
                        }
                    }
                }
                .listStyle(.inset(alternatesRowBackgrounds: true))
            }
        }
        .onAppear {
            if viewModel.startupItems.isEmpty {
                viewModel.loadStartupItems()
            }
        }
    }
}

struct StartupItemRow: View {
    let item: StartupItemInfo
    let onToggle: (Bool) -> Void

    var body: some View {
        HStack(spacing: 12) {
            Circle()
                .fill(item.enabled ? TidyTheme.emerald : Color.gray.opacity(0.4))
                .frame(width: 10, height: 10)

            VStack(alignment: .leading, spacing: 2) {
                Text(item.label)
                    .fontWeight(.medium)
                Text(item.program ?? "Unknown path")
                    .font(.caption2)
                    .foregroundStyle(.tertiary)
                    .lineLimit(1)
                    .truncationMode(.middle)
            }

            Spacer()

            Text(item.kind)
                .font(.caption2)
                .padding(.horizontal, 8)
                .padding(.vertical, 3)
                .background(kindColor.opacity(0.12), in: Capsule())
                .foregroundStyle(kindColor)

            Toggle("", isOn: Binding(
                get: { item.enabled },
                set: { newValue in onToggle(newValue) }
            ))
            .toggleStyle(.switch)
            .labelsHidden()
            .accessibilityLabel("Enable \(item.label)")
        }
        .padding(.vertical, 4)
    }

    private var kindColor: Color {
        switch item.kind {
        case "User Agent": return TidyTheme.sapphire
        case "System Agent": return TidyTheme.amber
        case "System Daemon": return TidyTheme.coral
        default: return .secondary
        }
    }
}
