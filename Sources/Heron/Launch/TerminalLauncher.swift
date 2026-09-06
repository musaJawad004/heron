import Foundation

/// Opens Claude Code sessions in the user's terminal of choice and brings
/// existing sessions to the front.
///
/// Contract (implemented by the launch agent):
/// - `newSession` runs `claude [extraArgs]` in `cwd` inside `app`.
/// - `resume` runs `claude --resume <id> [extraArgs]` in the session's cwd.
/// - Prefer approaches that need no Automation permission (e.g. a generated
///   `.command` file opened with the terminal app, or the terminal's own CLI
///   flags). Fall back to AppleScript only where required.
/// - `focus` tries to raise the window/tab whose tty matches `session.tty`
///   (Terminal.app and iTerm2 expose `tty` via AppleScript). Returns false if
///   it could not.
/// - All paths and arguments must be shell-quoted; never interpolate user
///   data into a shell string unquoted.
public struct TerminalLauncher: Sendable {
    public init() {}

    /// Terminals that are actually installed on this Mac, in preference order.
    public static func installedApps() -> [TerminalApp] {
        // STUB
        [.terminal]
    }

    public func newSession(in cwd: URL, app: TerminalApp, claudePath: String, extraArgs: [String]) throws {
        // STUB
    }

    public func resume(_ session: Session, app: TerminalApp, claudePath: String, extraArgs: [String]) throws {
        // STUB
    }

    @discardableResult
    public func focus(_ session: Session, app: TerminalApp) -> Bool {
        // STUB
        false
    }

    /// Locate the `claude` executable (PATH, ~/.local/bin, Homebrew, npm global).
    public static func findClaude() -> String? {
        // STUB
        nil
    }
}
