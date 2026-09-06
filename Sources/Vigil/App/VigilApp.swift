import AppKit
import SwiftUI

@main
struct VigilApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var delegate

    var body: some Scene {
        MenuBarExtra {
            MenuBarView()
                .environment(delegate.state)
        } label: {
            MenuBarLabel(state: delegate.state)
        }
        .menuBarExtraStyle(.window)

        Settings {
            SettingsView()
                .environment(delegate.state)
        }
    }
}

/// Owns the long-lived `AppState` and the floating panel; also keeps the app
/// out of the Dock (LSUIElement is set in Info.plist, this is belt and braces).
@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    let state = AppState()
    private var panel: FloatingPanelController?

    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApp.setActivationPolicy(.accessory)
        let panel = FloatingPanelController(state: state)
        self.panel = panel
        state.panelController = panel
        state.start()
        panel.setVisible(state.settings.showPanel)
    }

    func applicationWillTerminate(_ notification: Notification) {
        state.stop()
    }
}

/// The menu bar item: a template glyph plus the running-session count.
struct MenuBarLabel: View {
    let state: AppState

    var body: some View {
        // STUB — the menubar agent replaces this with the final glyph/badge.
        HStack(spacing: 3) {
            Image(systemName: "terminal")
            if state.settings.showCountInMenuBar, state.runningCount > 0 {
                Text("\(state.runningCount)")
            }
        }
    }
}
