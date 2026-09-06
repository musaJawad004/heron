import AppKit
import Foundation
import Observation
import ServiceManagement

/// Single source of truth for the UI. Owns the core services and merges their
/// output into `sessions` (running first, then recent history).
@MainActor
@Observable
public final class AppState {
    public let settings: AppSettings
    public let registry: SessionRegistry
    public let index: TranscriptIndex
    public let watcher: HookEventWatcher
    public let notifier: Notifier
    public let launcher: TerminalLauncher
    public let hookInstaller: HookInstaller

    /// Sessions currently running (from the live registry), newest first.
    public private(set) var running: [Session] = []
    /// Historical sessions that are not running, newest first.
    public private(set) var recent: [Session] = []
    /// Session ids currently waiting on the human (permission / question).
    public private(set) var attention: [String: HookEvent] = [:]
    /// Most recent hook events, newest first (capped).
    public private(set) var eventLog: [HookEvent] = []
    public private(set) var hookStatus: HookInstaller.Status = .notInstalled
    public private(set) var lastError: String?
    public private(set) var isRefreshingHistory = false

    /// Set by the app entry point so state can toggle the floating panel.
    public var panelController: FloatingPanelController?

    private var historyTask: Task<Void, Never>?
    private var started = false

    public init(
        settings: AppSettings = AppSettings(),
        registry: SessionRegistry = SessionRegistry(),
        index: TranscriptIndex = TranscriptIndex(),
        watcher: HookEventWatcher = HookEventWatcher(),
        notifier: Notifier = Notifier(),
        launcher: TerminalLauncher = TerminalLauncher(),
        hookInstaller: HookInstaller = HookInstaller()
    ) {
        self.settings = settings
        self.registry = registry
        self.index = index
        self.watcher = watcher
        self.notifier = notifier
        self.launcher = launcher
        self.hookInstaller = hookInstaller
    }

    // MARK: Derived

    public var runningCount: Int { running.count }
    public var attentionCount: Int { running.filter { $0.status.needsAttention || attention[$0.id] != nil }.count }
    public var busyCount: Int { running.filter { $0.status == .busy }.count }

    /// Running + recent, de-duplicated by id.
    public var allSessions: [Session] {
        var seen = Set(running.map(\.id))
        var out = running
        for s in recent where !seen.contains(s.id) {
            seen.insert(s.id)
            out.append(s)
        }
        return out
    }

    // MARK: Lifecycle

    public func start() {
        guard !started else { return }
        started = true
        hookStatus = hookInstaller.status()
        notifier.playSound = settings.playSound
        notifier.onActivate = { [weak self] sessionId in
            Task { @MainActor in self?.focus(sessionId: sessionId) }
        }

        registry.startWatching(interval: 1.0) { [weak self] sessions in
            Task { @MainActor in self?.applyRegistry(sessions) }
        }
        watcher.start { [weak self] event in
            Task { @MainActor in self?.handle(event) }
        }
        applyRegistry(registry.snapshot())
        refreshHistory()
    }

    public func stop() {
        registry.stop()
        watcher.stop()
    }

    public func refreshHistory() {
        historyTask?.cancel()
        isRefreshingHistory = true
        let limit = settings.recentLimit
        historyTask = Task { [index] in
            let sessions = await index.recentSessions(limit: limit)
            guard !Task.isCancelled else { return }
            self.recent = sessions
            self.isRefreshingHistory = false
        }
    }

    // MARK: Registry / events

    private func applyRegistry(_ sessions: [Session]) {
        var merged = sessions.sorted { ($0.startedAt ?? .distantPast) > ($1.startedAt ?? .distantPast) }
        for i in merged.indices {
            let id = merged[i].id
            if let a = attention[id] { merged[i].attention = a }
            // Keep titles we already learned so rows don't flicker.
            if merged[i].title == nil, let old = running.first(where: { $0.id == id }) ?? recent.first(where: { $0.id == id }) {
                merged[i].title = old.title
                if merged[i].gitBranch == nil { merged[i].gitBranch = old.gitBranch }
                if merged[i].transcriptPath == nil { merged[i].transcriptPath = old.transcriptPath }
            }
            // A session that is idle again is no longer waiting on us.
            if merged[i].status == .idle || merged[i].status == .busy, attention[id] != nil,
               let a = attention[id], a.receivedAt < (merged[i].lastActiveAt) {
                attention[id] = nil
                merged[i].attention = nil
            }
        }
        let changed = merged != running
        running = merged
        if changed {
            enrichRunningTitles()
            // A session that just appeared/disappeared changes history too.
            refreshHistoryDebounced()
        }
    }

    private func enrichRunningTitles() {
        let missing = running.filter { $0.title == nil }
        guard !missing.isEmpty else { return }
        Task { [index] in
            for s in missing {
                let enriched = await index.enrich(s)
                if let i = self.running.firstIndex(where: { $0.id == s.id }) {
                    var updated = self.running[i]
                    updated.title = enriched.title
                    updated.gitBranch = updated.gitBranch ?? enriched.gitBranch
                    updated.transcriptPath = updated.transcriptPath ?? enriched.transcriptPath
                    self.running[i] = updated
                }
            }
        }
    }

    private var historyDebounce: Task<Void, Never>?
    private func refreshHistoryDebounced() {
        historyDebounce?.cancel()
        historyDebounce = Task {
            try? await Task.sleep(for: .seconds(2))
            guard !Task.isCancelled else { return }
            refreshHistory()
        }
    }

    private func handle(_ event: HookEvent) {
        eventLog.insert(event, at: 0)
        if eventLog.count > 200 { eventLog.removeLast(eventLog.count - 200) }

        let session = allSessions.first { $0.id == event.sessionId }

        if event.requiresAttention {
            attention[event.sessionId] = event
            if let i = running.firstIndex(where: { $0.id == event.sessionId }) { running[i].attention = event }
            if settings.notifyOnPermission || (event.subtype == "idle_prompt" && settings.notifyOnIdle) {
                notifier.post(event: event, session: session)
            }
        } else if event.isCompletion {
            attention[event.sessionId] = nil
            if let i = running.firstIndex(where: { $0.id == event.sessionId }) { running[i].attention = nil }
            if settings.notifyOnDone { notifier.post(event: event, session: session) }
        } else if event.kind == .userPromptSubmit || event.kind == .sessionEnd {
            attention[event.sessionId] = nil
            if let i = running.firstIndex(where: { $0.id == event.sessionId }) { running[i].attention = nil }
            if event.kind == .sessionEnd { refreshHistoryDebounced() }
        } else if event.kind == .sessionStart {
            refreshHistoryDebounced()
        }
    }

    // MARK: Actions

    public func newSession(in folder: URL) {
        do {
            try launcher.newSession(
                in: folder,
                app: settings.terminalApp,
                claudePath: settings.resolvedClaudePath,
                extraArgs: settings.extraClaudeArgList
            )
            lastError = nil
        } catch {
            lastError = error.localizedDescription
        }
    }

    /// Present a folder picker then start a session there.
    public func newSessionWithPicker() {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.allowsMultipleSelection = false
        panel.canCreateDirectories = true
        panel.prompt = "Start Claude here"
        panel.message = "Choose the project folder for the new Claude Code session"
        let base = settings.defaultProjectsFolder
        panel.directoryURL = base.isEmpty ? VigilPaths.home : URL(fileURLWithPath: base)
        NSApp.activate(ignoringOtherApps: true)
        if panel.runModal() == .OK, let url = panel.url {
            newSession(in: url)
        }
    }

    public func resume(_ session: Session) {
        if session.isRunning {
            focus(session)
            return
        }
        do {
            try launcher.resume(
                session,
                app: settings.terminalApp,
                claudePath: settings.resolvedClaudePath,
                extraArgs: settings.extraClaudeArgList
            )
            lastError = nil
        } catch {
            lastError = error.localizedDescription
        }
    }

    public func focus(_ session: Session) {
        if !launcher.focus(session, app: settings.terminalApp) {
            // Best effort: at least bring the terminal app forward.
            if let url = NSWorkspace.shared.urlForApplication(withBundleIdentifier: settings.terminalApp.bundleIdentifier) {
                NSWorkspace.shared.openApplication(at: url, configuration: NSWorkspace.OpenConfiguration())
            }
        }
    }

    public func focus(sessionId: String) {
        if let s = allSessions.first(where: { $0.id == sessionId }) { focus(s) }
    }

    /// Send SIGTERM to a running session (after the UI confirmed).
    public func terminate(_ session: Session) {
        guard let pid = session.pid else { return }
        kill(pid, SIGTERM)
    }

    public func revealInFinder(_ session: Session) {
        NSWorkspace.shared.activateFileViewerSelecting([URL(fileURLWithPath: session.cwd)])
    }

    public func togglePanel() {
        settings.showPanel.toggle()
        panelController?.setVisible(settings.showPanel)
    }

    public func installHooks() {
        do {
            try hookInstaller.install()
            hookStatus = hookInstaller.status()
            lastError = nil
            Task { await notifier.requestAuthorization() }
        } catch {
            lastError = error.localizedDescription
        }
    }

    public func uninstallHooks() {
        do {
            try hookInstaller.uninstall()
            hookStatus = hookInstaller.status()
            lastError = nil
        } catch {
            lastError = error.localizedDescription
        }
    }

    public func setLaunchAtLogin(_ enabled: Bool) {
        do {
            if enabled { try SMAppService.mainApp.register() } else { try SMAppService.mainApp.unregister() }
            settings.launchAtLogin = enabled
            lastError = nil
        } catch {
            lastError = error.localizedDescription
        }
    }

    public func clearError() { lastError = nil }
}
