import SwiftUI

struct DockerView: View {
    @ObservedObject var viewModel: AppViewModel

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Docker")
                        .font(.largeTitle)
                        .fontWeight(.bold)
                    Text("Images, containers, volumes & build cache")
                        .foregroundStyle(.secondary)
                }
                Spacer()
                Button {
                    viewModel.loadDocker()
                } label: {
                    Label("Refresh", systemImage: "arrow.clockwise")
                }
                .disabled(viewModel.isLoadingDocker)
            }
            .padding()

            Divider()

            if viewModel.isLoadingDocker {
                Spacer()
                ProgressView("Checking Docker...")
                Spacer()
            } else if let docker = viewModel.dockerResult {
                ScrollView {
                    VStack(spacing: 20) {
                        // Status
                        HStack(spacing: 16) {
                            StatusPill(
                                label: "Installed",
                                isActive: docker.installed,
                                activeColor: .green
                            )
                            StatusPill(
                                label: "Running",
                                isActive: docker.running,
                                activeColor: .blue
                            )
                            Spacer()
                        }
                        .padding(.horizontal)

                        if !docker.installed {
                            ContentUnavailableView(
                                "Docker Not Installed",
                                systemImage: "shippingbox",
                                description: Text("Install Docker Desktop to manage containers")
                            )
                        } else if !docker.running {
                            ContentUnavailableView(
                                "Docker Not Running",
                                systemImage: "power",
                                description: Text("Start Docker Desktop to see usage details")
                            )
                        } else {
                            // Usage Summary
                            HStack(spacing: 32) {
                                DockerStatCard(
                                    title: "Total Size",
                                    value: docker.totalSizeFormatted,
                                    icon: "externaldrive.fill",
                                    color: .blue
                                )
                                DockerStatCard(
                                    title: "Reclaimable",
                                    value: docker.reclaimableFormatted,
                                    icon: "arrow.3.trianglepath",
                                    color: .orange
                                )
                            }
                            .padding()
                            .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 16))
                            .padding(.horizontal)

                            // Tip
                            HStack(spacing: 12) {
                                Image(systemName: "lightbulb.fill")
                                    .foregroundStyle(.yellow)
                                VStack(alignment: .leading, spacing: 2) {
                                    Text("Cleanup Tip")
                                        .font(.subheadline)
                                        .fontWeight(.semibold)
                                    Text("Use `docker system prune` for granular control over what gets removed. TidyMac reports Docker data size but recommends using Docker's own tools for cleanup.")
                                        .font(.caption)
                                        .foregroundStyle(.secondary)
                                }
                            }
                            .padding()
                            .background(.yellow.opacity(0.08), in: RoundedRectangle(cornerRadius: 12))
                            .padding(.horizontal)
                        }

                        Spacer(minLength: 20)
                    }
                    .padding(.top)
                }
            } else {
                Spacer()
                ContentUnavailableView(
                    "Docker Status",
                    systemImage: "shippingbox",
                    description: Text("Click Refresh to check Docker disk usage")
                )
                Spacer()
            }
        }
        .onAppear {
            if viewModel.dockerResult == nil {
                viewModel.loadDocker()
            }
        }
    }
}

struct StatusPill: View {
    let label: String
    let isActive: Bool
    let activeColor: Color

    var body: some View {
        HStack(spacing: 6) {
            Circle()
                .fill(isActive ? activeColor : .gray)
                .frame(width: 8, height: 8)
            Text(label)
                .font(.subheadline)
                .fontWeight(.medium)
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 6)
        .background(
            (isActive ? activeColor : .gray).opacity(0.1),
            in: Capsule()
        )
    }
}

struct DockerStatCard: View {
    let title: String
    let value: String
    let icon: String
    let color: Color

    var body: some View {
        VStack(spacing: 8) {
            Image(systemName: icon)
                .font(.title)
                .foregroundStyle(color)
            Text(value)
                .font(.title2)
                .fontWeight(.bold)
            Text(title)
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity)
    }
}
