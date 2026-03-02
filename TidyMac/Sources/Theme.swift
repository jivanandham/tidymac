import SwiftUI

// MARK: - TidyMac Design System

enum TidyTheme {
    // MARK: Brand Colors
    static let emerald = Color(hue: 0.42, saturation: 0.72, brightness: 0.68)
    static let teal = Color(hue: 0.48, saturation: 0.65, brightness: 0.75)
    static let sapphire = Color(hue: 0.58, saturation: 0.70, brightness: 0.72)
    static let coral = Color(hue: 0.03, saturation: 0.65, brightness: 0.90)
    static let amber = Color(hue: 0.10, saturation: 0.75, brightness: 0.92)
    static let lavender = Color(hue: 0.73, saturation: 0.45, brightness: 0.78)
    static let rose = Color(hue: 0.95, saturation: 0.55, brightness: 0.85)

    // MARK: Gradients
    static let primaryGradient = LinearGradient(
        colors: [emerald, teal],
        startPoint: .topLeading,
        endPoint: .bottomTrailing
    )

    static let scanGradient = LinearGradient(
        colors: [sapphire, teal],
        startPoint: .topLeading,
        endPoint: .bottomTrailing
    )

    static let warningGradient = LinearGradient(
        colors: [amber, coral],
        startPoint: .topLeading,
        endPoint: .bottomTrailing
    )

    static let dangerGradient = LinearGradient(
        colors: [coral, rose],
        startPoint: .topLeading,
        endPoint: .bottomTrailing
    )

    static let purpleGradient = LinearGradient(
        colors: [lavender, Color(hue: 0.78, saturation: 0.55, brightness: 0.72)],
        startPoint: .topLeading,
        endPoint: .bottomTrailing
    )

    // MARK: Ring Gradients
    static func ringGradient(percentage: Int) -> AngularGradient {
        let colors: [Color]
        if percentage < 60 {
            colors = [emerald, teal, emerald]
        } else if percentage < 80 {
            colors = [amber, coral, amber]
        } else {
            colors = [coral, rose, coral]
        }
        return AngularGradient(colors: colors, center: .center)
    }

    // MARK: Card Styles
    static let cardBackground = Color.primary.opacity(0.04)
    static let cardBorder = Color.primary.opacity(0.08)

    // MARK: Sidebar
    static let sidebarSelectedBackground = Color.accentColor.opacity(0.15)
    static let sidebarHoverBackground = Color.primary.opacity(0.06)
}

// MARK: - Reusable View Modifiers

struct GlassCard: ViewModifier {
    var cornerRadius: CGFloat = 16
    func body(content: Content) -> some View {
        content
            .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: cornerRadius))
            .overlay(
                RoundedRectangle(cornerRadius: cornerRadius)
                    .stroke(TidyTheme.cardBorder, lineWidth: 0.5)
            )
    }
}

struct GradientCard: ViewModifier {
    let gradient: LinearGradient
    var cornerRadius: CGFloat = 16
    func body(content: Content) -> some View {
        content
            .background(
                ZStack {
                    RoundedRectangle(cornerRadius: cornerRadius)
                        .fill(.ultraThinMaterial)
                    RoundedRectangle(cornerRadius: cornerRadius)
                        .fill(gradient.opacity(0.08))
                    RoundedRectangle(cornerRadius: cornerRadius)
                        .stroke(gradient.opacity(0.2), lineWidth: 0.5)
                }
            )
    }
}

extension View {
    func glassCard(cornerRadius: CGFloat = 16) -> some View {
        modifier(GlassCard(cornerRadius: cornerRadius))
    }

    func gradientCard(_ gradient: LinearGradient, cornerRadius: CGFloat = 16) -> some View {
        modifier(GradientCard(gradient: gradient, cornerRadius: cornerRadius))
    }
}

// MARK: - Animated Counter

struct AnimatedCounter: View {
    let value: String
    let font: Font
    let color: Color

    @State private var appeared = false

    var body: some View {
        Text(value)
            .font(font)
            .fontWeight(.bold)
            .foregroundStyle(color)
            .scaleEffect(appeared ? 1.0 : 0.5)
            .opacity(appeared ? 1.0 : 0.0)
            .animation(.spring(response: 0.6, dampingFraction: 0.7), value: appeared)
            .onAppear { appeared = true }
    }
}

// MARK: - Pulsing Dot

struct PulsingDot: View {
    let color: Color
    @State private var isPulsing = false

    var body: some View {
        Circle()
            .fill(color)
            .frame(width: 8, height: 8)
            .scaleEffect(isPulsing ? 1.4 : 1.0)
            .opacity(isPulsing ? 0.5 : 1.0)
            .animation(.easeInOut(duration: 0.8).repeatForever(autoreverses: true), value: isPulsing)
            .onAppear { isPulsing = true }
    }
}

// MARK: - Shimmer Loading Effect

struct ShimmerEffect: ViewModifier {
    @State private var phase: CGFloat = 0

    func body(content: Content) -> some View {
        content
            .overlay(
                LinearGradient(
                    colors: [.clear, .white.opacity(0.15), .clear],
                    startPoint: .leading,
                    endPoint: .trailing
                )
                .offset(x: phase)
                .animation(.linear(duration: 1.5).repeatForever(autoreverses: false), value: phase)
            )
            .clipped()
            .onAppear { phase = 400 }
    }
}

extension View {
    func shimmer() -> some View {
        modifier(ShimmerEffect())
    }
}
