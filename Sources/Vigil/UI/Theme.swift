import SwiftUI

/// Shared visual language. Keep this the only place that defines colours,
/// radii and type so light/dark stay consistent across the menu bar window,
/// the floating panel and settings.
enum Theme {
    static let cornerRadius: CGFloat = 10
    static let rowRadius: CGFloat = 8
    static let padding: CGFloat = 12

    /// Colour for a session status dot.
    static func color(for status: SessionStatus, attention: Bool = false) -> Color {
        if attention { return .orange }
        switch status {
        case .busy: return .green
        case .idle: return .secondary
        case .needsInput, .waiting: return .orange
        case .unknown: return .secondary.opacity(0.5)
        }
    }

    /// Short human label for a status.
    static func label(for status: SessionStatus, attention: Bool = false) -> String {
        if attention { return "Needs you" }
        switch status {
        case .busy: return "Working"
        case .idle: return "Idle"
        case .needsInput, .waiting: return "Needs you"
        case .unknown: return "Past"
        }
    }
}
