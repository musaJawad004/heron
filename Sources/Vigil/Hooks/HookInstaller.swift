import Foundation

/// Installs / removes the Vigil hook receiver in `~/.claude/settings.json`.
///
/// Contract (implemented by the hooks agent):
/// - Writes the receiver script to `VigilPaths.hookScript` (mode 0700). The
///   script reads stdin, writes it atomically into `VigilPaths.eventsDir`
///   (mode 0600 files, 0700 dir) and always exits 0 with no stdout, so it can
///   never block or alter Claude Code.
/// - Registers that script for `SessionStart`, `SessionEnd`, `Stop`,
///   `Notification`, `PermissionRequest` (and `UserPromptSubmit`) events.
/// - Merges into existing settings without disturbing other hooks; entries are
///   tagged (command path contains `vigil-hook`) so `uninstall` removes only
///   what Vigil added. Takes a timestamped backup of settings.json first.
public struct HookInstaller: Sendable {
    public enum Status: String, Sendable {
        case installed
        case notInstalled
        /// Some but not all events are registered, or the script is stale.
        case partial
    }

    public let settingsFile: URL
    public let scriptPath: URL
    public let eventsDir: URL

    public init(
        settingsFile: URL = VigilPaths.claudeSettingsFile,
        scriptPath: URL = VigilPaths.hookScript,
        eventsDir: URL = VigilPaths.eventsDir
    ) {
        self.settingsFile = settingsFile
        self.scriptPath = scriptPath
        self.eventsDir = eventsDir
    }

    /// Events Vigil registers a hook for.
    public static let events: [HookEventKind] = [
        .sessionStart, .sessionEnd, .stop, .notification, .permissionRequest, .userPromptSubmit,
    ]

    public func status() -> Status {
        // STUB
        .notInstalled
    }

    public func install() throws {
        // STUB
    }

    public func uninstall() throws {
        // STUB
    }
}
