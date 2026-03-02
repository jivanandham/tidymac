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
                    Text("Timeline of past cleanup sessions")
                        .foregroundStyle(.secondary)
                }
                Spacer()
                Button {
                    viewModel.loadHistory()
                } label: {
                    Label("Refresh", systemImage: "arrow.clockwise")
                }
                .disabled(viewModel.isLoadingHistory)
                .accessibilityLabel("Refresh History")
            }
            .padding()

            Divider()

            if viewModel.isLoadingHistory {
                Spacer()
                ProgressView("Loading history timeline...")
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
                ScrollView {
                    LazyVStack(spacing: 0) {
                        ForEach(Array(viewModel.undoSessions.enumerated()), id: \.element.id) { index, session in
                            TimelineRow(
                                session: session,
                                isFirst: index == 0,
                                isLast: index == viewModel.undoSessions.count - 1
                            ) {
                                selectedSession = session
                                showUndoConfirm = true
                            }
                        }
                    }
                    .padding()
                }
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
        .alert("Undo Complete", isPresented: $viewModel.showUndoResult) {
            Button("OK", role: .cancel) {}
        } message: {
            if let result = viewModel.undoResult {
                if result.errors.isEmpty {
                    Text("Successfully restored \(result.restoredCount) files (\(result.restoredBytesFormatted)).")
                } else {
                    Text("Restored \(result.restoredCount) files.\nErrors: \(result.errors.joined(separator: ", "))")
                }
            }
        }
    }
}

// MARK: - Timeline Subviews

struct TimelineRow: View {
    let session: UndoSession
    let isFirst: Bool
    let isLast: Bool
    let onUndo: () -> Void

    @State private var isHovered = false

    var body: some View {
        HStack(alignment: .top, spacing: 20) {
            // Left Column: Relative Time
            VStack(alignment: .trailing, spacing: 4) {
                Text(relativeTime(from: session.timestamp))
                    .font(.subheadline)
                    .fontWeight(.medium)
                
                Text(formattedDate(from: session.timestamp))
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }
            .frame(width: 80, alignment: .trailing)
            .padding(.top, 16)

            // Middle Column: Timeline Line
            VStack(spacing: 0) {
                Rectangle()
                    .fill(isFirst ? .clear : Color.gray.opacity(0.2))
                    .frame(width: 2, height: 16)
                
                Circle()
                    .fill(statusColor)
                    .frame(width: 14, height: 14)
                    .overlay(
                        Circle()
                            .stroke(Color(.windowBackgroundColor), lineWidth: 3)
                    )
                    .shadow(color: statusColor.opacity(0.3), radius: 4, x: 0, y: 2)
                    .scaleEffect(isHovered ? 1.2 : 1.0)
                    .animation(.spring(response: 0.3, dampingFraction: 0.6), value: isHovered)
                
                Rectangle()
                    .fill(isLast ? .clear : Color.gray.opacity(0.2))
                    .frame(width: 2)
            }

            // Right Column: Session Card
            VStack(alignment: .leading, spacing: 12) {
                HStack {
                    Image(systemName: statusIcon)
                        .foregroundStyle(statusColor)
                        .font(.title3)
                    
                    Text("Cleaned \(session.totalBytesFormatted)")
                        .font(.headline)
                    
                    Spacer()
                    
                    if session.restored {
                        Badge(text: "Restored", color: .green)
                    } else if session.isExpired {
                        Badge(text: "Expired", color: .gray)
                    } else if session.mode == "soft_delete" {
                        Button("Undo Session") {
                            onUndo()
                        }
                        .buttonStyle(.bordered)
                        .controlSize(.small)
                        .tint(TidyTheme.sapphire)
                        .accessibilityLabel("Restore Session \(session.sessionId)")
                    }
                }

                HStack(spacing: 16) {
                    Label("\(session.totalFiles) files", systemImage: "doc.on.doc")
                        .font(.caption)
                    
                    Label(session.profile.capitalized, systemImage: "slider.horizontal.3")
                        .font(.caption)
                    
                    Label(session.mode == "hard_delete" ? "Permanent" : "Soft Delete", systemImage: "trash")
                        .font(.caption)
                }
                .foregroundStyle(.secondary)
                
                Text("Session ID: \(session.sessionId)")
                    .font(.caption2)
                    .foregroundStyle(.tertiary)
                    .monospaced()
            }
            .padding(16)
            .background(
                RoundedRectangle(cornerRadius: 12)
                    .fill(isHovered ? TidyTheme.cardBackground.opacity(2) : TidyTheme.cardBackground)
            )
            .overlay(
                RoundedRectangle(cornerRadius: 12)
                    .stroke(TidyTheme.cardBorder, lineWidth: 0.5)
            )
            .padding(.vertical, 8)
            .onHover { hovering in
                withAnimation { isHovered = hovering }
            }
        }
    }

    private var statusIcon: String {
        if session.restored { return "arrow.uturn.backward.circle.fill" }
        if session.isExpired { return "clock.badge.xmark" }
        if session.mode == "hard_delete" { return "trash.fill" }
        return "archivebox.fill"
    }

    private var statusColor: Color {
        if session.restored { return TidyTheme.emerald }
        if session.isExpired { return .gray }
        if session.mode == "hard_delete" { return TidyTheme.coral }
        return TidyTheme.amber
    }

    // Helper to format ISO8601 string to relative time
    private func relativeTime(from isoString: String) -> String {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        guard let date = formatter.date(from: isoString) ?? ISO8601DateFormatter().date(from: isoString) else {
            return "Unknown"
        }
        
        let relativeFormatter = RelativeDateTimeFormatter()
        relativeFormatter.unitsStyle = .full
        return relativeFormatter.localizedString(for: date, relativeTo: Date())
    }
    
    // Helper to format ISO8601 string to short date
    private func formattedDate(from isoString: String) -> String {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        guard let date = formatter.date(from: isoString) ?? ISO8601DateFormatter().date(from: isoString) else {
            return ""
        }
        
        let dateFormatter = DateFormatter()
        dateFormatter.dateStyle = .medium
        dateFormatter.timeStyle = .none
        return dateFormatter.string(from: date)
    }
}

struct Badge: View {
    let text: String
    let color: Color
    
    var body: some View {
        Text(text)
            .font(.caption2)
            .fontWeight(.medium)
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
            .background(color.opacity(0.12), in: Capsule())
            .foregroundStyle(color)
    }
}
