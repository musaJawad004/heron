import Foundation

/// Posts local user notifications (UNUserNotificationCenter) for hook events.
/// Never shows prompt contents beyond a short message; never leaves the Mac.
///
/// Contract (implemented by the hooks agent):
/// - `requestAuthorization()` asks once; result is cached.
/// - `post(event:session:)` builds a concise title/body such as
///   "civl-mobile-app needs permission" / "Bash: git push" and, when
///   `playSound` is true, attaches the default sound. Clicking the
///   notification should call `onActivate(sessionId)` so the app can focus
///   the session's terminal.
@MainActor
public final class Notifier {
    public var playSound = true
    public var onActivate: ((String) -> Void)?

    public init() {}

    @discardableResult
    public func requestAuthorization() async -> Bool {
        // STUB
        false
    }

    public func post(event: HookEvent, session: Session?) {
        // STUB
    }
}
