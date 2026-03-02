import SwiftUI

enum SidebarItem: String, CaseIterable, Identifiable {
    case dashboard = "Dashboard"
    case scan = "Scan & Clean"
    case apps = "Applications"
    case startup = "Startup Items"
    case privacy = "Privacy"
    case docker = "Docker"
    case history = "History"

    var id: String { rawValue }

    var icon: String {
        switch self {
        case .dashboard: return "gauge.with.dots.needle.33percent"
        case .scan: return "magnifyingglass"
        case .apps: return "app.badge"
        case .startup: return "bolt.circle"
        case .privacy: return "lock.shield"
        case .docker: return "shippingbox"
        case .history: return "clock.arrow.circlepath"
        }
    }

    var accentColor: Color {
        switch self {
        case .dashboard: return TidyTheme.emerald
        case .scan: return TidyTheme.sapphire
        case .apps: return TidyTheme.lavender
        case .startup: return TidyTheme.amber
        case .privacy: return TidyTheme.teal
        case .docker: return TidyTheme.coral
        case .history: return .secondary
        }
    }
}

struct ContentView: View {
    @State private var selectedItem: SidebarItem = .dashboard
    @StateObject private var viewModel = AppViewModel()

    var body: some View {
        NavigationSplitView {
            SidebarView(selection: $selectedItem)
        } detail: {
            Group {
                switch selectedItem {
                case .dashboard:
                    DashboardView(viewModel: viewModel)
                case .scan:
                    ScanView(viewModel: viewModel)
                case .apps:
                    AppsView(viewModel: viewModel)
                case .startup:
                    StartupView(viewModel: viewModel)
                case .privacy:
                    PrivacyView(viewModel: viewModel)
                case .docker:
                    DockerView(viewModel: viewModel)
                case .history:
                    HistoryView(viewModel: viewModel)
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .navigationSplitViewColumnWidth(min: 190, ideal: 210, max: 250)
        .onAppear {
            viewModel.loadDiskUsage()
        }
    }
}

struct SidebarView: View {
    @Binding var selection: SidebarItem
    @State private var hoveredItem: SidebarItem?

    var body: some View {
        List(SidebarItem.allCases, selection: $selection) { item in
            HStack(spacing: 10) {
                Image(systemName: item.icon)
                    .foregroundStyle(selection == item ? item.accentColor : .secondary)
                    .frame(width: 22)
                    .font(.system(size: 14))
                Text(item.rawValue)
                    .font(.system(size: 13))
                Spacer()
                if selection == item {
                    Circle()
                        .fill(item.accentColor)
                        .frame(width: 6, height: 6)
                }
            }
            .padding(.vertical, 3)
            .tag(item)
        }
        .listStyle(.sidebar)
        .safeAreaInset(edge: .bottom) {
            VStack(spacing: 4) {
                Divider()
                HStack {
                    Image(systemName: "leaf.fill")
                        .foregroundStyle(TidyTheme.emerald)
                    Text("TidyMac")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    Spacer()
                    Text("v\(TidyMacBridge.shared.version())")
                        .font(.caption2)
                        .foregroundStyle(.tertiary)
                }
                .padding(.horizontal, 16)
                .padding(.bottom, 8)
            }
        }
    }
}
