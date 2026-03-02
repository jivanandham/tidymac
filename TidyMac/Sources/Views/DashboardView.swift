import SwiftUI

struct DashboardView: View {
    @ObservedObject var viewModel: AppViewModel

    var body: some View {
        ScrollView {
            VStack(spacing: 24) {
                // Header with Health Score
                HStack(alignment: .top) {
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
                    .accessibilityLabel("Refresh Disk Usage")
                }
                .padding(.horizontal)

                // Disk Usage Ring + Stats
                if let usage = viewModel.diskUsage {
                    // Main stats card
                    HStack(spacing: 32) {
                        DiskRingView(usage: usage)
                            .frame(width: 200, height: 200)
                            .accessibilityLabel("Disk usage: \(usage.usedPercentage)% used")

                        VStack(alignment: .leading, spacing: 16) {
                            StatCard(
                                title: "Total Capacity",
                                value: usage.totalCapacityFormatted,
                                icon: "internaldrive",
                                color: TidyTheme.sapphire
                            )
                            StatCard(
                                title: "Used",
                                value: usage.usedFormatted,
                                icon: "chart.bar.fill",
                                color: TidyTheme.amber
                            )
                            StatCard(
                                title: "Available",
                                value: usage.availableFormatted,
                                icon: "checkmark.circle.fill",
                                color: TidyTheme.emerald
                            )
                        }

                        Spacer()

                        // Health Score
                        VStack(spacing: 8) {
                            let score = healthScore(usage)
                            ZStack {
                                Circle()
                                    .stroke(Color.gray.opacity(0.15), lineWidth: 6)
                                    .frame(width: 72, height: 72)
                                Circle()
                                    .trim(from: 0, to: CGFloat(score) / 100.0)
                                    .stroke(
                                        scoreGradient(score),
                                        style: StrokeStyle(lineWidth: 6, lineCap: .round)
                                    )
                                    .frame(width: 72, height: 72)
                                    .rotationEffect(.degrees(-90))
                                    .animation(.easeInOut(duration: 1.0), value: score)
                                Text("\(score)")
                                    .font(.system(size: 22, weight: .bold, design: .rounded))
                            }
                            Text("Health")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                            Text(healthLabel(score))
                                .font(.caption2)
                                .fontWeight(.medium)
                                .foregroundStyle(scoreColor(score))
                        }
                    }
                    .padding()
                    .glassCard()
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
                    .glassCard()
                    .padding(.horizontal)

                } else if viewModel.isLoadingDisk {
                    VStack(spacing: 16) {
                        ProgressView()
                            .scaleEffect(1.5)
                        Text("Analyzing disk usage...")
                            .foregroundStyle(.secondary)
                    }
                    .frame(maxWidth: .infinity, minHeight: 200)
                    .glassCard()
                    .padding(.horizontal)
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
                            gradient: TidyTheme.primaryGradient
                        ) {
                            viewModel.selectedProfile = "quick"
                            viewModel.runScan()
                        }

                        QuickActionButton(
                            title: "Developer Scan",
                            subtitle: "Xcode, Docker, npm...",
                            icon: "wrench.and.screwdriver",
                            gradient: TidyTheme.purpleGradient
                        ) {
                            viewModel.selectedProfile = "developer"
                            viewModel.runScan()
                        }

                        QuickActionButton(
                            title: "Deep Scan",
                            subtitle: "Everything + large files",
                            icon: "magnifyingglass",
                            gradient: TidyTheme.warningGradient
                        ) {
                            viewModel.selectedProfile = "deep"
                            viewModel.runScan()
                        }

                        QuickActionButton(
                            title: "Privacy Audit",
                            subtitle: "Cookies & trackers",
                            icon: "lock.shield",
                            gradient: TidyTheme.scanGradient
                        ) {
                            viewModel.loadPrivacy()
                        }
                    }
                }
                .padding()
                .glassCard()
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
                                .background(TidyTheme.sapphire.opacity(0.15), in: Capsule())
                                .foregroundStyle(TidyTheme.sapphire)
                        }

                        HStack(spacing: 24) {
                            VStack {
                                AnimatedCounter(
                                    value: scan.totalReclaimableFormatted,
                                    font: .title2,
                                    color: TidyTheme.amber
                                )
                                Text("Reclaimable")
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            }

                            Divider().frame(height: 40)

                            VStack {
                                AnimatedCounter(
                                    value: "\(scan.totalFiles)",
                                    font: .title2,
                                    color: .primary
                                )
                                Text("Files")
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            }

                            Divider().frame(height: 40)

                            VStack {
                                AnimatedCounter(
                                    value: "\(scan.items.count)",
                                    font: .title2,
                                    color: .primary
                                )
                                Text("Categories")
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            }

                            Divider().frame(height: 40)

                            VStack {
                                AnimatedCounter(
                                    value: String(format: "%.1fs", scan.durationSecs),
                                    font: .title2,
                                    color: TidyTheme.emerald
                                )
                                Text("Scan Time")
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            }
                        }
                        .frame(maxWidth: .infinity)
                    }
                    .padding()
                    .gradientCard(TidyTheme.warningGradient)
                    .padding(.horizontal)
                }

                Spacer(minLength: 20)
            }
            .padding(.top)
        }
    }

    // MARK: - Health Score

    private func healthScore(_ usage: DiskUsageResult) -> Int {
        let pct = usage.usedPercentage
        if pct < 50 { return 95 }
        if pct < 60 { return 85 }
        if pct < 70 { return 72 }
        if pct < 80 { return 55 }
        if pct < 90 { return 35 }
        return 15
    }

    private func healthLabel(_ score: Int) -> String {
        if score >= 80 { return "Excellent" }
        if score >= 60 { return "Good" }
        if score >= 40 { return "Fair" }
        return "Critical"
    }

    private func scoreColor(_ score: Int) -> Color {
        if score >= 80 { return TidyTheme.emerald }
        if score >= 60 { return TidyTheme.teal }
        if score >= 40 { return TidyTheme.amber }
        return TidyTheme.coral
    }

    private func scoreGradient(_ score: Int) -> LinearGradient {
        if score >= 80 { return TidyTheme.primaryGradient }
        if score >= 60 { return TidyTheme.scanGradient }
        if score >= 40 { return TidyTheme.warningGradient }
        return TidyTheme.dangerGradient
    }
}

// MARK: - Subviews

struct DiskRingView: View {
    let usage: DiskUsageResult
    @State private var animatedPct: CGFloat = 0

    var body: some View {
        ZStack {
            Circle()
                .stroke(Color.gray.opacity(0.12), lineWidth: 20)

            Circle()
                .trim(from: 0, to: animatedPct)
                .stroke(
                    TidyTheme.ringGradient(percentage: usage.usedPercentage),
                    style: StrokeStyle(lineWidth: 20, lineCap: .round)
                )
                .rotationEffect(.degrees(-90))

            VStack(spacing: 2) {
                Text("\(usage.usedPercentage)%")
                    .font(.system(size: 36, weight: .bold, design: .rounded))
                Text("Used")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
        .padding(10)
        .onAppear {
            withAnimation(.easeInOut(duration: 1.2)) {
                animatedPct = CGFloat(usage.usedPercentage) / 100.0
            }
        }
        .onChange(of: usage.usedPercentage) { _, newVal in
            withAnimation(.easeInOut(duration: 0.8)) {
                animatedPct = CGFloat(newVal) / 100.0
            }
        }
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
    @State private var isHovered = false

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
            .accessibilityElement(children: .combine)
            .accessibilityLabel("\(category.name): \(category.sizeFormatted)")

            Spacer()

            if totalUsed > 0 {
                let pct = Double(category.size) / Double(totalUsed) * 100
                Text(String(format: "%.1f%%", pct))
                    .font(.caption)
                    .fontWeight(.medium)
                    .foregroundStyle(.secondary)
            }
        }
        .padding(10)
        .background(
            RoundedRectangle(cornerRadius: 8)
                .fill(isHovered ? TidyTheme.cardBackground.opacity(2) : TidyTheme.cardBackground)
        )
        .onHover { hovering in
            withAnimation(.easeInOut(duration: 0.15)) { isHovered = hovering }
        }
    }
}

struct QuickActionButton: View {
    let title: String
    let subtitle: String
    let icon: String
    let gradient: LinearGradient
    let action: () -> Void

    @State private var isHovered = false

    var body: some View {
        Button(action: action) {
            VStack(spacing: 8) {
                Image(systemName: icon)
                    .font(.title2)
                    .foregroundStyle(gradient)
                    .frame(width: 44, height: 44)
                    .background(
                        Circle()
                            .fill(gradient.opacity(0.12))
                            .scaleEffect(isHovered ? 1.15 : 1.0)
                    )

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
            .background(
                RoundedRectangle(cornerRadius: 12)
                    .fill(isHovered ? TidyTheme.cardBackground.opacity(2) : TidyTheme.cardBackground)
            )
        }
        .buttonStyle(.plain)
        .accessibilityLabel("\(title): \(subtitle)")
        .onHover { hovering in
            withAnimation(.easeInOut(duration: 0.2)) { isHovered = hovering }
        }
    }
}
