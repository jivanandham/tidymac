import SwiftUI

struct PrivacyView: View {
    @ObservedObject var viewModel: AppViewModel

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Privacy Audit")
                        .font(.largeTitle)
                        .fontWeight(.bold)
                    Text("Browser data, cookies, and trackers")
                        .foregroundStyle(.secondary)
                }
                Spacer()
                Button {
                    viewModel.loadPrivacy()
                } label: {
                    Label("Scan", systemImage: "lock.shield")
                }
                .buttonStyle(.borderedProminent)
                .tint(.green)
                .disabled(viewModel.isLoadingPrivacy)
            }
            .padding()

            Divider()

            if viewModel.isLoadingPrivacy {
                Spacer()
                ProgressView("Running privacy audit...")
                Spacer()
            } else if let result = viewModel.privacyResult {
                ScrollView {
                    VStack(spacing: 20) {
                        // Summary
                        HStack(spacing: 32) {
                            PrivacyStatCard(
                                title: "Total Privacy Data",
                                value: result.totalPrivacyDataFormatted,
                                icon: "shield.lefthalf.filled",
                                color: .red
                            )
                            PrivacyStatCard(
                                title: "Browsers Found",
                                value: "\(result.browserProfiles.count)",
                                icon: "globe",
                                color: .blue
                            )
                            PrivacyStatCard(
                                title: "Cookie Locations",
                                value: "\(result.cookieLocationsCount)",
                                icon: "doc.text",
                                color: .orange
                            )
                            PrivacyStatCard(
                                title: "Tracking Apps",
                                value: "\(result.trackingApps.count)",
                                icon: "eye.trianglebadge.exclamationmark",
                                color: .purple
                            )
                        }
                        .padding()
                        .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 16))
                        .padding(.horizontal)

                        // Browser Profiles
                        if !result.browserProfiles.isEmpty {
                            VStack(alignment: .leading, spacing: 12) {
                                Text("Browser Profiles")
                                    .font(.headline)
                                    .padding(.horizontal)

                                ForEach(result.browserProfiles) { profile in
                                    BrowserProfileRow(profile: profile)
                                }
                            }
                            .padding()
                            .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 16))
                            .padding(.horizontal)
                        }

                        // Tracking Apps
                        if !result.trackingApps.isEmpty {
                            VStack(alignment: .leading, spacing: 12) {
                                Text("Apps with Tracking Data")
                                    .font(.headline)
                                    .padding(.horizontal)

                                ForEach(result.trackingApps) { app in
                                    HStack {
                                        Image(systemName: "eye.slash")
                                            .foregroundStyle(.purple)
                                            .frame(width: 24)
                                        VStack(alignment: .leading, spacing: 2) {
                                            Text(app.name)
                                                .fontWeight(.medium)
                                            Text(app.kind)
                                                .font(.caption)
                                                .foregroundStyle(.secondary)
                                        }
                                        Spacer()
                                        Text(app.dataSizeFormatted)
                                            .fontWeight(.semibold)
                                            .foregroundStyle(.secondary)
                                    }
                                    .padding(.horizontal)
                                    .padding(.vertical, 4)
                                }
                            }
                            .padding()
                            .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 16))
                            .padding(.horizontal)
                        }

                        Spacer(minLength: 20)
                    }
                    .padding(.top)
                }
            } else {
                Spacer()
                ContentUnavailableView(
                    "Privacy Audit",
                    systemImage: "lock.shield",
                    description: Text("Click Scan to audit browser data, cookies, and trackers")
                )
                Spacer()
            }
        }
        .onAppear {
            if viewModel.privacyResult == nil {
                viewModel.loadPrivacy()
            }
        }
    }
}

struct PrivacyStatCard: View {
    let title: String
    let value: String
    let icon: String
    let color: Color

    var body: some View {
        VStack(spacing: 8) {
            Image(systemName: icon)
                .font(.title2)
                .foregroundStyle(color)
            Text(value)
                .font(.title3)
                .fontWeight(.bold)
            Text(title)
                .font(.caption)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
        }
        .frame(maxWidth: .infinity)
    }
}

struct BrowserProfileRow: View {
    let profile: BrowserProfileInfo

    var body: some View {
        VStack(spacing: 8) {
            HStack {
                Image(systemName: browserIcon)
                    .foregroundStyle(.blue)
                    .frame(width: 24)
                Text(profile.browser)
                    .fontWeight(.medium)
                Spacer()
                Text(profile.totalSizeFormatted)
                    .fontWeight(.bold)
            }

            HStack(spacing: 16) {
                PrivacyDataPill(label: "Cookies", value: profile.cookiesSizeFormatted, color: .orange)
                PrivacyDataPill(label: "Cache", value: profile.cacheSizeFormatted, color: .blue)
                Spacer()
            }
        }
        .padding()
        .background(Color.primary.opacity(0.03), in: RoundedRectangle(cornerRadius: 10))
        .padding(.horizontal)
    }

    private var browserIcon: String {
        let name = profile.browser.lowercased()
        if name.contains("safari") { return "safari" }
        if name.contains("chrome") { return "globe" }
        if name.contains("firefox") { return "flame" }
        return "globe"
    }
}

struct PrivacyDataPill: View {
    let label: String
    let value: String
    let color: Color

    var body: some View {
        HStack(spacing: 4) {
            Circle()
                .fill(color)
                .frame(width: 6, height: 6)
            Text(label)
                .font(.caption2)
                .foregroundStyle(.secondary)
            Text(value)
                .font(.caption2)
                .fontWeight(.medium)
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 4)
        .background(color.opacity(0.08), in: Capsule())
    }
}
