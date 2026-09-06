import Foundation

/// Watches the spool directory (`VigilPaths.eventsDir`) that the installed
/// hook script writes one JSON file per event into, parses each file into a
/// `HookEvent`, deletes it, and reports it.
///
/// Contract (implemented by the hooks agent):
/// - Uses a DispatchSource vnode watcher on the directory plus a slow
///   fallback poll (every few seconds) so nothing is missed.
/// - Files are processed in name order (names are sortable timestamps).
/// - Malformed / oversized (> 256 KB) files are deleted and ignored.
/// - `parse` is pure and unit-testable.
public final class HookEventWatcher: @unchecked Sendable {
    public let spoolDir: URL

    public init(spoolDir: URL = VigilPaths.eventsDir) {
        self.spoolDir = spoolDir
    }

    public func start(onEvent: @escaping @Sendable (HookEvent) -> Void) {
        // STUB
    }

    public func stop() {
        // STUB
    }

    /// Parse the raw stdin JSON that Claude Code handed the hook.
    public static func parse(_ data: Data) throws -> HookEvent {
        // STUB
        throw NSError(domain: "Vigil.HookEventWatcher", code: 1, userInfo: [NSLocalizedDescriptionKey: "not implemented"])
    }
}
