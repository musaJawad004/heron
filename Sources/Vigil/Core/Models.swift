import Foundation

// MARK: - Shared domain types
//
// These types are the contract between every module in Vigil. Keep them
// small, value-typed and free of AppKit/SwiftUI imports so the core can be
// unit-tested without a UI.

/// Live status of a running Claude Code session, as reported by the CLI's own
/// registry file in `~/.claude/sessions/<pid>.json` (`status` field).
public enum SessionStatus: String, Codable, Hashable, Sendable, CaseIterable {
    /// Claude is generating / running tools.
    case busy
    /// Waiting for the user to type the next prompt.
    case idle
    /// Claude is blocked on the user (permission prompt, question, plan approval).
    case needsInput = "needs_input"
    /// Reserved by the CLI; treated like `needsInput` for display purposes.
    case waiting
    /// Registry did not report a status we understand.
    case unknown

    public init(rawStatus: String?) {
        guard let raw = rawStatus, let value = SessionStatus(rawValue: raw) else {
            self = .unknown
            return
        }
        self = value
    }

    /// True when the session is blocked on the human.
    public var needsAttention: Bool { self == .needsInput || self == .waiting }
}

/// One Claude Code session — either currently running (has a `pid`) or a
/// historical transcript that can be resumed.
public struct Session: Identifiable, Hashable, Sendable {
    /// Claude Code session id (UUID string). Also the transcript file name.
    public let id: String
    /// Display name from the live registry (`name`), e.g. "civl-mobile-app-1b".
    public var name: String?
    /// Human-friendly summary (first user prompt), populated from the transcript.
    public var title: String?
    /// Working directory the session was started in.
    public var cwd: String
    /// Process id when the session is running, otherwise nil.
    public var pid: Int32?
    /// Live status; `.unknown` for historical sessions.
    public var status: SessionStatus
    /// When the running process started (registry `startedAt`).
    public var startedAt: Date?
    /// Most recent activity we know of (registry `updatedAt` or transcript mtime).
    public var lastActiveAt: Date
    /// Claude Code version that wrote the session.
    public var version: String?
    /// Git branch recorded in the transcript, if any.
    public var gitBranch: String?
    /// Absolute path of the `.jsonl` transcript, if located.
    public var transcriptPath: String?
    /// Controlling terminal of the running process (e.g. "ttys003"), if known.
    public var tty: String?
    /// Hook event that most recently asked for the user's attention, if any.
    public var attention: HookEvent?

    public init(
        id: String,
        name: String? = nil,
        title: String? = nil,
        cwd: String,
        pid: Int32? = nil,
        status: SessionStatus = .unknown,
        startedAt: Date? = nil,
        lastActiveAt: Date,
        version: String? = nil,
        gitBranch: String? = nil,
        transcriptPath: String? = nil,
        tty: String? = nil,
        attention: HookEvent? = nil
    ) {
        self.id = id
        self.name = name
        self.title = title
        self.cwd = cwd
        self.pid = pid
        self.status = status
        self.startedAt = startedAt
        self.lastActiveAt = lastActiveAt
        self.version = version
        self.gitBranch = gitBranch
        self.transcriptPath = transcriptPath
        self.tty = tty
        self.attention = attention
    }

    public var isRunning: Bool { pid != nil }

    /// Last path component of `cwd`, used as the compact project label.
    public var projectName: String {
        let url = URL(fileURLWithPath: cwd)
        let last = url.lastPathComponent
        return last.isEmpty ? cwd : last
    }

    /// Best available short label for lists: name, then title, then project.
    public var displayName: String {
        if let name, !name.isEmpty { return name }
        if let title, !title.isEmpty { return title }
        return projectName
    }
}

// MARK: - Hook events

/// Which Claude Code hook produced an event. Raw values match
/// `hook_event_name` in the hook payload.
public enum HookEventKind: String, Codable, Hashable, Sendable {
    case sessionStart = "SessionStart"
    case sessionEnd = "SessionEnd"
    case stop = "Stop"
    case subagentStop = "SubagentStop"
    case notification = "Notification"
    case permissionRequest = "PermissionRequest"
    case userPromptSubmit = "UserPromptSubmit"
    case preToolUse = "PreToolUse"
    case postToolUse = "PostToolUse"
    case other = "Other"
}

/// A single event received from a Claude Code hook via the local spool
/// directory. Everything here originated on this machine; nothing leaves it.
public struct HookEvent: Identifiable, Hashable, Codable, Sendable {
    public let id: UUID
    public let kind: HookEventKind
    /// `session_id` from the payload.
    public let sessionId: String
    /// `cwd` from the payload.
    public let cwd: String?
    /// `transcript_path` from the payload.
    public let transcriptPath: String?
    /// `notification_type` for `.notification` events (e.g. "permission_prompt",
    /// "idle_prompt"); `reason` for `.sessionEnd`; `source` for `.sessionStart`.
    public let subtype: String?
    /// Human readable message (`message` field), when present.
    public let message: String?
    /// Tool name for tool / permission events, when present.
    public let toolName: String?
    /// When Vigil read the event from the spool.
    public let receivedAt: Date

    public init(
        id: UUID = UUID(),
        kind: HookEventKind,
        sessionId: String,
        cwd: String? = nil,
        transcriptPath: String? = nil,
        subtype: String? = nil,
        message: String? = nil,
        toolName: String? = nil,
        receivedAt: Date = Date()
    ) {
        self.id = id
        self.kind = kind
        self.sessionId = sessionId
        self.cwd = cwd
        self.transcriptPath = transcriptPath
        self.subtype = subtype
        self.message = message
        self.toolName = toolName
        self.receivedAt = receivedAt
    }

    /// True for events that mean "a human needs to look at this session".
    public var requiresAttention: Bool {
        switch kind {
        case .permissionRequest: return true
        case .notification:
            return subtype == "permission_prompt" || subtype == "idle_prompt"
                || subtype == "elicitation_dialog"
        default: return false
        }
    }

    /// True for events that mean "Claude finished its turn".
    public var isCompletion: Bool { kind == .stop }
}

// MARK: - Terminal apps

/// Terminal emulators Vigil knows how to open a new Claude session in.
public enum TerminalApp: String, Codable, CaseIterable, Identifiable, Sendable {
    case terminal = "com.apple.Terminal"
    case iterm = "com.googlecode.iterm2"
    case ghostty = "com.mitchellh.ghostty"
    case warp = "dev.warp.Warp-Stable"
    case kitty = "net.kovidgoyal.kitty"
    case alacritty = "org.alacritty"
    case wezterm = "com.github.wez.wezterm"

    public var id: String { rawValue }

    public var displayName: String {
        switch self {
        case .terminal: return "Terminal"
        case .iterm: return "iTerm2"
        case .ghostty: return "Ghostty"
        case .warp: return "Warp"
        case .kitty: return "kitty"
        case .alacritty: return "Alacritty"
        case .wezterm: return "WezTerm"
        }
    }

    /// Bundle identifier used with `NSWorkspace` / `open -b`.
    public var bundleIdentifier: String { rawValue }
}

// MARK: - Well-known paths

/// Locations Vigil reads from or writes to. Everything is under the user's
/// home directory; Vigil never talks to the network.
public enum VigilPaths {
    public static var home: URL { FileManager.default.homeDirectoryForCurrentUser }

    /// `~/.claude`
    public static var claudeDir: URL { home.appendingPathComponent(".claude", isDirectory: true) }
    /// `~/.claude/sessions` — live registry written by the CLI (one JSON per pid).
    public static var claudeSessionsDir: URL { claudeDir.appendingPathComponent("sessions", isDirectory: true) }
    /// `~/.claude/projects` — transcripts grouped by encoded cwd.
    public static var claudeProjectsDir: URL { claudeDir.appendingPathComponent("projects", isDirectory: true) }
    /// `~/.claude/history.jsonl` — prompt history index (display, project, sessionId, timestamp).
    public static var claudeHistoryFile: URL { claudeDir.appendingPathComponent("history.jsonl") }
    /// `~/.claude/settings.json` — user settings where hooks are registered.
    public static var claudeSettingsFile: URL { claudeDir.appendingPathComponent("settings.json") }

    /// `~/Library/Application Support/Vigil`
    public static var appSupportDir: URL {
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? home.appendingPathComponent("Library/Application Support", isDirectory: true)
        return base.appendingPathComponent("Vigil", isDirectory: true)
    }
    /// Spool directory hook scripts drop event JSON files into.
    public static var eventsDir: URL { appSupportDir.appendingPathComponent("events", isDirectory: true) }
    /// Where the hook receiver script is installed.
    public static var hookScript: URL { appSupportDir.appendingPathComponent("bin/vigil-hook", isDirectory: false) }
    /// On-disk cache for the transcript index.
    public static var cacheDir: URL { appSupportDir.appendingPathComponent("cache", isDirectory: true) }
}
