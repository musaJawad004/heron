import Foundation

/// Discovers historical (resumable) sessions from `~/.claude/projects/**/*.jsonl`
/// and `~/.claude/history.jsonl`, and enriches them with a title (first user
/// prompt), cwd, git branch and last-activity time.
///
/// Contract (implemented by the core agent):
/// - Never read whole multi-megabyte transcripts on the main thread; read only
///   the head of each file (first ~64 KB) to find `cwd`, `gitBranch`, `version`
///   and the first `"type":"user"` message with string content.
/// - Cache results on disk under `HeronPaths.cacheDir` keyed by path + mtime +
///   size so relaunches are instant.
/// - `recentSessions(limit:)` returns sessions sorted by `lastActiveAt` desc.
public actor TranscriptIndex {
    public let projectsDir: URL
    public let historyFile: URL
    public let cacheDir: URL

    public init(
        projectsDir: URL = HeronPaths.claudeProjectsDir,
        historyFile: URL = HeronPaths.claudeHistoryFile,
        cacheDir: URL = HeronPaths.cacheDir
    ) {
        self.projectsDir = projectsDir
        self.historyFile = historyFile
        self.cacheDir = cacheDir
    }

    /// All known sessions (running or not), newest first, capped at `limit`.
    public func recentSessions(limit: Int = 50) async -> [Session] {
        // STUB — replaced by the core module implementation.
        []
    }

    /// Fill in `title`, `cwd` (if empty), `gitBranch`, `version`,
    /// `transcriptPath` for a session we only know the id of.
    public func enrich(_ session: Session) async -> Session {
        // STUB
        session
    }

    /// Forget cached data so the next call re-scans.
    public func invalidate() {
        // STUB
    }
}
