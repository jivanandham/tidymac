import SwiftUI

enum SidebarItem: String, CaseIterable, Identifiable {
    case dashboard = "Dashboard"
    case scan = "Scan & Clean"
    case apps = "Applications"
    case privacy = "Privacy"
    case docker = "Docker"
    case history = "History"

    var id: String { rawValue }

    var icon: String {
        switch self {
        case .dashboard: return "gauge.with.dots.needle.33percent"
        case .scan: return "magnifyingglass"
        case .apps: return "app.badge"
        case .privacy: return "lock.shield"
        case .docker: return "shippingbox"
        case .history: return "clock.arrow.circlepath"
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
        .navigationSplitViewColumnWidth(min: 180, ideal: 200, max: 240)
        .onAppear {
            viewModel.loadDiskUsage()
        }
    }
}

struct SidebarView: View {
    @Binding var selection: SidebarItem

    var body: some View {
        List(SidebarItem.allCases, selection: $selection) { item in
            Label(item.rawValue, systemImage: item.icon)
                .tag(item)
        }
        .listStyle(.sidebar)
        .safeAreaInset(edge: .bottom) {
            VStack(spacing: 4) {
                Divider()
                HStack {
                    Image(systemName: "leaf.fill")
                        .foregroundStyle(.green)
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
