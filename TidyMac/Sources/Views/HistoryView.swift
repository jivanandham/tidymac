import SwiftUI

struct HistoryView: View {
    @ObservedObject var viewModel: AppViewModel
    @State private var showUndoConfirm = false
    @State private var selectedSession: UndoSession?

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                VStack(alignment: .leading, spacing: 4) {
                    Text("History")
                        .font(.largeTitle)
                        .fontWeight(.bold)
                    Text("Past cleanup sessions with undo support")
                        .foregroundStyle(.secondary)
                }
                Spacer()
                Button {
                    viewModel.loadHistory()
                } label: {
                    Label("Refresh", systemImage: "arrow.clockwise")
                }
                .disabled(viewModel.isLoadingHistory)
            }
            .padding()

            Divider()

            if viewModel.isLoadingHistory {
                Spacer()
                ProgressView("Loading history...")
                Spacer()
            } else if viewModel.undoSessions.isEmpty {
                Spacer()
                ContentUnavailableView(
                    "No History",
                    systemImage: "clock.arrow.circlepath",
                    description: Text("Cleanup sessions will appear here after you run a clean operation")
                )
                Spacer()
            } else {
                List {
                    ForEach(viewModel.undoSessions) { session in
                        SessionRow(session: session) {
                            selectedSession = session
                            showUndoConfirm = true
                        }
                    }
                }
                .listStyle(.inset(alternatesRowBackgrounds: true))
            }
        }
        .onAppear {
            viewModel.loadHistory()
        }
        .alert("Restore Session?", isPresented: $showUndoConfirm) {
            Button("Cancel", role: .cancel) {}
            Button("Restore") {
                if let session = selectedSession {
                    viewModel.undoSession(id: session.sessionId)
                }
            }
        } message: {
            if let session = selectedSession {
                Text("Restore \(session.totalFiles) files (\(session.totalBytesFormatted)) from session \(session.sessionId)?")
            }
        }
    }
}

struct SessionRow: View {
    let session: UndoSession
    let onUndo: () -> Void

    var body: some View {
        HStack(spacing: 12) {
            Image(systemName: statusIcon)
                .foregroundStyle(statusColor)
                .font(.title3)
                .frame(width: 28)

            VStack(alignment: .leading, spacing: 4) {
                HStack(spacing: 8) {
                    Text(session.sessionId)
                        .fontWeight(.medium)
                    Text(session.profile)
                        .font(.caption)
                        .padding(.horizontal, 6)
                        .padding(.vertical, 2)
                        .background(.blue.opacity(0.1), in: Capsule())
                        .foregroundStyle(.blue)
                    Text(session.mode)
                        .font(.caption)
                        .padding(.horizontal, 6)
                        .padding(.vertical, 2)
                        .background(.purple.opacity(0.1), in: Capsule())
                        .foregroundStyle(.purple)
                }

                HStack(spacing: 16) {
                    Label("\(session.totalFiles) files", systemImage: "doc.on.doc")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    Label(session.totalBytesFormatted, systemImage: "externaldrive")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    Text(session.timestamp)
                        .font(.caption)
                        .foregroundStyle(.tertiary)
                }
            }

            Spacer()

            if session.restored {
                Text("Restored")
                    .font(.caption)
                    .fontWeight(.medium)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 4)
                    .background(.green.opacity(0.15), in: Capsule())
                    .foregroundStyle(.green)
            } else if session.isExpired {
                Text("Expired")
                    .font(.caption)
                    .fontWeight(.medium)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 4)
                    .background(.gray.opacity(0.15), in: Capsule())
                    .foregroundStyle(.gray)
            } else if session.mode == "soft_delete" {
                Button("Undo") {
                    onUndo()
                }
                .buttonStyle(.bordered)
                .controlSize(.small)
            }
        }
        .padding(.vertical, 6)
    }

    private var statusIcon: String {
        if session.restored { return "arrow.uturn.backward.circle.fill" }
        if session.isExpired { return "clock.badge.xmark" }
        if session.mode == "hard_delete" { return "trash.fill" }
        return "archivebox.fill"
    }

    private var statusColor: Color {
        if session.restored { return .green }
        if session.isExpired { return .gray }
        if session.mode == "hard_delete" { return .red }
        return .orange
    }
}
