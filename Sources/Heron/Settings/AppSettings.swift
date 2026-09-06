import Foundation
import Observation

/// User preferences. Backed by `UserDefaults.standard`; nothing here ever
/// leaves the machine.
@MainActor
@Observable
public final class AppSettings {
    public enum PanelEdge: String, Codable, CaseIterable, Sendable {
        case left, right
    }

    private let defaults: UserDefaults

    public init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
        terminalApp = TerminalApp(rawValue: defaults.string(forKey: Keys.terminalApp) ?? "") ?? .terminal
        claudePath = defaults.string(forKey: Keys.claudePath) ?? ""
        extraClaudeArgs = defaults.string(forKey: Keys.extraClaudeArgs) ?? ""
        showPanel = defaults.object(forKey: Keys.showPanel) as? Bool ?? true
        panelEdge = PanelEdge(rawValue: defaults.string(forKey: Keys.panelEdge) ?? "") ?? .left
        panelFollowsActiveScreen = defaults.object(forKey: Keys.panelFollowsActiveScreen) as? Bool ?? true
        panelOpacity = defaults.object(forKey: Keys.panelOpacity) as? Double ?? 1.0
        panelExpanded = defaults.object(forKey: Keys.panelExpanded) as? Bool ?? true
        panelOrigin = Self.decodePoint(defaults.string(forKey: Keys.panelOrigin))
        notifyOnPermission = defaults.object(forKey: Keys.notifyOnPermission) as? Bool ?? true
        notifyOnDone = defaults.object(forKey: Keys.notifyOnDone) as? Bool ?? true
        notifyOnIdle = defaults.object(forKey: Keys.notifyOnIdle) as? Bool ?? false
        playSound = defaults.object(forKey: Keys.playSound) as? Bool ?? true
        recentLimit = defaults.object(forKey: Keys.recentLimit) as? Int ?? 20
        showCountInMenuBar = defaults.object(forKey: Keys.showCountInMenuBar) as? Bool ?? true
        defaultProjectsFolder = defaults.string(forKey: Keys.defaultProjectsFolder) ?? ""
        launchAtLogin = defaults.object(forKey: Keys.launchAtLogin) as? Bool ?? false
    }

    // MARK: Launching
    public var terminalApp: TerminalApp { didSet { defaults.set(terminalApp.rawValue, forKey: Keys.terminalApp) } }
    /// Empty means "auto-detect".
    public var claudePath: String { didSet { defaults.set(claudePath, forKey: Keys.claudePath) } }
    /// Extra CLI flags appended to every launch, space separated.
    public var extraClaudeArgs: String { didSet { defaults.set(extraClaudeArgs, forKey: Keys.extraClaudeArgs) } }
    /// Folder the "New session" picker opens in. Empty = home.
    public var defaultProjectsFolder: String { didSet { defaults.set(defaultProjectsFolder, forKey: Keys.defaultProjectsFolder) } }

    // MARK: Floating panel
    public var showPanel: Bool { didSet { defaults.set(showPanel, forKey: Keys.showPanel) } }
    public var panelEdge: PanelEdge { didSet { defaults.set(panelEdge.rawValue, forKey: Keys.panelEdge) } }
    /// When true the panel moves to whichever screen holds the mouse/key window when shown.
    public var panelFollowsActiveScreen: Bool { didSet { defaults.set(panelFollowsActiveScreen, forKey: Keys.panelFollowsActiveScreen) } }
    public var panelOpacity: Double { didSet { defaults.set(panelOpacity, forKey: Keys.panelOpacity) } }
    /// Expanded shows a list; collapsed shows only the count pill.
    public var panelExpanded: Bool { didSet { defaults.set(panelExpanded, forKey: Keys.panelExpanded) } }
    /// Last dragged position (screen coordinates), nil until the user drags it.
    public var panelOrigin: CGPoint? { didSet { defaults.set(Self.encodePoint(panelOrigin), forKey: Keys.panelOrigin) } }

    // MARK: Notifications
    public var notifyOnPermission: Bool { didSet { defaults.set(notifyOnPermission, forKey: Keys.notifyOnPermission) } }
    public var notifyOnDone: Bool { didSet { defaults.set(notifyOnDone, forKey: Keys.notifyOnDone) } }
    public var notifyOnIdle: Bool { didSet { defaults.set(notifyOnIdle, forKey: Keys.notifyOnIdle) } }
    public var playSound: Bool { didSet { defaults.set(playSound, forKey: Keys.playSound) } }

    // MARK: General
    public var recentLimit: Int { didSet { defaults.set(recentLimit, forKey: Keys.recentLimit) } }
    public var showCountInMenuBar: Bool { didSet { defaults.set(showCountInMenuBar, forKey: Keys.showCountInMenuBar) } }
    /// Mirrors SMAppService state; AppState applies the change.
    public var launchAtLogin: Bool { didSet { defaults.set(launchAtLogin, forKey: Keys.launchAtLogin) } }

    /// Resolved claude executable: explicit setting, else auto-detected, else "claude".
    public var resolvedClaudePath: String {
        if !claudePath.isEmpty { return claudePath }
        return TerminalLauncher.findClaude() ?? "claude"
    }

    /// `extraClaudeArgs` split on whitespace.
    public var extraClaudeArgList: [String] {
        extraClaudeArgs.split(whereSeparator: { $0 == " " || $0 == "\t" }).map(String.init)
    }

    // MARK: - Keys

    private enum Keys {
        static let terminalApp = "terminalApp"
        static let claudePath = "claudePath"
        static let extraClaudeArgs = "extraClaudeArgs"
        static let defaultProjectsFolder = "defaultProjectsFolder"
        static let showPanel = "showPanel"
        static let panelEdge = "panelEdge"
        static let panelFollowsActiveScreen = "panelFollowsActiveScreen"
        static let panelOpacity = "panelOpacity"
        static let panelExpanded = "panelExpanded"
        static let panelOrigin = "panelOrigin"
        static let notifyOnPermission = "notifyOnPermission"
        static let notifyOnDone = "notifyOnDone"
        static let notifyOnIdle = "notifyOnIdle"
        static let playSound = "playSound"
        static let recentLimit = "recentLimit"
        static let showCountInMenuBar = "showCountInMenuBar"
        static let launchAtLogin = "launchAtLogin"
    }

    private static func encodePoint(_ p: CGPoint?) -> String? {
        guard let p else { return nil }
        return "\(p.x),\(p.y)"
    }

    private static func decodePoint(_ s: String?) -> CGPoint? {
        guard let s else { return nil }
        let parts = s.split(separator: ",").compactMap { Double($0) }
        guard parts.count == 2 else { return nil }
        return CGPoint(x: parts[0], y: parts[1])
    }
}
