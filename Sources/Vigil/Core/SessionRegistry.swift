import Foundation

/// Reads the live session registry that Claude Code maintains in
/// `~/.claude/sessions/<pid>.json` and turns it into `[Session]`.
///
/// Contract (implemented by the core agent):
/// - `snapshot()` is synchronous, cheap (a handful of small files) and must
///   drop entries whose `pid` is no longer alive (`kill(pid, 0)`), so stale
///   registry files from crashed sessions never show as running.
/// - `startWatching` polls on `interval` and also reacts to directory changes
///   (DispatchSource on the directory fd) and calls `onChange` on a background
///   queue whenever the set of sessions or any status changed.
public final class SessionRegistry: @unchecked Sendable {
    public let directory: URL

    public init(directory: URL = VigilPaths.claudeSessionsDir) {
        self.directory = directory
    }

    /// Parse every `<pid>.json` in `directory` into a `Session`.
    public func snapshot() -> [Session] {
        // STUB — replaced by the core module implementation.
        []
    }

    /// Begin observing. Safe to call once; subsequent calls are no-ops.
    public func startWatching(interval: TimeInterval = 1.0, onChange: @escaping @Sendable ([Session]) -> Void) {
        // STUB
    }

    public func stop() {
        // STUB
    }
}
