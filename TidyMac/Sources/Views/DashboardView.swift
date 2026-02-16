import SwiftUI

struct DashboardView: View {
    @ObservedObject var viewModel: AppViewModel

    var body: some View {
        ScrollView {
            VStack(spacing: 24) {
                // Header
                HStack {
                    VStack(alignment: .leading, spacing: 4) {
                        Text("Dashboard")
                            .font(.largeTitle)
                            .fontWeight(.bold)
                        Text("Your Mac at a glance")
                            .foregroundStyle(.secondary)
                    }
                    Spacer()
                    Button {
                        viewModel.loadDiskUsage()
                    } label: {
                        Label("Refresh", systemImage: "arrow.clockwise")
                    }
                }
                .padding(.horizontal)

                // Disk Usage Ring + Stats
                if let usage = viewModel.diskUsage {
                    HStack(spacing: 32) {
                        DiskRingView(usage: usage)
                            .frame(width: 200, height: 200)

                        VStack(alignment: .leading, spacing: 16) {
                            StatCard(
                                title: "Total Capacity",
                                value: usage.totalCapacityFormatted,
                                icon: "internaldrive",
                                color: .blue
                            )
                            StatCard(
                                title: "Used",
                                value: usage.usedFormatted,
                                icon: "chart.bar.fill",
                                color: .orange
                            )
                            StatCard(
                                title: "Available",
                                value: usage.availableFormatted,
                                icon: "checkmark.circle.fill",
                                color: .green
                            )
                        }

                        Spacer()
                    }
                    .padding()
                    .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 16))
                    .padding(.horizontal)

                    // Category Breakdown
                    VStack(alignment: .leading, spacing: 12) {
                        Text("Storage Breakdown")
                            .font(.headline)

                        LazyVGrid(columns: [
                            GridItem(.flexible()),
                            GridItem(.flexible()),
                        ], spacing: 12) {
                            ForEach(usage.categories) { cat in
                                CategoryCard(category: cat, totalUsed: usage.used)
                            }
                        }
                    }
                    .padding()
                    .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 16))
                    .padding(.horizontal)

                } else if viewModel.isLoadingDisk {
                    ProgressView("Analyzing disk usage...")
                        .frame(maxWidth: .infinity, minHeight: 200)
                } else {
                    ContentUnavailableView(
                        "No Data",
                        systemImage: "internaldrive",
                        description: Text("Click Refresh to analyze disk usage")
                    )
                }

                // Quick Actions
                VStack(alignment: .leading, spacing: 12) {
                    Text("Quick Actions")
                        .font(.headline)

                    HStack(spacing: 16) {
                        QuickActionButton(
                            title: "Quick Scan",
                            subtitle: "Caches & temp files",
                            icon: "hare",
                            color: .blue
                        ) {
                            viewModel.selectedProfile = "quick"
                            viewModel.runScan()
                        }

                        QuickActionButton(
                            title: "Developer Scan",
                            subtitle: "Xcode, Docker, npm...",
                            icon: "wrench.and.screwdriver",
                            color: .purple
                        ) {
                            viewModel.selectedProfile = "developer"
                            viewModel.runScan()
                        }

                        QuickActionButton(
                            title: "Deep Scan",
                            subtitle: "Everything + large files",
                            icon: "magnifyingglass",
                            color: .orange
                        ) {
                            viewModel.selectedProfile = "deep"
                            viewModel.runScan()
                        }

                        QuickActionButton(
                            title: "Privacy Audit",
                            subtitle: "Cookies & trackers",
                            icon: "lock.shield",
                            color: .green
                        ) {
                            viewModel.loadPrivacy()
                        }
                    }
                }
                .padding()
                .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 16))
                .padding(.horizontal)

                // Last Scan Summary
                if let scan = viewModel.scanResult {
                    VStack(alignment: .leading, spacing: 12) {
                        HStack {
                            Text("Last Scan Results")
                                .font(.headline)
                            Spacer()
                            Text("\(scan.profile) profile")
                                .font(.caption)
                                .padding(.horizontal, 8)
                                .padding(.vertical, 4)
                                .background(.blue.opacity(0.15), in: Capsule())
                        }

                        HStack(spacing: 24) {
                            VStack {
                                Text(scan.totalReclaimableFormatted)
                                    .font(.title2)
                                    .fontWeight(.bold)
                                    .foregroundStyle(.orange)
                                Text("Reclaimable")
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            }

                            Divider().frame(height: 40)

                            VStack {
                                Text("\(scan.totalFiles)")
                                    .font(.title2)
                                    .fontWeight(.bold)
                                Text("Files")
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            }

                            Divider().frame(height: 40)

                            VStack {
                                Text("\(scan.items.count)")
                                    .font(.title2)
                                    .fontWeight(.bold)
                                Text("Categories")
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            }

                            Divider().frame(height: 40)

                            VStack {
                                Text(String(format: "%.1fs", scan.durationSecs))
                                    .font(.title2)
                                    .fontWeight(.bold)
                                Text("Scan Time")
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            }
                        }
                        .frame(maxWidth: .infinity)
                    }
                    .padding()
                    .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 16))
                    .padding(.horizontal)
                }

                Spacer(minLength: 20)
            }
            .padding(.top)
        }
    }
}

// MARK: - Subviews

struct DiskRingView: View {
    let usage: DiskUsageResult

    var body: some View {
        ZStack {
            Circle()
                .stroke(Color.gray.opacity(0.2), lineWidth: 20)

            Circle()
                .trim(from: 0, to: CGFloat(usage.usedPercentage) / 100.0)
                .stroke(
                    AngularGradient(
                        colors: [ringColor, ringColor.opacity(0.6)],
                        center: .center,
                        startAngle: .degrees(0),
                        endAngle: .degrees(360)
                    ),
                    style: StrokeStyle(lineWidth: 20, lineCap: .round)
                )
                .rotationEffect(.degrees(-90))
                .animation(.easeInOut(duration: 1.0), value: usage.usedPercentage)

            VStack(spacing: 2) {
                Text("\(usage.usedPercentage)%")
                    .font(.system(size: 36, weight: .bold, design: .rounded))
                Text("Used")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
        .padding(10)
    }

    private var ringColor: Color {
        let pct = usage.usedPercentage
        if pct < 60 { return .green }
        if pct < 80 { return .orange }
        return .red
    }
}

struct StatCard: View {
    let title: String
    let value: String
    let icon: String
    let color: Color

    var body: some View {
        HStack(spacing: 12) {
            Image(systemName: icon)
                .font(.title3)
                .foregroundStyle(color)
                .frame(width: 28)

            VStack(alignment: .leading, spacing: 2) {
                Text(title)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                Text(value)
                    .font(.title3)
                    .fontWeight(.semibold)
            }
        }
    }
}

struct CategoryCard: View {
    let category: DiskCategory
    let totalUsed: UInt64

    var body: some View {
        HStack {
            Text(category.icon)
                .font(.title3)

            VStack(alignment: .leading, spacing: 2) {
                Text(category.name)
                    .font(.subheadline)
                    .fontWeight(.medium)
                Text(category.sizeFormatted)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            Spacer()

            if totalUsed > 0 {
                let pct = Double(category.size) / Double(totalUsed) * 100
                Text(String(format: "%.1f%%", pct))
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
        .padding(10)
        .background(Color.primary.opacity(0.04), in: RoundedRectangle(cornerRadius: 8))
    }
}

struct QuickActionButton: View {
    let title: String
    let subtitle: String
    let icon: String
    let color: Color
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            VStack(spacing: 8) {
                Image(systemName: icon)
                    .font(.title2)
                    .foregroundStyle(color)
                    .frame(width: 44, height: 44)
                    .background(color.opacity(0.12), in: Circle())

                Text(title)
                    .font(.subheadline)
                    .fontWeight(.medium)

                Text(subtitle)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
            }
            .frame(maxWidth: .infinity)
            .padding(.vertical, 12)
            .background(Color.primary.opacity(0.03), in: RoundedRectangle(cornerRadius: 12))
        }
        .buttonStyle(.plain)
    }
}
